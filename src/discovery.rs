//! Cross-node tool discovery — announce and discover tools across nodes via majra pub/sub.
//!
//! The [`DiscoveryService`] publishes tool announcements to the event bus and
//! provides a subscription mechanism for receiving announcements from peer nodes.
//!
//! Enable the `discovery` feature to use this module.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::events::{self, EventSink};
use crate::registry::ToolDef;

/// A tool announcement broadcast by a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ToolAnnouncement {
    /// Unique identifier of the announcing node.
    pub node_id: String,
    /// Tools available on this node.
    pub tools: Vec<ToolDef>,
}

impl ToolAnnouncement {
    #[must_use]
    pub fn new(node_id: impl Into<String>, tools: Vec<ToolDef>) -> Self {
        Self {
            node_id: node_id.into(),
            tools,
        }
    }
}

/// Service for announcing and discovering tools across nodes.
///
/// Uses the event sink to publish announcements and majra's pub/sub
/// for subscribing to peer announcements.
pub struct DiscoveryService {
    node_id: String,
    events: Arc<dyn EventSink>,
}

impl DiscoveryService {
    #[must_use]
    pub fn new(node_id: impl Into<String>, events: Arc<dyn EventSink>) -> Self {
        Self {
            node_id: node_id.into(),
            events,
        }
    }

    /// Announce this node's available tools to the network.
    pub fn announce(&self, tools: &[ToolDef]) {
        let announcement = ToolAnnouncement {
            node_id: self.node_id.clone(),
            tools: tools.to_vec(),
        };
        tracing::info!(
            node_id = %self.node_id,
            tool_count = tools.len(),
            "announcing tools"
        );
        self.events.publish(
            events::TOPIC_TOOL_ANNOUNCE,
            serde_json::to_value(&announcement).unwrap_or_default(),
        );
    }

    /// Get this node's ID.
    #[must_use]
    #[inline]
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}

/// Subscribe to tool announcements from peer nodes via majra pub/sub.
///
/// Returns a receiver that yields [`ToolAnnouncement`] values as they arrive.
/// The subscription uses majra's topic matching on the announce topic.
#[must_use]
pub fn subscribe(pubsub: &majra::pubsub::PubSub) -> DiscoveryReceiver {
    let rx = pubsub.subscribe(events::TOPIC_TOOL_ANNOUNCE);
    DiscoveryReceiver { rx }
}

/// Receiver for tool announcements from peer nodes.
pub struct DiscoveryReceiver {
    rx: tokio::sync::broadcast::Receiver<majra::pubsub::TopicMessage>,
}

impl DiscoveryReceiver {
    /// Try to receive the next announcement without blocking.
    pub fn try_recv(&mut self) -> Option<ToolAnnouncement> {
        match self.rx.try_recv() {
            Ok(msg) => match serde_json::from_value(msg.payload) {
                Ok(announcement) => Some(announcement),
                Err(e) => {
                    tracing::warn!(error = %e, "failed to deserialize tool announcement");
                    None
                }
            },
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::MajraEvents;
    use crate::registry::ToolSchema;
    use majra::pubsub::PubSub;
    use std::collections::HashMap;

    fn make_tool(name: &str) -> ToolDef {
        ToolDef {
            name: name.into(),
            description: format!("{name} tool"),
            input_schema: ToolSchema {
                schema_type: "object".into(),
                properties: HashMap::new(),
                required: vec![],
            },
            version: None,
            deprecated: None, annotations: None,
        }
    }

    fn make_service_and_receiver() -> (DiscoveryService, DiscoveryReceiver) {
        let pubsub = PubSub::new();
        let rx = subscribe(&pubsub);
        let events = MajraEvents::with_pubsub(pubsub);
        let service = DiscoveryService::new("node-1", Arc::new(events));
        (service, rx)
    }

    #[test]
    fn announce_round_trip() {
        let (service, mut rx) = make_service_and_receiver();

        let tools = vec![make_tool("echo"), make_tool("scan")];
        service.announce(&tools);

        let announcement = rx.try_recv().unwrap();
        assert_eq!(announcement.node_id, "node-1");
        assert_eq!(announcement.tools.len(), 2);
        assert_eq!(announcement.tools[0].name, "echo");
        assert_eq!(announcement.tools[1].name, "scan");
    }

    #[test]
    fn multiple_announcements() {
        let (service, mut rx) = make_service_and_receiver();

        service.announce(&[make_tool("t1")]);
        service.announce(&[make_tool("t2"), make_tool("t3")]);

        let a1 = rx.try_recv().unwrap();
        let a2 = rx.try_recv().unwrap();
        assert_eq!(a1.tools.len(), 1);
        assert_eq!(a2.tools.len(), 2);
    }

    #[test]
    fn empty_when_no_announcements() {
        let pubsub = PubSub::new();
        let mut rx = subscribe(&pubsub);
        assert!(rx.try_recv().is_none());
    }

    #[test]
    fn announcement_serialization() {
        let ann = ToolAnnouncement::new("node-x", vec![make_tool("foo")]);
        let json = serde_json::to_string(&ann).unwrap();
        assert!(json.contains("node-x"));
        assert!(json.contains("foo"));

        let back: ToolAnnouncement = serde_json::from_str(&json).unwrap();
        assert_eq!(back.node_id, "node-x");
        assert_eq!(back.tools.len(), 1);
    }

    #[test]
    fn node_id_accessor() {
        let (service, _rx) = make_service_and_receiver();
        assert_eq!(service.node_id(), "node-1");
    }
}
