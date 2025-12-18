use crate::env::{get_env_var, get_optional_env_var};
use crate::helpers::path_to_dirname;
use crate::tools::{CreateFile, EditFile, ReadDir, ReadFile, RunCmd};
use anyhow::Context;
use chrono::Local;
use colored::Colorize;
use futures::StreamExt;
use rig::OneOrMany;
use rig::agent::{Agent, MultiTurnStreamItem};
use rig::client::{Client, CompletionClient};
use rig::completion::CompletionModel;
use rig::message::{AssistantContent, Message, UserContent};
use rig::providers::anthropic::client::AnthropicExt;
use rig::providers::gemini::client::GeminiExt;
use rig::providers::openrouter::client::OpenRouterExt;
use rig::providers::{anthropic, gemini, openrouter};
use rig::streaming::{StreamedAssistantContent, StreamedUserContent, StreamingPrompt};
use std::path::Path;
use tracing::{debug, error};

const BANNER: &str = include_str!("assets/logo.txt");
const SYSTEM_PROMPT: &str = include_str!("assets/system-prompt.txt");

pub async fn run() -> anyhow::Result<()> {
    let xdg = etcetera::choose_base_strategy().context("couldn't determine your home directory")?;
    crate::telemetry::setup(&xdg).context("couldn't set up logging")?;

    let provider = get_env_var("PROVIDER")?;
    let api_key = get_env_var("API_KEY")?;
    let model_name = get_env_var("MODEL_NAME")?;
    let base_url = get_optional_env_var("BASE_URL")?;

    let cwd = std::env::current_dir().context("couldnt' determine current working directory")?;
    let agx_log_dir = crate::telemetry::get_log_dir(&xdg);
    let project_log_dir = agx_log_dir.join("projects").join(path_to_dirname(&cwd));
    tokio::fs::create_dir_all(&project_log_dir)
        .await
        .with_context(|| {
            format!(
                "failed to create directory for logs: {:?}",
                &project_log_dir,
            )
        })?;

    match provider.as_str() {
        "gemini" => {
            let mut builder = gemini::Client::builder().api_key(api_key);
            if let Some(u) = base_url {
                builder = builder.base_url(u);
            }
            let client: Client<GeminiExt> = builder.build().context("couldn't build client")?;

            let agent = client
                .agent(model_name)
                .preamble(SYSTEM_PROMPT)
                .tool(CreateFile)
                .tool(EditFile)
                .tool(ReadDir)
                .tool(ReadFile)
                .tool(RunCmd)
                .build();

            agent_loop(&agent, &project_log_dir).await?;
        }
        "openrouter" => {
            let mut builder = openrouter::Client::builder().api_key(api_key);
            if let Some(u) = base_url {
                builder = builder.base_url(u);
            }
            let client: Client<OpenRouterExt> = builder.build().context("couldn't build client")?;

            let agent = client
                .agent(model_name)
                .preamble(SYSTEM_PROMPT)
                .tool(CreateFile)
                .tool(EditFile)
                .tool(ReadDir)
                .tool(ReadFile)
                .tool(RunCmd)
                .build();

            agent_loop(&agent, &project_log_dir).await?;
        }
        "anthropic" => {
            let mut builder = anthropic::Client::builder().api_key(api_key);
            if let Some(u) = base_url {
                builder = builder.base_url(u);
            }
            let client: Client<AnthropicExt> = builder.build().context("couldn't build client")?;

            let agent = client
                .agent(model_name)
                .preamble(SYSTEM_PROMPT)
                .max_tokens(50000)
                .tool(CreateFile)
                .tool(EditFile)
                .tool(ReadDir)
                .tool(ReadFile)
                .tool(RunCmd)
                .build();

            agent_loop(&agent, &project_log_dir).await?;
        }
        other => anyhow::bail!(r#"unknown provider specified: "{other}""#),
    }

    Ok(())
}

async fn agent_loop<M, P>(agent: &Agent<M>, project_log_dir: P) -> anyhow::Result<()>
where
    M: CompletionModel + 'static,
    P: AsRef<Path>,
{
    let now = Local::now();
    let chats_dir = project_log_dir
        .as_ref()
        .join("chats")
        .join(now.format("%Y-%m-%d-%H-%M-%S").to_string());
    tokio::fs::create_dir_all(&chats_dir)
        .await
        .with_context(|| format!("failed to create directory for chats: {:?}", &chats_dir,))?;
    let history_file_path = project_log_dir.as_ref().join("history.txt");

    let mut editor = rustyline::DefaultEditor::new()?;
    let _ = editor.load_history(&history_file_path);

    print!(
        "

{}
",
        BANNER.purple(),
    );

    let mut chat_history = vec![];

    let mut loop_index = 0;
    loop {
        let query = editor.readline("\n>>> ").context("couldn't read input")?;

        match query.trim() {
            "" => {}
            "bye" | "exit" | "quit" | ":q" => {
                break;
            }
            q => {
                _ = editor.add_history_entry(q);

                chat_history.push(Message::user(q));
                debug!(loop_index, query = q, "user entered query");

                let mut stream = agent
                    .stream_prompt(q)
                    .multi_turn(10)
                    .with_history(chat_history.clone())
                    .await;

                let mut response_text = String::new();

                let mut stream_index = 0;
                while let Some(item) = stream.next().await {
                    stream_index += 1;
                    match item {
                        Ok(rig::agent::MultiTurnStreamItem::StreamAssistantItem(content)) => {
                            match content {
                                StreamedAssistantContent::Text(text) => {
                                    debug!(loop_index, stream_index, kind="StreamedAssistantContent::Text", text = %text.text, "stream item received");
                                    print!("{}", text.text.purple());
                                    response_text.push_str(&text.text);
                                }
                                StreamedAssistantContent::ToolCall(tool_call) => {
                                    debug!(loop_index, stream_index, kind="StreamedAssistantContent::ToolCall", id = %tool_call.id, name = %tool_call.function.name, "stream item received");
                                    println!(
                                        "\n{}",
                                        format!(
                                            "[tool-call] {}({})",
                                            tool_call.function.name, tool_call.function.arguments
                                        )
                                        .cyan()
                                    );

                                    if !response_text.is_empty() {
                                        chat_history.push(Message::assistant(&response_text));
                                        response_text.clear();
                                    }

                                    chat_history.push(Message::Assistant {
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
                                        loop_index,
                                        stream_index,
                                        kind = "StreamedAssistantContent::ToolCallDelta",
                                        "stream item received"
                                    );
                                }
                                StreamedAssistantContent::Reasoning(reasoning) => {
                                    debug!(
                                        loop_index,
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
                                        loop_index,
                                        stream_index,
                                        kind = "StreamedAssistantContent::ReasoningDelta",
                                        "stream item received"
                                    );
                                }
                                StreamedAssistantContent::Final(_) => {
                                    debug!(
                                        loop_index,
                                        stream_index,
                                        kind = "StreamedAssistantContent::Final",
                                        "stream item received"
                                    );
                                }
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamUserItem(user_content)) => match user_content
                        {
                            StreamedUserContent::ToolResult(tool_result) => {
                                debug!(loop_index, stream_index, kind="StreamedUserContent::ToolResult", id = %tool_result.id, "stream item received");
                                chat_history.push(Message::User {
                                    content: OneOrMany::one(UserContent::tool_result(
                                        tool_result.id,
                                        tool_result.content,
                                    )),
                                });
                            }
                        },
                        Ok(MultiTurnStreamItem::FinalResponse(_final_resp)) => {
                            debug!(
                                loop_index,
                                stream_index,
                                kind = "MultiTurnStreamItem::FinalResponse",
                                "stream item received"
                            );
                            println!();
                        }
                        Ok(_) => {
                            debug!(
                                loop_index,
                                stream_index,
                                kind = "Ok(_unknown)",
                                "stream item received"
                            );
                        }
                        Err(e) => {
                            debug!(loop_index, stream_index, kind="Err", error = %e, "stream item received");
                            print_error(anyhow::anyhow!(e));
                            break;
                        }
                    }
                }

                if !response_text.is_empty() {
                    chat_history.push(Message::assistant(&response_text));
                }

                match serde_json::to_string_pretty(&chat_history) {
                    Ok(history) => {
                        let chat_history_file_path =
                            chats_dir.join(format!("loop-{}.json", loop_index));
                        if let Err(e) = tokio::fs::write(&chat_history_file_path, history).await {
                            error!(loop_index, err=%e, "one agent loop done; couldn't write chat history to file")
                        }
                    }
                    Err(err) => {
                        error!(loop_index, %err, "one agent loop done; couldn't serialize chat history")
                    }
                }
                loop_index += 1;
            }
        }
    }

    let _ = editor.save_history(&history_file_path);

    Ok(())
}

fn print_error(error: anyhow::Error) {
    println!("{}", format!("Error: {:?}", error).red());
}
