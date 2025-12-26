use chrono::{DateTime, Utc};
use rig::message::{Message, ToolCall};
use serde::Serialize;
use tokio::sync::broadcast::{Receiver, Sender};

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DebugEvent {
    LlmRequest {
        prompt: Message,
        history: Vec<Message>,
        timestamp: DateTime<Utc>,
    },
    TextDelta {
        text: String,
        timestamp: DateTime<Utc>,
    },
    ToolCall {
        call: ToolCall,
        timestamp: DateTime<Utc>,
    },
}

impl DebugEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            DebugEvent::LlmRequest { .. } => "llm_request",
            DebugEvent::TextDelta { .. } => "text_delta",
            DebugEvent::ToolCall { .. } => "tool_call",
        }
    }
}

#[derive(Clone)]
pub struct DebugEventSender(Sender<DebugEvent>);

impl DebugEventSender {
    pub fn new(sender: Sender<DebugEvent>) -> Self {
        Self(sender)
    }

    pub fn send(&self, event: DebugEvent) {
        let _ = self.0.send(event);
    }
}

pub struct DebugEventReceiver(Sender<DebugEvent>);

impl DebugEventReceiver {
    pub fn new(sender: Sender<DebugEvent>) -> Self {
        Self(sender)
    }

    pub fn subscribe(&self) -> Receiver<DebugEvent> {
        self.0.subscribe()
    }
}
