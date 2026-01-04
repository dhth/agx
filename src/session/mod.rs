mod hitl;

use crate::config::save_local_config;
use crate::domain::{CmdPattern, Config, DebugEvent, DebugEventSender, Provider};
use crate::tools::AgxToolCall;
use anyhow::Context;
use chrono::{Local, Utc};
use colored::Colorize;
use futures::StreamExt;
use hitl::Approvals;
use rig::OneOrMany;
use rig::agent::Agent;
use rig::completion::{Completion, CompletionModel, GetTokenUsage};
use rig::message::{
    AssistantContent, Message, ToolCall, ToolResult, ToolResultContent, UserContent,
};
use rig::streaming::StreamedAssistantContent;
use rustyline::DefaultEditor;
use std::borrow::Cow;
use std::path::PathBuf;
use std::str::FromStr;

const BANNER: &str = include_str!("assets/logo.txt");
const COMMANDS: &str = include_str!("assets/commands.txt");
const SYSTEM_PROMPT: &str = include_str!("assets/system-prompt.txt");

enum ToolCallConfirmation {
    Approved,
    AutoApproved,
    Rejected,
    FeedbackProvided(String),
}

pub struct Session<M>
where
    M: CompletionModel + 'static,
{
    config: Config,
    agent: Agent<M>,
    project_context: Option<String>,
    editor: DefaultEditor,
    approvals: Approvals,
    project_dir: PathBuf,
    project_log_dir: PathBuf,
    chats_dir: PathBuf,
    provider: Provider,
    model_name: String,
    tokens_in_context: u64,
    debug_tx: Option<DebugEventSender>,
    chat_history: Vec<Message>,
    print_newline_before_prompt: bool,
}

impl<M> Session<M>
where
    M: CompletionModel + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: Config,
        agent: Agent<M>,
        project_context: Option<String>,
        project_dir: PathBuf,
        project_log_dir: PathBuf,
        provider: Provider,
        model_name: impl Into<String>,
        debug_tx: Option<DebugEventSender>,
    ) -> anyhow::Result<Self> {
        let chats_dir = project_log_dir
            .join("chats")
            .join(Local::now().format("%Y-%m-%d-%H-%M-%S").to_string());

        let editor = DefaultEditor::new()?;
        let approvals = Approvals {
            fs_changes: false,
            approved_commands: config.approved_commands.clone(),
        };

        Ok(Self {
            config,
            agent,
            project_context,
            editor,
            approvals,
            project_dir,
            project_log_dir,
            chats_dir,
            provider,
            model_name: model_name.into(),
            tokens_in_context: 0,
            debug_tx,
            chat_history: Vec::new(),
            print_newline_before_prompt: false,
        })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.chats_dir)
            .await
            .with_context(|| {
                format!(
                    "failed to create directory for storing chat: {:?}",
                    &self.chats_dir,
                )
            })?;
        let history_file_path = self.project_log_dir.join("history.txt");

        let _ = self.editor.load_history(&history_file_path);

        print!(
            "
{}
",
            BANNER.purple(),
        );

        let prompt_marker = "> ".bright_blue().to_string();
        loop {
            let token_info = if self.tokens_in_context > 0 {
                Some(format!("  ~{} tokens", get_token_count_repr(self.tokens_in_context)).green())
            } else {
                None
            };
            let metadata = format!(
                "{}  {}{}",
                format!("[{}/{}]", &self.provider, &self.model_name).yellow(),
                self.project_dir.to_string_lossy().blue(),
                token_info.unwrap_or_default(),
            );

            let prefix = if self.print_newline_before_prompt {
                "\n"
            } else {
                self.print_newline_before_prompt = true;
                ""
            };
            println!("{}{}", prefix, metadata);
            let user_input = self
                .editor
                .readline(&prompt_marker)
                .context("couldn't read input")?;

            match user_input.trim() {
                "" => {}
                "clear" => {
                    _ = self.editor.clear_screen();
                    self.print_newline_before_prompt = false;
                    continue;
                }
                "/help" => {
                    print!("{}", COMMANDS.green());
                    continue;
                }
                "/new" => {
                    self.chat_history.clear();
                    self.tokens_in_context = 0;
                    self.print_newline_before_prompt = false;
                    self.chats_dir = self
                        .project_log_dir
                        .join("chats")
                        .join(Local::now().format("%Y-%m-%d-%H-%M-%S").to_string());

                    tokio::fs::create_dir_all(&self.chats_dir)
                        .await
                        .with_context(|| {
                            format!(
                                "failed to create directory for storing chat: {:?}",
                                &self.chats_dir,
                            )
                        })?;

                    if let Some(tx) = &self.debug_tx {
                        tx.send(DebugEvent::new_session());
                    }

                    _ = self.editor.clear_screen();
                    continue;
                }
                "/approvals" => {
                    print!("{}", self.approvals.to_string().green());
                    continue;
                }
                "/quit" | "/exit" | "bye" | ":q" => {
                    break;
                }
                p => {
                    _ = self.editor.add_history_entry(p);

                    self.handle_prompt(p).await;
                    if let Some(tx) = &self.debug_tx {
                        tx.send(DebugEvent::turn_complete(&self.chat_history));
                    }
                }
            }
        }

        let _ = self.editor.save_history(&history_file_path);

        Ok(())
    }

    async fn handle_prompt(&mut self, prompt: &str) {
        let mut prompt = Message::user(prompt);

        loop {
            let (response_text, tool_calls) = tokio::select! {
                Ok(_) = tokio::signal::ctrl_c() => {
                    println!("{}", "\ninterrupted (prompt discarded)".red());
                    if let Some(tx) = &self.debug_tx {
                        tx.send(DebugEvent::interrupted());
                    }
                    return;
                }
                result = self.stream_llm_response(prompt.clone()) => {
                    match result {
                        Ok(r) => {
                            self.chat_history.push(prompt);
                            r
                        },
                        Err(e) => {
                            print_error(e);
                            break;
                        }
                    }
                }
            };

            let mut assistant_contents = vec![];

            if !response_text.is_empty() {
                assistant_contents.push(AssistantContent::text(&response_text));
            }

            for tc in &tool_calls {
                assistant_contents.push(AssistantContent::ToolCall(tc.clone()));
            }

            if !assistant_contents.is_empty() {
                #[allow(clippy::expect_used)]
                self.chat_history.push(Message::Assistant {
                    id: None,
                    content: OneOrMany::many(assistant_contents)
                        .expect("should've pushed assistant contents to chat history"),
                });
            }

            if tool_calls.is_empty() {
                break;
            }

            let mut tool_results = vec![];

            for (i, tool_call) in tool_calls.iter().enumerate() {
                let id = tool_call.id.clone();
                let call_id = tool_call.call_id.clone();

                let tool_call = match AgxToolCall::try_from(tool_call.clone()) {
                    Ok(t) => t,
                    Err(e) => {
                        let result = make_tool_result(
                            id,
                            call_id,
                            format!("failed to parse tool call: {e}"),
                        );
                        self.push_tool_result(&mut tool_results, result);
                        continue;
                    }
                };

                let confirmation = if tool_call.needs_confirmation() {
                    let details = match tool_call.details().await {
                        Ok(d) => d,
                        Err(e) => {
                            let result = make_tool_result(id, call_id, e.to_string());
                            self.push_tool_result(&mut tool_results, result);
                            continue;
                        }
                    };

                    self.confirm_tool_call(&tool_call, details.as_deref()).await
                } else {
                    ToolCallConfirmation::Approved
                };

                match confirmation {
                    ToolCallConfirmation::Approved | ToolCallConfirmation::AutoApproved => {
                        tokio::select! {
                            Ok(_) = tokio::signal::ctrl_c() => {
                                println!("{}", "\ninterrupted".red());
                                let result = make_tool_result(
                                    id.clone(),
                                    call_id,
                                    "tool call interrupted by user",
                                );
                                self.push_tool_result(&mut tool_results, result);

                                if let Some(tx) = &self.debug_tx {
                                    tx.send(DebugEvent::interrupted());
                                }

                                self.push_skipped_results(
                                    &tool_calls[i + 1..],
                                    &mut tool_results,
                                    "tool call skipped because user interrupted a previous tool call",
                                );

                                self.chat_history.push(Message::User {
                                    #[allow(clippy::expect_used)]
                                    content: OneOrMany::many(
                                        tool_results
                                            .into_iter()
                                            .map(UserContent::ToolResult)
                                            .collect::<Vec<_>>(),
                                    )
                                    .expect("tool results should've been added to chat history"),
                                });

                                return;
                            }
                            result = tool_call.execute() => {
                                match result {
                                    Ok(output) => {
                                        let result = make_tool_result(id, call_id, output);
                                        self.push_tool_result(&mut tool_results, result);
                                    },
                                    Err(e) => {
                                        print_error(anyhow::anyhow!("{}", e));
                                        let result = make_tool_result(id, call_id, e.to_string());
                                        self.push_tool_result(&mut tool_results, result);
                                    }
                                }
                            }
                        }
                    }
                    ToolCallConfirmation::Rejected => {
                        println!("{}", "conversation stopped".red());
                        let result = make_tool_result(id, call_id, "user rejected tool call");
                        self.push_tool_result(&mut tool_results, result);
                        self.push_skipped_results(
                            &tool_calls[i + 1..],
                            &mut tool_results,
                            "tool call skipped because user rejected a previous tool call",
                        );
                        self.chat_history.push(Message::User {
                            #[allow(clippy::expect_used)]
                            content: OneOrMany::many(
                                tool_results
                                    .into_iter()
                                    .map(UserContent::ToolResult)
                                    .collect::<Vec<_>>(),
                            )
                            .expect("tool results should've been added to chat history"),
                        });
                        return;
                    }
                    ToolCallConfirmation::FeedbackProvided(text) => {
                        println!("{}", "tool call rejected; providing feedback to LLM".red());
                        let result = make_tool_result(
                            id,
                            call_id,
                            format!("user rejected tool call with feedback: {text}"),
                        );
                        self.push_tool_result(&mut tool_results, result);
                        self.push_skipped_results(
                            &tool_calls[i + 1..],
                            &mut tool_results,
                            "tool call skipped because user provided feedback on a previous tool call",
                        );
                        break;
                    }
                }
            }

            if tool_results.is_empty() {
                break;
            }

            prompt = Message::User {
                #[allow(clippy::expect_used)]
                content: OneOrMany::many(
                    tool_results
                        .into_iter()
                        .map(UserContent::ToolResult)
                        .collect::<Vec<_>>(),
                )
                .expect("tool results should've been set as the next prompt"),
            };
        }
    }

    async fn stream_llm_response(
        &mut self,
        prompt: Message,
    ) -> anyhow::Result<(String, Vec<ToolCall>)> {
        let request_builder = self
            .agent
            .completion(prompt.clone(), self.chat_history.clone())
            .await
            .context("couldn't build LLM request builder")?
            .preamble(self.get_preamble());

        let mut stream = request_builder
            .stream()
            .await
            .context("couldn't build LLM request stream")?;

        if let Some(tx) = &self.debug_tx {
            tx.send(DebugEvent::llm_request(&prompt, &self.chat_history));
        }

        let mut response_text = String::new();

        let mut tool_calls = vec![];

        while let Some(result) = stream.next().await {
            match result {
                Ok(content) => match content {
                    StreamedAssistantContent::Text(text) => {
                        if response_text.is_empty() {
                            println!();
                        }
                        print!("{text}");
                        response_text.push_str(&text.text);
                    }
                    StreamedAssistantContent::ToolCall(tool_call) => {
                        if let Some(tx) = &self.debug_tx {
                            tx.send(DebugEvent::tool_call(tool_call.clone()));
                        }
                        tool_calls.push(tool_call);
                    }
                    StreamedAssistantContent::ToolCallDelta { .. } => {}
                    StreamedAssistantContent::Reasoning(reasoning) => {
                        print!("\n{}", "[reasoning] ".cyan());
                        for r in &reasoning.reasoning {
                            print!("{}", r.to_string().cyan());
                        }
                        if let Some(tx) = &self.debug_tx {
                            tx.send(DebugEvent::reasoning(reasoning));
                        }
                    }
                    StreamedAssistantContent::ReasoningDelta { .. } => {}
                    StreamedAssistantContent::Final(r) => {
                        if let Some(usage) = r.token_usage() {
                            self.tokens_in_context = usage.total_tokens;
                        }
                        if !response_text.is_empty()
                            && let Some(tx) = &self.debug_tx
                        {
                            tx.send(DebugEvent::assistant_text(&response_text));
                        }
                        if let Some(tx) = &self.debug_tx {
                            tx.send(DebugEvent::stream_complete());
                        }
                        println!();
                    }
                },
                Err(e) => {
                    anyhow::bail!(e);
                }
            }
        }

        Ok((response_text, tool_calls))
    }

    async fn confirm_tool_call(
        &mut self,
        tool_call: &AgxToolCall,
        details: Option<&str>,
    ) -> ToolCallConfirmation {
        // TODO: temporary hack to skip HITL
        if std::env::var("AGX_SKIP_HITL")
            .map(|s| s == "1")
            .unwrap_or_default()
        {
            return ToolCallConfirmation::Approved;
        }

        if self.approvals.is_tool_call_approved(tool_call) {
            return ToolCallConfirmation::AutoApproved;
        }

        println!(
            "{}",
            format!("[request for tool-call] {}", tool_call.repr()).bright_purple()
        );

        if let Some(info) = details {
            println!("{}", info);
        }

        let approval_line = match tool_call {
            AgxToolCall::CreateFile { .. } | AgxToolCall::EditFile { .. } => {
                Some("to allow all edits in this session".to_string())
            }
            AgxToolCall::RunCmd { args } => {
                if let Ok(cmd_pattern) = CmdPattern::from_str(&args.command) {
                    Some(format!(r#"to always allow "{cmd_pattern}" commands"#,))
                } else {
                    // TODO: this error shouldn't happen this deep in the call stack
                    None
                }
            }
            _ => None,
        };

        let confirmation_prompt = format!(
            "
type:
- y / <enter> to proceed
- a           {}
- n / no      to reject
- reject and provide feedback: ",
            approval_line.unwrap_or("to always approve this tool call".to_string())
        );

        match self.editor.readline(&confirmation_prompt) {
            Ok(input) => {
                let trimmed = input.trim();
                match trimmed {
                    "" | "y" => ToolCallConfirmation::Approved,
                    "a" => {
                        // TODO: this can be made nicer
                        if let Some(confirmation_msg) = self.approvals.save_approval(tool_call) {
                            if matches!(tool_call, AgxToolCall::RunCmd { .. }) {
                                self.config.approved_commands =
                                    self.approvals.approved_commands.clone();
                                if let Err(e) = save_local_config(&self.config)
                                    .await
                                    .context("couldn't update agx's local config")
                                {
                                    print_error(e);
                                }
                            }
                            println!("{}", confirmation_msg.green());
                        }

                        ToolCallConfirmation::AutoApproved
                    }
                    "n" | "no" => ToolCallConfirmation::Rejected,
                    feedback => ToolCallConfirmation::FeedbackProvided(feedback.to_string()),
                }
            }
            Err(_) => ToolCallConfirmation::Rejected,
        }
    }

    fn push_skipped_results(
        &self,
        remaining_tool_calls: &[ToolCall],
        tool_results: &mut Vec<ToolResult>,
        reason: &str,
    ) {
        for tc in remaining_tool_calls {
            let result = make_tool_result(tc.id.clone(), tc.call_id.clone(), reason);
            self.push_tool_result(tool_results, result);
        }
    }

    fn push_tool_result(&self, tool_results: &mut Vec<ToolResult>, result: ToolResult) {
        if let Some(tx) = &self.debug_tx {
            tx.send(DebugEvent::tool_result(&result));
        }
        tool_results.push(result);
    }

    fn get_preamble(&self) -> String {
        let now = Utc::now().format("%A, %B %d, %Y %H:%M UTC").to_string();
        let system_prompt = match &self.project_context {
            Some(p) => Cow::Owned(format!(
                "{}

The following is context specific to this project:

{}",
                SYSTEM_PROMPT, p
            )),
            None => Cow::Borrowed(SYSTEM_PROMPT),
        };
        format!(
            "{}

---
Extra information for you
Current directory: {}
Current date/time: {}
",
            system_prompt,
            self.project_dir.to_string_lossy(),
            now,
        )
    }
}

fn print_error(error: anyhow::Error) {
    println!("{}", format!("error: {:?}", error).red());
}

fn make_tool_result(id: String, call_id: Option<String>, text: impl Into<String>) -> ToolResult {
    ToolResult {
        id,
        call_id,
        content: OneOrMany::one(ToolResultContent::text(text)),
    }
}

fn get_token_count_repr(count: u64) -> String {
    if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}
