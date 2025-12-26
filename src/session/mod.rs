mod hitl;

use crate::domain::{DebugEvent, DebugEventSender, Provider};
use crate::tools::cancel::CancelTx;
use anyhow::Context;
use chrono::Local;
use colored::Colorize;
use futures::StreamExt;
use hitl::Hitl;
use rig::OneOrMany;
use rig::agent::{Agent, MultiTurnStreamItem};
use rig::completion::CompletionModel;
use rig::message::{AssistantContent, Message, UserContent};
use rig::streaming::{StreamedAssistantContent, StreamedUserContent, StreamingPrompt};
use std::path::PathBuf;
use tracing::{debug, error, info};

const BANNER: &str = include_str!("assets/logo.txt");
const COMMANDS: &str = include_str!("assets/commands.txt");

pub struct Session<M>
where
    M: CompletionModel + 'static,
{
    agent: Agent<M>,
    project_dir: PathBuf,
    project_log_dir: PathBuf,
    chats_dir: PathBuf,
    provider: Provider,
    model_name: String,
    cancel_tx: CancelTx,
    debug_tx: DebugEventSender,
    loop_count: usize,
    chat_history: Vec<Message>,
    hitl: Hitl,
    pending_prompt: Option<String>,
    print_newline_before_prompt: bool,
    cancellation_operational: bool,
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
        cancel_tx: CancelTx,
        debug_tx: DebugEventSender,
    ) -> Self {
        let chats_dir = project_log_dir
            .join("chats")
            .join(Local::now().format("%Y-%m-%d-%H-%M-%S").to_string());

        let debug_tx_clone = debug_tx.clone();

        Self {
            agent,
            project_dir,
            project_log_dir,
            chats_dir,
            provider,
            model_name: model_name.into(),
            cancel_tx,
            debug_tx,
            loop_count: 0,
            chat_history: Vec::new(),
            hitl: Hitl::new(debug_tx_clone),
            pending_prompt: None,
            print_newline_before_prompt: false,
            cancellation_operational: true,
        }
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

        let mut editor = rustyline::DefaultEditor::new()?;
        let _ = editor.load_history(&history_file_path);

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
            let user_input = if let Some(q) = self.pending_prompt.take() {
                q
            } else {
                editor
                    .readline(&prompt_marker)
                    .context("couldn't read input")?
            };

            match user_input.trim() {
                "" => {}
                "clear" => {
                    _ = editor.clear_screen();
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

                    _ = editor.clear_screen();
                    continue;
                }
                "/quit" | "/exit" | "bye" | ":q" => {
                    break;
                }
                p => {
                    _ = editor.add_history_entry(p);

                    self.save_to_chat_history(Message::user(p));
                    self.handle_prompt(p).await;
                    self.cancellation_operational = self.cancel_tx.reset();
                }
            }
        }

        let _ = editor.save_history(&history_file_path);

        Ok(())
    }

    async fn handle_prompt(&mut self, prompt: &str) {
        debug!(
            loop_index = self.loop_count,
            query = prompt,
            "user entered query"
        );

        let mut stream = self
            .agent
            .stream_prompt(prompt)
            .multi_turn(30)
            .with_hook(self.hitl.clone())
            .with_history(self.chat_history.clone())
            .await;

        let mut response_text = String::new();

        let mut stream_index = 0;

        loop {
            tokio::select! {
                Ok(_) = tokio::signal::ctrl_c() => {
                    self.print_newline_before_prompt = false;
                    if self.cancel_tx.is_cancelled() {
                        println!("{}", "\ncancellation in progress".yellow());
                        continue;
                    }

                    if let Some(Message::Assistant { content, .. }) = self.chat_history.last()
                        && let AssistantContent::ToolCall(_) = content.first()
                    {
                        if !self.cancellation_operational {
                            self.chat_history.pop();
                            println!("{}", "\nconversation stopped; ongoing command invocations might still be running (this is unexpected; let @dhth know about this via https://github.com/dhth/agx/issues)".red());
                            break;
                        }

                        info!(loop_index = self.loop_count, "user interrupted tool call invocation");
                        if !self.cancel_tx.cancel() {
                            self.cancellation_operational = false;
                            error!(loop_index = self.loop_count, stream_index, "couldn't send cancellation signal");
                        }

                        println!("{}", "\ncancelled; conveying this to the LLM...".red());
                    } else {
                        println!("{}", "\nconversation stopped".red());
                        break;
                    }
                }
                maybe_item = stream.next() => {
                    match maybe_item {
                        Some(item) => {
                            match item {
                                Ok(rig::agent::MultiTurnStreamItem::StreamAssistantItem(content)) => {
                                    match content {
                                        StreamedAssistantContent::Text(text) => {
                                            debug!(loop_index = self.loop_count, stream_index, kind="StreamedAssistantContent::Text", text = %text.text, "stream item received");
                                            print!("{}", text.text);
                                            response_text.push_str(&text.text);
                                        }
                                        StreamedAssistantContent::ToolCall(tool_call) => {
                                            if !response_text.is_empty() {
                                                self.save_to_chat_history(Message::assistant(&response_text));
                                                response_text.clear();
                                                println!();
                                            }

                                            debug!(loop_index = self.loop_count, stream_index, kind="StreamedAssistantContent::ToolCall", id = %tool_call.id, name = %tool_call.function.name, "stream item received");
                                            self.save_to_chat_history(Message::Assistant {
                                                id: None,
                                                content: OneOrMany::one(AssistantContent::tool_call(
                                                    &tool_call.id,
                                                    &tool_call.function.name,
                                                    tool_call.function.arguments.clone(),
                                                )),
                                            });

                                        }
                                        StreamedAssistantContent::ToolCallDelta { .. } => {
                                            debug!(
                                                loop_index = self.loop_count,
                                                stream_index,
                                                kind = "StreamedAssistantContent::ToolCallDelta",
                                                "stream item received"
                                            );
                                        }
                                        StreamedAssistantContent::Reasoning(reasoning) => {
                                            debug!(
                                                loop_index = self.loop_count,
                                                stream_index,
                                                kind = "StreamedAssistantContent::Reasoning",
                                                "stream item received"
                                            );
                                            print!("\n{}", "[reasoning] ".cyan());
                                            for r in &reasoning.reasoning {
                                                print!("{}", r.to_string().cyan());
                                            }
                                            self.debug_tx.send(DebugEvent::from(&Message::Assistant {
                                                id: None,
                                                content: OneOrMany::one(AssistantContent::Reasoning(reasoning)),
                                            }));

                                        }
                                        StreamedAssistantContent::ReasoningDelta { .. } => {
                                            debug!(
                                                loop_index = self.loop_count,
                                                stream_index,
                                                kind = "StreamedAssistantContent::ReasoningDelta",
                                                "stream item received"
                                            );
                                        }
                                        StreamedAssistantContent::Final(_) => {
                                            debug!(
                                                loop_index = self.loop_count,
                                                stream_index,
                                                kind = "StreamedAssistantContent::Final",
                                                "stream item received"
                                            );
                                        }
                                    }
                                }
                                Ok(MultiTurnStreamItem::StreamUserItem(user_content)) => match user_content {
                                    StreamedUserContent::ToolResult(tool_result) => {
                                        debug!(loop_index = self.loop_count, stream_index, kind="StreamedUserContent::ToolResult", id = %tool_result.id, "stream item received");
                                        self.save_to_chat_history(Message::User {
                                            content: OneOrMany::one(UserContent::tool_result(
                                                tool_result.id,
                                                tool_result.content,
                                            )),
                                        });
                                    }
                                },
                                Ok(MultiTurnStreamItem::FinalResponse(_final_resp)) => {
                                    debug!(
                                        loop_index = self.loop_count,
                                        stream_index,
                                        kind = "MultiTurnStreamItem::FinalResponse",
                                        "stream item received"
                                    );
                                    println!();
                                }
                                Ok(_) => {
                                    debug!(
                                        loop_index = self.loop_count,
                                        stream_index,
                                        kind = "Ok(_unknown)",
                                        "stream item received"
                                    );
                                }
                                Err(e) => {
                                    // TODO: ugly hack for now. proper error handling will require rig
                                    // exporting agent::prompt_request::streaming::StreamingError
                                    if e.to_string().contains("PromptCancelled") {
                                        if let Some(Message::Assistant { content, .. }) = self.chat_history.last()
                                            && let AssistantContent::ToolCall(_) = content.first()
                                        {
                                            self.chat_history.pop();

                                            self.pending_prompt = self.hitl.take_feedback();
                                            let feedback_text = match &self.pending_prompt {
                                                Some(feedback) => {
                                                    info!(loop_index = self.loop_count, err=%e, feedback, "user cancelled tool call and provided feedback");
                                                    "tool call cancelled; continuing with your feedback"
                                                }
                                                None => {
                                                    info!(loop_index = self.loop_count, err=%e, "user cancelled tool call loop");
                                                    "tool call cancelled"
                                                }
                                            };
                                            println!("{}", feedback_text.red());
                                        }

                                        self.print_newline_before_prompt = false;
                                    } else {
                                        debug!(loop_index = self.loop_count, stream_index, kind="Err", error = %e, "stream item received");
                                        print_error(anyhow::anyhow!(e));
                                    }
                                    break;
                                }
                            }
                            stream_index += 1;
                        },
                        None => break,
                    }

                }
            }
        }

        if !response_text.is_empty() {
            self.save_to_chat_history(Message::assistant(&response_text));
        }

        match serde_json::to_string_pretty(&self.chat_history) {
            Ok(history) => {
                let chat_history_file_path = self
                    .chats_dir
                    .join(format!("loop-{}.json", self.loop_count));
                if let Err(e) = tokio::fs::write(&chat_history_file_path, history).await {
                    error!(loop_index = self.loop_count, err=%e, "one agent loop done; couldn't write chat history to file")
                }
            }
            Err(err) => {
                error!(loop_index = self.loop_count, %err, "one agent loop done; couldn't serialize chat history")
            }
        }

        self.loop_count += 1;
    }

    fn save_to_chat_history(&mut self, message: Message) {
        self.debug_tx.send(DebugEvent::from(&message));
        self.chat_history.push(message);
    }
}

fn print_error(error: anyhow::Error) {
    println!("{}", format!("Error: {:?}", error).red());
}
