use crate::domain::{DebugEvent, DebugEventSender, Provider};
use crate::tools::AgxToolCall;
use anyhow::Context;
use chrono::Local;
use colored::Colorize;
use futures::StreamExt;
use rig::OneOrMany;
use rig::agent::Agent;
use rig::completion::{Completion, CompletionModel};
use rig::message::{
    AssistantContent, Message, ToolCall, ToolResult, ToolResultContent, UserContent,
};
use rig::streaming::StreamedAssistantContent;
use rustyline::DefaultEditor;
use std::path::PathBuf;

const BANNER: &str = include_str!("assets/logo.txt");
const COMMANDS: &str = include_str!("assets/commands.txt");

enum ToolCallConfirmation {
    Proceed,
    Reject,
    Feedback(String),
}

pub struct Session<M>
where
    M: CompletionModel + 'static,
{
    agent: Agent<M>,
    editor: DefaultEditor,
    project_dir: PathBuf,
    project_log_dir: PathBuf,
    chats_dir: PathBuf,
    provider: Provider,
    model_name: String,
    debug_tx: Option<DebugEventSender>,
    chat_history: Vec<Message>,
    print_newline_before_prompt: bool,
}

impl<M> Session<M>
where
    M: CompletionModel + 'static,
{
    pub fn new(
        agent: Agent<M>,
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

        Ok(Self {
            agent,
            editor,
            project_dir,
            project_log_dir,
            chats_dir,
            provider,
            model_name: model_name.into(),
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
        let metadata = format!(
            "{}  {}",
            format!("[{}/{}]", &self.provider, &self.model_name).yellow(),
            self.project_dir.to_string_lossy().blue(),
        );

        loop {
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
                    println!("{}", "\ninterrupted".red());
                    if let Some(tx) = &self.debug_tx {
                        tx.send(DebugEvent::interrupted());
                    }
                    return;
                }
                result = self.stream_llm_response(prompt.clone()) => {
                    match result {
                        Ok(r) => r,
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

            for tool_call in tool_calls {
                let id = tool_call.id.clone();
                let call_id = tool_call.call_id.clone();

                let tool_call = match AgxToolCall::try_from(tool_call) {
                    Ok(t) => t,
                    Err(e) => {
                        tool_results.push(ToolResult {
                            id,
                            call_id,
                            content: OneOrMany::one(ToolResultContent::text(format!(
                                "failed to parse tool call: {}",
                                e
                            ))),
                        });
                        continue;
                    }
                };

                println!(
                    "{}",
                    format!("[tool call] {}", tool_call.repr()).bright_purple()
                );

                let details = match tool_call.details().await {
                    Ok(d) => d,
                    Err(e) => {
                        tool_results.push(ToolResult {
                            id,
                            call_id,
                            content: OneOrMany::one(ToolResultContent::text(format!(
                                "tool call failed: {}",
                                e.message()
                            ))),
                        });
                        continue;
                    }
                };

                if let Some(info) = details {
                    println!("{}", info);
                }

                let confirmation = if tool_call.needs_confirmation() {
                    self.confirm_tool_call()
                } else {
                    ToolCallConfirmation::Proceed
                };

                match confirmation {
                    ToolCallConfirmation::Proceed => {
                        let tool_result = tokio::select! {
                            Ok(_) = tokio::signal::ctrl_c() => {
                                println!("{}", "\ninterrupted".red());
                                if let Some(tx) = &self.debug_tx {
                                    tx.send(DebugEvent::tool_result(&id, call_id.clone(), "tool call interrupted by user"));
                                    tx.send(DebugEvent::interrupted());
                                }
                                let interrupted_result = ToolResult {
                                    id: id.clone(),
                                    call_id,
                                    content: OneOrMany::one(ToolResultContent::text("tool call interrupted by user")),
                                };
                                tool_results.push(interrupted_result);

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
                                        if let Some(tx) = &self.debug_tx {
                                            tx.send(DebugEvent::tool_result(&id, call_id.clone(), &output));
                                        }
                                        ToolResult {
                                            id,
                                            call_id,
                                            content: OneOrMany::one(ToolResultContent::text(output)),
                                        }
                                    },
                                    Err(e) => {
                                        print_error(anyhow::anyhow!("{}", e));
                                        let error_str = e.to_string();
                                        if let Some(tx) = &self.debug_tx {
                                            tx.send(DebugEvent::tool_result(&id, call_id.clone(), &error_str));
                                        }
                                        ToolResult {
                                            id,
                                            call_id,
                                            content: OneOrMany::one(ToolResultContent::text(error_str)),
                                        }
                                    }
                                }
                            }
                        };
                        tool_results.push(tool_result);
                    }
                    ToolCallConfirmation::Reject => {
                        println!("{}", "conversation stopped".red());
                        return;
                    }
                    ToolCallConfirmation::Feedback(text) => {
                        tool_results.push(ToolResult {
                            id,
                            call_id,
                            content: OneOrMany::one(ToolResultContent::text(format!(
                                "user rejected tool call with feedback: {}",
                                text
                            ))),
                        });
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
            .context("couldn't build LLM request builder")?;

        let mut stream = request_builder
            .stream()
            .await
            .context("couldn't build LLM request stream")?;

        if let Some(tx) = &self.debug_tx {
            tx.send(DebugEvent::llm_request(&prompt, &self.chat_history));
        }

        self.chat_history.push(prompt);

        let mut response_text = String::new();

        let mut tool_calls = vec![];

        while let Some(result) = stream.next().await {
            match result {
                Ok(content) => match content {
                    StreamedAssistantContent::Text(text) => {
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
                    StreamedAssistantContent::Final(_) => {
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

    fn confirm_tool_call(&mut self) -> ToolCallConfirmation {
        // TODO: temporary hack to skip HITL
        if std::env::var("AGX_SKIP_HITL")
            .map(|s| s == "1")
            .unwrap_or_default()
        {
            return ToolCallConfirmation::Proceed;
        }

        match self
            .editor
            .readline("proceed? (enter=yes, n=stop, or type feedback) ")
        {
            Ok(input) => {
                let trimmed = input.trim();
                match trimmed {
                    "" => ToolCallConfirmation::Proceed,
                    "n" | "no" => ToolCallConfirmation::Reject,
                    feedback => ToolCallConfirmation::Feedback(feedback.to_string()),
                }
            }
            Err(_) => ToolCallConfirmation::Reject,
        }
    }
}

fn print_error(error: anyhow::Error) {
    println!("{}", format!("error: {:?}", error).red());
}
