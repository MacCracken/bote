use thiserror::Error;

#[derive(Debug, Error)]
pub enum BoteError {
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("invalid params for tool '{tool}': {reason}")]
    InvalidParams { tool: String, reason: String },
    #[error("tool execution failed: {tool} — {reason}")]
    ExecFailed { tool: String, reason: String },
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("transport closed")]
    TransportClosed,
    #[error("transport bind failed: {0}")]
    BindFailed(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl BoteError {
    /// Convert to a JSON-RPC error code.
    pub fn rpc_code(&self) -> i32 {
        match self {
            Self::Parse(_) => -32700,
            Self::Protocol(_) => -32600,
            Self::ToolNotFound(_) => -32601,
            Self::InvalidParams { .. } => -32602,
            Self::ExecFailed { .. } => -32000,
            Self::TransportClosed => -32003,
            Self::BindFailed(_) => -32003,
            Self::Json(_) => -32700,
            Self::Io(_) => -32603,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpc_codes_all_variants() {
        assert_eq!(BoteError::Parse("bad".into()).rpc_code(), -32700);
        assert_eq!(BoteError::Protocol("bad".into()).rpc_code(), -32600);
        assert_eq!(BoteError::ToolNotFound("x".into()).rpc_code(), -32601);
        assert_eq!(
            BoteError::InvalidParams { tool: "x".into(), reason: "y".into() }.rpc_code(),
            -32602
        );
        assert_eq!(
            BoteError::ExecFailed { tool: "x".into(), reason: "y".into() }.rpc_code(),
            -32000
        );
        assert_eq!(BoteError::TransportClosed.rpc_code(), -32003);
        assert_eq!(BoteError::BindFailed("port in use".into()).rpc_code(), -32003);

        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken");
        assert_eq!(BoteError::Io(io_err).rpc_code(), -32603);
    }

    #[test]
    fn display_messages() {
        assert_eq!(BoteError::ToolNotFound("foo".into()).to_string(), "tool not found: foo");
        assert_eq!(
            BoteError::InvalidParams { tool: "t".into(), reason: "r".into() }.to_string(),
            "invalid params for tool 't': r"
        );
        assert_eq!(
            BoteError::ExecFailed { tool: "t".into(), reason: "r".into() }.to_string(),
            "tool execution failed: t — r"
        );
        assert_eq!(BoteError::Protocol("bad".into()).to_string(), "protocol error: bad");
        assert_eq!(BoteError::Parse("bad".into()).to_string(), "parse error: bad");
        assert_eq!(BoteError::TransportClosed.to_string(), "transport closed");
        assert_eq!(
            BoteError::BindFailed("port in use".into()).to_string(),
            "transport bind failed: port in use"
        );
    }

    #[test]
    fn json_error_from_serde() {
        let err: BoteError = serde_json::from_str::<serde_json::Value>("not json").unwrap_err().into();
        assert_eq!(err.rpc_code(), -32700);
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn io_error_from_std() {
        let err: BoteError = std::io::Error::new(std::io::ErrorKind::NotFound, "gone").into();
        assert_eq!(err.rpc_code(), -32603);
        assert!(err.to_string().contains("gone"));
    }
}
