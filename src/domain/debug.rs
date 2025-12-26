use chrono::{DateTime, Utc};
use rig::message::Message;
use serde::Serialize;
use tokio::sync::broadcast::{Receiver, Sender};

#[derive(Debug, Serialize, Clone)]
pub struct DebugEvent {
    pub timestamp: DateTime<Utc>,
    pub payload: DebugEventPayload,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DebugEventPayload {
    Message {
        message: Message,
    },
    LlmRequest {
        prompt: Message,
        history: Vec<Message>,
    },
}

impl From<&Message> for DebugEvent {
    fn from(value: &Message) -> Self {
        let timestamp = Utc::now();
        Self {
            timestamp,
            payload: DebugEventPayload::Message {
                message: value.clone(),
            },
        }
    }
}

impl DebugEvent {
    pub fn llm_request(prompt: &Message, history: &[Message]) -> Self {
        let timestamp = Utc::now();
        let payload = DebugEventPayload::LlmRequest {
            prompt: prompt.clone(),
            history: history.to_vec(),
        };

        Self { timestamp, payload }
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
