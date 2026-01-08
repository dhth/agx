use rig::message::{Message, UserContent};

pub trait MessageExt {
    fn summary(&self) -> String;
}

impl MessageExt for Message {
    fn summary(&self) -> String {
        match self {
            Message::User { content } => match content.first() {
                UserContent::Text(text) => format!("text: {}", text.text()),
                UserContent::ToolResult(tool_result) => {
                    format!("tool_result: id={}", tool_result.id)
                }
                _ => "other".to_string(),
            },
            Message::Assistant { .. } => "assistant".to_string(),
        }
    }
}
