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
            Self::Json(_) => -32700,
            Self::Io(_) => -32603,
        }
    }
}
