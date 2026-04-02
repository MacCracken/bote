//! Transport middleware helpers — shared between HTTP and streamable transports.
//!
//! These functions extract and validate MCP headers, returning axum `Response`
//! errors on failure. Each returns `Result<T, Response>` where the `Err` variant
//! is the HTTP error response to return to the client.

use std::sync::Arc;

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::session::{
    MCP_PROTOCOL_VERSION_HEADER, MCP_SESSION_ID_HEADER, SessionStore, validate_origin,
    validate_protocol_version,
};

/// Check `Origin` header for DNS rebinding protection. Returns 403 on failure.
#[allow(clippy::result_large_err)]
pub(crate) fn check_origin(headers: &HeaderMap, allowed: &[String]) -> Result<(), Response> {
    let origin = headers
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    validate_origin(origin, allowed).map_err(|reason| {
        tracing::warn!(origin, %reason, "origin rejected");
        (StatusCode::FORBIDDEN, reason).into_response()
    })
}

/// Check `MCP-Protocol-Version` header. Returns 400 on failure.
/// Missing header is allowed (for plain HTTP transport).
#[allow(clippy::result_large_err)]
pub(crate) fn check_protocol_version(headers: &HeaderMap) -> Result<(), Response> {
    if let Some(version) = headers
        .get(MCP_PROTOCOL_VERSION_HEADER)
        .and_then(|v| v.to_str().ok())
    {
        validate_protocol_version(version).map_err(|reason| {
            tracing::warn!(%reason, "protocol version rejected");
            (StatusCode::BAD_REQUEST, reason).into_response()
        })?;
    }
    Ok(())
}

/// Check `MCP-Protocol-Version` header — **required** (streamable transport).
/// Returns 400 if missing or invalid.
#[allow(clippy::result_large_err)]
pub(crate) fn check_protocol_version_required(headers: &HeaderMap) -> Result<(), Response> {
    let version = headers
        .get(MCP_PROTOCOL_VERSION_HEADER)
        .and_then(|v| v.to_str().ok());
    match version {
        Some(v) => validate_protocol_version(v).map(|_| ()).map_err(|reason| {
            tracing::warn!(%reason, "protocol version rejected");
            (StatusCode::BAD_REQUEST, reason).into_response()
        }),
        None => Err((
            StatusCode::BAD_REQUEST,
            "missing MCP-Protocol-Version header",
        )
            .into_response()),
    }
}

/// Check `MCP-Session-Id` header. Returns 404 if session is invalid.
/// No-op if sessions are disabled or this is an initialize request.
#[allow(clippy::result_large_err)]
pub(crate) fn check_session(
    headers: &HeaderMap,
    store: &Option<Arc<SessionStore>>,
    is_initialize: bool,
) -> Result<(), Response> {
    let Some(store) = store else { return Ok(()) };
    if is_initialize {
        return Ok(());
    }
    let session_id = headers
        .get(MCP_SESSION_ID_HEADER)
        .and_then(|v| v.to_str().ok());
    match session_id {
        Some(id) => {
            if store.validate(id).is_none() {
                tracing::warn!(session_id = %id, "invalid session ID");
                return Err((StatusCode::NOT_FOUND, "invalid or expired session").into_response());
            }
            Ok(())
        }
        None => {
            tracing::warn!("missing MCP-Session-Id header");
            Err((StatusCode::NOT_FOUND, "missing MCP-Session-Id header").into_response())
        }
    }
}

/// Check `Authorization: Bearer` header. Returns 401/403 on failure.
/// Returns `Ok(None)` if no validator is configured.
#[cfg(feature = "auth")]
#[allow(clippy::result_large_err)]
pub(crate) fn check_bearer(
    headers: &HeaderMap,
    validator: &Option<Arc<dyn crate::auth::TokenValidator>>,
    metadata_url: &Option<String>,
) -> Result<Option<crate::auth::TokenClaims>, Response> {
    let Some(validator) = validator else {
        return Ok(None);
    };
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let make_401 = |msg: &str| {
        let header = metadata_url
            .as_deref()
            .map(crate::auth::www_authenticate_header)
            .unwrap_or_else(|| "Bearer".into());
        (
            StatusCode::UNAUTHORIZED,
            [("www-authenticate", header)],
            msg.to_string(),
        )
            .into_response()
    };

    match token {
        Some(t) => match validator.validate_token(t) {
            crate::auth::TokenValidation::Valid(claims) => Ok(Some(claims)),
            crate::auth::TokenValidation::InsufficientScope { required } => {
                let header = crate::auth::insufficient_scope_header(&required);
                Err((
                    StatusCode::FORBIDDEN,
                    [("www-authenticate", header)],
                    "insufficient scope",
                )
                    .into_response())
            }
            crate::auth::TokenValidation::WrongResource { expected, .. } => {
                let header = metadata_url
                    .as_deref()
                    .map(crate::auth::www_authenticate_header)
                    .unwrap_or_else(|| "Bearer".into());
                Err((
                    StatusCode::FORBIDDEN,
                    [("www-authenticate", header)],
                    format!("token not valid for resource: {expected}"),
                )
                    .into_response())
            }
            crate::auth::TokenValidation::Expired => Err(make_401("token expired")),
            crate::auth::TokenValidation::Invalid(reason) => Err(make_401(&reason)),
            crate::auth::TokenValidation::Missing => Err(make_401("bearer token required")),
        },
        None => Err(make_401("bearer token required")),
    }
}
