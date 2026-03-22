//! Event publishing — broadcast tool call and registration events.
//!
//! The [`EventSink`] trait defines the interface. Enable the `events` feature
//! for the [`MajraEvents`] implementation backed by majra's pub/sub engine.

/// Event topic for tool call completion.
pub const TOPIC_TOOL_COMPLETED: &str = "bote/tool/completed";
/// Event topic for tool call failure.
pub const TOPIC_TOOL_FAILED: &str = "bote/tool/failed";
/// Event topic for tool registration.
pub const TOPIC_TOOL_REGISTERED: &str = "bote/tool/registered";

/// Trait for event publishing backends.
pub trait EventSink: Send + Sync {
    /// Publish an event to a topic.
    fn publish(&self, topic: &str, payload: serde_json::Value);
}

/// No-op event sink (used when event publishing is disabled).
impl EventSink for () {
    fn publish(&self, _topic: &str, _payload: serde_json::Value) {}
}

// --- majra integration (feature = "events") ---

#[cfg(feature = "events")]
mod majra_impl {
    use super::*;
    use majra::pubsub::PubSub;

    /// Event sink backed by majra's pub/sub engine.
    pub struct MajraEvents {
        pubsub: PubSub,
    }

    impl MajraEvents {
        pub fn new() -> Self {
            Self {
                pubsub: PubSub::new(),
            }
        }

        /// Create from an existing PubSub instance.
        pub fn with_pubsub(pubsub: PubSub) -> Self {
            Self { pubsub }
        }

        /// Access the underlying PubSub (e.g. for subscribing).
        pub fn pubsub(&self) -> &PubSub {
            &self.pubsub
        }
    }

    impl Default for MajraEvents {
        fn default() -> Self {
            Self::new()
        }
    }

    impl EventSink for MajraEvents {
        fn publish(&self, topic: &str, payload: serde_json::Value) {
            self.pubsub.publish(topic, payload);
        }
    }
}

#[cfg(feature = "events")]
pub use majra_impl::MajraEvents;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_sink_compiles() {
        let sink: &dyn EventSink = &();
        sink.publish("test/topic", serde_json::json!({"hello": "world"}));
    }
}

#[cfg(all(test, feature = "events"))]
mod events_tests {
    use super::*;
    use majra::pubsub::PubSub;

    #[test]
    fn majra_events_publishes() {
        let pubsub = PubSub::new();
        let mut rx = pubsub.subscribe("bote/tool/#");
        let events = MajraEvents::with_pubsub(pubsub);

        events.publish(
            "bote/tool/completed",
            serde_json::json!({"tool_name": "echo", "duration_ms": 10}),
        );

        let msg = rx.try_recv().unwrap();
        assert_eq!(msg.topic, "bote/tool/completed");
        assert_eq!(msg.payload["tool_name"], "echo");
    }

    #[test]
    fn majra_events_multiple_topics() {
        let pubsub = PubSub::new();
        let mut rx = pubsub.subscribe("bote/#");
        let events = MajraEvents::with_pubsub(pubsub);

        events.publish("bote/tool/called", serde_json::json!({"tool_name": "a"}));
        events.publish("bote/tool/completed", serde_json::json!({"tool_name": "a"}));
        events.publish("bote/tool/registered", serde_json::json!({"tool_name": "b"}));

        let m1 = rx.try_recv().unwrap();
        let m2 = rx.try_recv().unwrap();
        let m3 = rx.try_recv().unwrap();
        assert_eq!(m1.topic, "bote/tool/called");
        assert_eq!(m2.topic, "bote/tool/completed");
        assert_eq!(m3.topic, "bote/tool/registered");
    }
}
