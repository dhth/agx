use crate::domain::Provider;
use anyhow::Context;
use chrono::Local;
use colored::Colorize;
use futures::StreamExt;
use rig::OneOrMany;
use rig::agent::{Agent, MultiTurnStreamItem};
use rig::completion::CompletionModel;
use rig::message::{AssistantContent, Message, UserContent};
use rig::streaming::{StreamedAssistantContent, StreamedUserContent, StreamingPrompt};
use std::path::PathBuf;
use tracing::{debug, error};

const BANNER: &str = include_str!("assets/logo.txt");

pub struct Session<M>
where
    M: CompletionModel + 'static,
{
    agent: Agent<M>,
    project_log_dir: PathBuf,
    provider: Provider,
    model_name: String,
    tokens_used: u64,
    loop_count: usize,
    chat_history: Vec<Message>,
}

impl<M> Session<M>
where
    M: CompletionModel + 'static,
{
    pub fn new(
        agent: Agent<M>,
        project_log_dir: PathBuf,
        provider: Provider,
        model_name: impl Into<String>,
    ) -> Self {
        Self {
            agent,
            project_log_dir,
            provider,
            model_name: model_name.into(),
            tokens_used: 0,
            loop_count: 0,
            chat_history: Vec::new(),
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let now = Local::now();
        let chats_dir = self
            .project_log_dir
            .join("chats")
            .join(now.format("%Y-%m-%d-%H-%M-%S").to_string());
        tokio::fs::create_dir_all(&chats_dir)
            .await
            .with_context(|| format!("failed to create directory for chats: {:?}", &chats_dir,))?;
        let history_file_path = self.project_log_dir.join("history.txt");

        let mut editor = rustyline::DefaultEditor::new()?;
        let _ = editor.load_history(&history_file_path);

        print!(
            "
{}
",
            BANNER.purple(),
        );

        loop {
            println!("\n{}", self.get_info().yellow());
            let query = editor.readline(">>> ").context("couldn't read input")?;

            match query.trim() {
                "" => {}
                "bye" | "exit" | "quit" | ":q" => {
                    break;
                }
                q => {
                    _ = editor.add_history_entry(q);

                    self.chat_history.push(Message::user(q));
                    debug!(
                        loop_index = self.loop_count,
                        query = q,
                        "user entered query"
                    );

                    let mut stream = self
                        .agent
                        .stream_prompt(q)
                        .multi_turn(10)
                        .with_history(self.chat_history.clone())
                        .await;

                    let mut response_text = String::new();

                    let mut stream_index = 0;
                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(rig::agent::MultiTurnStreamItem::StreamAssistantItem(content)) => {
                                match content {
                                    StreamedAssistantContent::Text(text) => {
                                        debug!(loop_index = self.loop_count, stream_index, kind="StreamedAssistantContent::Text", text = %text.text, "stream item received");
                                        print!("{}", text.text.purple());
                                        response_text.push_str(&text.text);
                                    }
                                    StreamedAssistantContent::ToolCall(tool_call) => {
                                        debug!(loop_index = self.loop_count, stream_index, kind="StreamedAssistantContent::ToolCall", id = %tool_call.id, name = %tool_call.function.name, "stream item received");
                                        println!(
                                            "\n{}",
                                            format!(
                                                "[tool-call] {}({})",
                                                tool_call.function.name,
                                                tool_call.function.arguments
                                            )
                                            .cyan()
                                        );

                                        if !response_text.is_empty() {
                                            self.chat_history
                                                .push(Message::assistant(&response_text));
                                            response_text.clear();
                                        }

                                        self.chat_history.push(Message::Assistant {
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
                                        for reasoning in reasoning.reasoning {
                                            print!("{}", reasoning.to_string().cyan());
                                        }
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
                            Ok(MultiTurnStreamItem::StreamUserItem(user_content)) => {
                                match user_content {
                                    StreamedUserContent::ToolResult(tool_result) => {
                                        debug!(loop_index = self.loop_count, stream_index, kind="StreamedUserContent::ToolResult", id = %tool_result.id, "stream item received");
                                        self.chat_history.push(Message::User {
                                            content: OneOrMany::one(UserContent::tool_result(
                                                tool_result.id,
                                                tool_result.content,
                                            )),
                                        });
                                    }
                                }
                            }
                            Ok(MultiTurnStreamItem::FinalResponse(final_resp)) => {
                                self.tokens_used += final_resp.usage().total_tokens;
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
                                debug!(loop_index = self.loop_count, stream_index, kind="Err", error = %e, "stream item received");
                                print_error(anyhow::anyhow!(e));
                                break;
                            }
                        }
                        stream_index += 1;
                    }

                    if !response_text.is_empty() {
                        self.chat_history.push(Message::assistant(&response_text));
                    }

                    match serde_json::to_string_pretty(&self.chat_history) {
                        Ok(history) => {
                            let chat_history_file_path =
                                chats_dir.join(format!("loop-{}.json", self.loop_count));
                            if let Err(e) = tokio::fs::write(&chat_history_file_path, history).await
                            {
                                error!(loop_index = self.loop_count, err=%e, "one agent loop done; couldn't write chat history to file")
                            }
                        }
                        Err(err) => {
                            error!(loop_index = self.loop_count, %err, "one agent loop done; couldn't serialize chat history")
                        }
                    }
                    self.loop_count += 1;
                }
            }
        }

        let _ = editor.save_history(&history_file_path);

        Ok(())
    }

    fn get_info(&self) -> String {
        if self.tokens_used == 0 {
            format!("[{}/{}]", &self.provider, &self.model_name)
        } else {
            format!(
                "[{}/{}]  {} tokens used",
                &self.provider, &self.model_name, self.tokens_used
            )
        }
    }
}

fn print_error(error: anyhow::Error) {
    println!("{}", format!("Error: {:?}", error).red());
}
