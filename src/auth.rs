//! OAuth 2.1 authorization for MCP (spec 2025-11-25).
//!
//! Implements the MCP authorization flow:
//! - PKCE (S256 mandatory)
//! - Resource indicators (RFC 8707)
//! - Client metadata documents (HTTPS URL as client_id)
//! - Bearer token validation
//! - Scope challenges (403 + WWW-Authenticate)
//! - Protected resource metadata discovery (RFC 9728)
//!
//! ## Feature
//!
//! Gated behind the `auth` feature. Enable with:
//! ```toml
//! bote = { version = "0.91", features = ["auth"] }
//! ```

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// OAuth 2.1 configuration for an MCP server.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OAuthConfig {
    /// Resource URI for this MCP server (RFC 8707).
    /// e.g. "https://mcp.example.com/mcp"
    pub resource_uri: String,
    /// Authorization server metadata URL.
    /// e.g. "https://auth.example.com/.well-known/oauth-authorization-server"
    pub authorization_server: String,
    /// Scopes this MCP server supports.
    pub scopes_supported: Vec<String>,
    /// Whether to require authentication on all endpoints.
    pub require_auth: bool,
}

// ---------------------------------------------------------------------------
// PKCE
// ---------------------------------------------------------------------------

/// PKCE code challenge method (only S256 per MCP spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CodeChallengeMethod {
    /// SHA-256 hash of the code verifier.
    S256,
}

/// Generate a PKCE code verifier (43-128 characters, URL-safe).
///
/// # Errors
///
/// Returns an error if the OS random number generator fails.
pub fn generate_code_verifier() -> crate::Result<String> {
    use std::fmt::Write;
    let bytes: [u8; 32] = rand_bytes()?;
    let mut verifier = String::with_capacity(64);
    for b in &bytes {
        let _ = write!(verifier, "{:02x}", b);
    }
    Ok(verifier)
}

/// Compute the S256 code challenge from a verifier.
#[must_use]
pub fn compute_code_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(verifier.as_bytes());
    base64_url_encode(&hash)
}

/// Verify a PKCE code challenge against the verifier.
#[must_use]
pub fn verify_pkce(verifier: &str, challenge: &str) -> bool {
    let computed = compute_code_challenge(verifier);
    // Constant-time comparison to prevent timing attacks
    constant_time_eq(computed.as_bytes(), challenge.as_bytes())
}

// ---------------------------------------------------------------------------
// Token validation
// ---------------------------------------------------------------------------

/// A validated Bearer token with extracted claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TokenClaims {
    /// Subject (client ID or user ID).
    pub sub: String,
    /// Granted scopes.
    pub scopes: HashSet<String>,
    /// Expiration timestamp (Unix epoch seconds).
    pub exp: u64,
    /// Resource URI this token was issued for.
    pub resource: Option<String>,
}

impl TokenClaims {
    /// Check if the token has a specific scope.
    #[must_use]
    #[inline]
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.contains(scope)
    }

    /// Check if the token is expired.
    #[must_use]
    #[inline]
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now >= self.exp
    }

    /// Check if the token is valid for the given resource URI.
    #[must_use]
    #[inline]
    pub fn valid_for_resource(&self, resource_uri: &str) -> bool {
        self.resource
            .as_ref()
            .map(|r| r == resource_uri)
            .unwrap_or(true) // No resource claim = valid for any
    }
}

/// Result of a token validation attempt.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum TokenValidation {
    /// Token is valid.
    Valid(TokenClaims),
    /// Token is expired.
    Expired,
    /// Token has insufficient scope.
    InsufficientScope { required: String },
    /// Token is for the wrong resource.
    WrongResource { expected: String, got: String },
    /// Token is missing or malformed.
    Missing,
    /// Token validation failed.
    Invalid(String),
}

// ---------------------------------------------------------------------------
// Protected Resource Metadata (RFC 9728)
// ---------------------------------------------------------------------------

/// OAuth protected resource metadata (RFC 9728).
///
/// Served at `/.well-known/oauth-protected-resource` or referenced
/// in `WWW-Authenticate: Bearer resource_metadata="..."` headers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProtectedResourceMetadata {
    /// Canonical resource URI.
    pub resource: String,
    /// Authorization servers that can issue tokens for this resource.
    pub authorization_servers: Vec<String>,
    /// Scopes this resource supports.
    #[serde(default)]
    pub scopes_supported: Vec<String>,
}

impl ProtectedResourceMetadata {
    /// Build from an OAuthConfig.
    #[must_use]
    pub fn from_config(config: &OAuthConfig) -> Self {
        Self {
            resource: config.resource_uri.clone(),
            authorization_servers: vec![config.authorization_server.clone()],
            scopes_supported: config.scopes_supported.clone(),
        }
    }
}

/// Build a `WWW-Authenticate` header for a 401 response.
#[must_use]
pub fn www_authenticate_header(metadata_url: &str) -> String {
    format!(r#"Bearer resource_metadata="{metadata_url}""#)
}

/// Build a `WWW-Authenticate` header for a 403 insufficient scope response.
#[must_use]
pub fn insufficient_scope_header(required_scope: &str) -> String {
    format!(r#"Bearer error="insufficient_scope", scope="{required_scope}""#)
}

// ---------------------------------------------------------------------------
// Token validator trait
// ---------------------------------------------------------------------------

/// Trait for validating bearer tokens.
///
/// Consumers implement this to integrate with their auth server (JWT
/// validation, token introspection, etc.). Bote's transport middleware
/// calls [`validate_token`](Self::validate_token) on every authenticated
/// request.
pub trait TokenValidator: Send + Sync {
    /// Validate a bearer token string and return the result.
    fn validate_token(&self, token: &str) -> TokenValidation;
}

// ---------------------------------------------------------------------------
// Client metadata document
// ---------------------------------------------------------------------------

/// OAuth client metadata document (fetched from client_id URL).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClientMetadata {
    /// Client identifier (HTTPS URL).
    pub client_id: String,
    /// Redirect URIs.
    #[serde(default)]
    pub redirect_uris: Vec<String>,
    /// Client name.
    #[serde(default)]
    pub client_name: Option<String>,
    /// Supported grant types.
    #[serde(default)]
    pub grant_types: Vec<String>,
    /// Supported response types.
    #[serde(default)]
    pub response_types: Vec<String>,
    /// Token endpoint auth method.
    #[serde(default)]
    pub token_endpoint_auth_method: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn rand_bytes<const N: usize>() -> crate::Result<[u8; N]> {
    let mut buf = [0u8; N];
    getrandom::getrandom(&mut buf).map_err(|e| crate::error::BoteError::ExecFailed {
        tool: "auth".into(),
        reason: format!("getrandom failed: {e}"),
    })?;
    Ok(buf)
}

fn base64_url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_roundtrip() {
        let verifier = generate_code_verifier().unwrap();
        assert!(verifier.len() >= 43);
        let challenge = compute_code_challenge(&verifier);
        assert!(verify_pkce(&verifier, &challenge));
    }

    #[test]
    fn pkce_wrong_verifier_fails() {
        let verifier = generate_code_verifier().unwrap();
        let challenge = compute_code_challenge(&verifier);
        assert!(!verify_pkce("wrong-verifier", &challenge));
    }

    #[test]
    fn token_claims_scope_check() {
        let claims = TokenClaims {
            sub: "client-1".into(),
            scopes: ["read".into(), "write".into()].into_iter().collect(),
            exp: u64::MAX,
            resource: None,
        };
        assert!(claims.has_scope("read"));
        assert!(claims.has_scope("write"));
        assert!(!claims.has_scope("admin"));
    }

    #[test]
    fn token_claims_expired() {
        let claims = TokenClaims {
            sub: "client-1".into(),
            scopes: HashSet::new(),
            exp: 0, // already expired
            resource: None,
        };
        assert!(claims.is_expired());
    }

    #[test]
    fn token_claims_not_expired() {
        let claims = TokenClaims {
            sub: "client-1".into(),
            scopes: HashSet::new(),
            exp: u64::MAX,
            resource: None,
        };
        assert!(!claims.is_expired());
    }

    #[test]
    fn token_resource_validation() {
        let claims = TokenClaims {
            sub: "client-1".into(),
            scopes: HashSet::new(),
            exp: u64::MAX,
            resource: Some("https://mcp.example.com/mcp".into()),
        };
        assert!(claims.valid_for_resource("https://mcp.example.com/mcp"));
        assert!(!claims.valid_for_resource("https://other.example.com/mcp"));
    }

    #[test]
    fn token_no_resource_valid_for_any() {
        let claims = TokenClaims {
            sub: "client-1".into(),
            scopes: HashSet::new(),
            exp: u64::MAX,
            resource: None,
        };
        assert!(claims.valid_for_resource("https://anything.com"));
    }

    #[test]
    fn protected_resource_metadata_from_config() {
        let config = OAuthConfig {
            resource_uri: "https://mcp.example.com/mcp".into(),
            authorization_server: "https://auth.example.com".into(),
            scopes_supported: vec!["read".into(), "write".into()],
            require_auth: true,
        };
        let meta = ProtectedResourceMetadata::from_config(&config);
        assert_eq!(meta.resource, "https://mcp.example.com/mcp");
        assert_eq!(meta.authorization_servers.len(), 1);
        assert_eq!(meta.scopes_supported.len(), 2);
    }

    #[test]
    fn www_authenticate_header_format() {
        let header =
            www_authenticate_header("https://mcp.example.com/.well-known/oauth-protected-resource");
        assert!(header.starts_with("Bearer resource_metadata="));
    }

    #[test]
    fn insufficient_scope_header_format() {
        let header = insufficient_scope_header("admin");
        assert!(header.contains("insufficient_scope"));
        assert!(header.contains("admin"));
    }

    #[test]
    fn oauth_config_default() {
        let config = OAuthConfig::default();
        assert!(!config.require_auth);
        assert!(config.resource_uri.is_empty());
    }

    #[test]
    fn client_metadata_serde() {
        let meta = ClientMetadata {
            client_id: "https://app.example.com/oauth/client-metadata.json".into(),
            redirect_uris: vec!["https://app.example.com/callback".into()],
            client_name: Some("Test App".into()),
            grant_types: vec!["authorization_code".into()],
            response_types: vec!["code".into()],
            token_endpoint_auth_method: Some("none".into()),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let back: ClientMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back.client_id, meta.client_id);
    }
}
