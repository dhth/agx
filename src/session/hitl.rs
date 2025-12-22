use std::io::Write;

use colored::Colorize;
use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::completion::CompletionModel;

#[derive(Clone)]
pub struct Hitl;

impl<M> StreamingPromptHook<M> for Hitl
where
    M: CompletionModel,
{
    async fn on_tool_call(
        &self,
        tool_name: &str,
        _: Option<String>,
        args: &str,
        cancel_sig: CancelSignal,
    ) {
        println!(
            "\n{}",
            format!("[request for tool-call] {} ({})", tool_name, args).bright_purple()
        );

        print!("press enter to proceed, 'n' to reject: ");
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_ok() && input.trim() == "n" {
            cancel_sig.cancel();
        }
    }
}
