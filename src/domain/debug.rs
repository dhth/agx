use chrono::{DateTime, Utc};
use rig::{
    OneOrMany,
    message::{AssistantContent, Message, UserContent},
};
use serde::Serialize;
use tokio::sync::broadcast::{Receiver, Sender};

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DebugEvent {
    UserMsg {
        content: OneOrMany<UserContent>,
        timestamp: DateTime<Utc>,
    },
    AssistantMsg {
        id: Option<String>,
        content: OneOrMany<AssistantContent>,
        timestamp: DateTime<Utc>,
    },
    LlmRequest {
        prompt: Message,
        history: Vec<Message>,
        timestamp: DateTime<Utc>,
    },
}

impl From<Message> for DebugEvent {
    fn from(value: Message) -> Self {
        let timestamp = Utc::now();
        match value {
            Message::User { content } => Self::UserMsg {
                content,
                timestamp,
            },
            Message::Assistant { id, content } => Self::AssistantMsg {
                id,
                content,
                timestamp,
            },
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

#[derive(Clone)]
pub struct DebugEventReceiver(Sender<DebugEvent>);

impl DebugEventReceiver {
    pub fn new(sender: Sender<DebugEvent>) -> Self {
        Self(sender)
    }

    pub fn subscribe(&self) -> Receiver<DebugEvent> {
        self.0.subscribe()
    }
}
