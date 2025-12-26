use crate::domain::{DebugEvent, DebugEventSender};
use crate::tools::{get_tool_repr, needs_confirmation};
use colored::Colorize;
use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::completion::CompletionModel;
use std::io::Write;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Hitl {
    feedback: Arc<Mutex<Option<String>>>,
    debug_tx: Option<DebugEventSender>,
}

impl Hitl {
    pub fn new(debug_tx: Option<DebugEventSender>) -> Self {
        Self {
            feedback: Arc::new(Mutex::new(None)),
            debug_tx,
        }
    }
    pub fn take_feedback(&self) -> Option<String> {
        self.feedback.lock().ok()?.take()
    }
}

impl<M> StreamingPromptHook<M> for Hitl
where
    M: CompletionModel,
{
    async fn on_completion_call(
        &self,
        prompt: &rig::message::Message,
        history: &[rig::message::Message],
        _cancel_sig: CancelSignal,
    ) {
        if let Some(tx) = &self.debug_tx {
            tx.send(DebugEvent::llm_request(prompt, history));
        }
    }

    async fn on_tool_call(
        &self,
        tool_name: &str,
        _: Option<String>,
        args: &str,
        cancel_sig: CancelSignal,
    ) {
        if !needs_confirmation(tool_name) {
            return;
        }

        println!(
            "\n{}",
            format!("[request for tool-call] {}", get_tool_repr(tool_name, args)).bright_purple()
        );

        print!(
            "press enter to proceed, type 'n/no' to reject, or type your feedback if you want things to be done differently: "
        );
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_ok() {
            let trimmed = input.trim();
            if trimmed.is_empty() {
                return;
            }

            if (!trimmed.eq_ignore_ascii_case("n") && !trimmed.eq_ignore_ascii_case("no"))
                && let Ok(mut guard) = self.feedback.lock()
            {
                *guard = Some(trimmed.to_string());
            }
            cancel_sig.cancel();
        }
    }
}
