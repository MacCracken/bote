//! Streaming primitives — progress notifications, cancellation, and streaming handler support.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

/// Cancellation token — thread-safe flag that handlers poll to detect cancellation.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Check whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// A progress update emitted by a streaming handler.
#[derive(Debug, Clone, Serialize)]
pub struct ProgressUpdate {
    pub progress: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Sender half for emitting progress updates from a streaming handler.
#[derive(Debug, Clone)]
pub struct ProgressSender {
    tx: mpsc::Sender<ProgressUpdate>,
}

impl ProgressSender {
    pub(crate) fn new(tx: mpsc::Sender<ProgressUpdate>) -> Self {
        Self { tx }
    }

    /// Send a progress update. Silently ignores disconnected receivers.
    pub fn send(&self, update: ProgressUpdate) {
        let _ = self.tx.send(update);
    }

    /// Convenience: report progress out of total.
    pub fn report(&self, progress: u64, total: u64) {
        self.send(ProgressUpdate {
            progress,
            total: Some(total),
            message: None,
        });
    }

    /// Convenience: report progress with a message.
    pub fn report_msg(&self, progress: u64, total: u64, message: impl Into<String>) {
        self.send(ProgressUpdate {
            progress,
            total: Some(total),
            message: Some(message.into()),
        });
    }
}

/// Context passed to streaming tool handlers.
///
/// Provides a progress sender for emitting updates and a cancellation token
/// for checking whether the client has cancelled the request.
#[derive(Debug, Clone)]
pub struct StreamContext {
    pub progress: ProgressSender,
    pub cancellation: CancellationToken,
}

/// A streaming tool handler. Receives arguments and a `StreamContext` for
/// progress/cancellation. Returns the final result value.
pub type StreamingToolHandler =
    Arc<dyn Fn(serde_json::Value, StreamContext) -> serde_json::Value + Send + Sync>;

/// Create a connected `(StreamContext, mpsc::Receiver<ProgressUpdate>, CancellationToken)`.
pub(crate) fn make_stream_context() -> (StreamContext, mpsc::Receiver<ProgressUpdate>, CancellationToken) {
    let (tx, rx) = mpsc::channel();
    let token = CancellationToken::new();
    let ctx = StreamContext {
        progress: ProgressSender::new(tx),
        cancellation: token.clone(),
    };
    (ctx, rx, token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_token_lifecycle() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        let clone = token.clone();
        token.cancel();

        assert!(token.is_cancelled());
        assert!(clone.is_cancelled());
    }

    #[test]
    fn cancellation_token_default() {
        let token = CancellationToken::default();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn progress_sender_send_recv() {
        let (tx, rx) = mpsc::channel();
        let sender = ProgressSender::new(tx);

        sender.report(3, 10);
        sender.report_msg(5, 10, "halfway");

        let u1 = rx.recv().unwrap();
        assert_eq!(u1.progress, 3);
        assert_eq!(u1.total, Some(10));
        assert!(u1.message.is_none());

        let u2 = rx.recv().unwrap();
        assert_eq!(u2.progress, 5);
        assert_eq!(u2.message.as_deref(), Some("halfway"));
    }

    #[test]
    fn progress_sender_after_receiver_dropped() {
        let (tx, rx) = mpsc::channel();
        let sender = ProgressSender::new(tx);
        drop(rx);

        // Should not panic.
        sender.report(1, 1);
        sender.report_msg(1, 1, "done");
        sender.send(ProgressUpdate {
            progress: 1,
            total: None,
            message: None,
        });
    }

    #[test]
    fn make_stream_context_connected() {
        let (ctx, rx, token) = make_stream_context();

        ctx.progress.report(1, 5);
        let update = rx.recv().unwrap();
        assert_eq!(update.progress, 1);

        assert!(!ctx.cancellation.is_cancelled());
        token.cancel();
        assert!(ctx.cancellation.is_cancelled());
    }

    #[test]
    fn progress_update_serializes() {
        let update = ProgressUpdate {
            progress: 3,
            total: Some(10),
            message: Some("working".into()),
        };
        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("\"progress\":3"));
        assert!(json.contains("\"total\":10"));
        assert!(json.contains("\"working\""));
    }

    #[test]
    fn progress_update_omits_none_message() {
        let update = ProgressUpdate {
            progress: 1,
            total: None,
            message: None,
        };
        let json = serde_json::to_string(&update).unwrap();
        assert!(!json.contains("message"));
        assert!(!json.contains("total"));
    }
}
