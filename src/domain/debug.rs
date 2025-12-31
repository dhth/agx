use chrono::{DateTime, Utc};
use rig::message::{Message, Reasoning, ToolCall, ToolResult};
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
    LlmRequest {
        prompt: Message,
        history: Vec<Message>,
    },
    AssistantText {
        text: String,
    },
    ToolCall {
        tool_call: ToolCall,
    },
    Reasoning {
        reasoning: Reasoning,
    },
    ToolResult(ToolResult),
    StreamComplete,
    TurnComplete {
        history: Vec<Message>,
    },
    Interrupted,
    NewSession,
}

impl DebugEvent {
    pub fn llm_request(prompt: &Message, history: &[Message]) -> Self {
        Self::new(DebugEventPayload::LlmRequest {
            prompt: prompt.clone(),
            history: history.to_vec(),
        })
    }

    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self::new(DebugEventPayload::AssistantText { text: text.into() })
    }

    pub fn tool_call(tool_call: ToolCall) -> Self {
        Self::new(DebugEventPayload::ToolCall { tool_call })
    }

    pub fn reasoning(reasoning: Reasoning) -> Self {
        Self::new(DebugEventPayload::Reasoning { reasoning })
    }

    pub fn tool_result(result: &ToolResult) -> Self {
        Self::new(DebugEventPayload::ToolResult(result.clone()))
    }

    pub fn stream_complete() -> Self {
        Self::new(DebugEventPayload::StreamComplete)
    }

    pub fn turn_complete(history: &[Message]) -> Self {
        Self::new(DebugEventPayload::TurnComplete {
            history: history.to_vec(),
        })
    }

    pub fn interrupted() -> Self {
        Self::new(DebugEventPayload::Interrupted)
    }

    pub fn new_session() -> Self {
        Self::new(DebugEventPayload::NewSession)
    }

    fn new(payload: DebugEventPayload) -> Self {
        Self {
            timestamp: Utc::now(),
            payload,
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
