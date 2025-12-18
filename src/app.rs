use crate::env::{get_env_var, get_optional_env_var};
use crate::tools::{CreateFile, EditFile, ReadDir, ReadFile, RunCmd};
use anyhow::Context;
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
use std::path::PathBuf;
use tracing::{debug, trace};

const BANNER: &str = include_str!("assets/logo.txt");
const SYSTEM_PROMPT: &str = include_str!("assets/system-prompt.txt");

pub async fn run() -> anyhow::Result<()> {
    let xdg = etcetera::choose_base_strategy().context("couldn't determine your home directory")?;
    crate::telemetry::setup(&xdg).context("couldn't set up logging")?;

    let provider = get_env_var("PROVIDER")?;
    let api_key = get_env_var("API_KEY")?;
    let model_name = get_env_var("MODEL_NAME")?;
    let base_url = get_optional_env_var("BASE_URL")?;

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

            agent_loop(&agent).await?;
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

            agent_loop(&agent).await?;
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

            agent_loop(&agent).await?;
        }
        other => anyhow::bail!(r#"unknown provider specified: "{other}""#),
    }

    Ok(())
}

async fn agent_loop<M>(agent: &Agent<M>) -> anyhow::Result<()>
where
    M: CompletionModel + 'static,
{
    let history_file_path = PathBuf::from(".agx");
    if let Some(parent) = history_file_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory for history: {:?}", parent,))?;
    }

    let mut editor = rustyline::DefaultEditor::new()?;
    let _ = editor.load_history(&history_file_path);

    print!(
        "

{}
",
        BANNER.purple(),
    );

    let mut chat_history = vec![];

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
                debug!(query = q, "user entered query");

                let mut stream = agent
                    .stream_prompt(q)
                    .multi_turn(10)
                    .with_history(chat_history.clone())
                    .await;

                let mut response_text = String::new();

                while let Some(item) = stream.next().await {
                    match item {
                        Ok(rig::agent::MultiTurnStreamItem::StreamAssistantItem(content)) => {
                            match content {
                                StreamedAssistantContent::Text(text) => {
                                    print!("{}", text.text.purple());
                                    response_text.push_str(&text.text);
                                }
                                StreamedAssistantContent::ToolCall(tool_call) => {
                                    println!(
                                        "\n{}",
                                        format!(
                                            "[tool-call] {}({})",
                                            tool_call.function.name, tool_call.function.arguments
                                        )
                                        .yellow()
                                    );

                                    chat_history.push(Message::Assistant {
                                        id: None,
                                        content: OneOrMany::one(AssistantContent::tool_call(
                                            &tool_call.id,
                                            &tool_call.function.name,
                                            tool_call.function.arguments.clone(),
                                        )),
                                    });
                                }
                                StreamedAssistantContent::ToolCallDelta { id: _, delta: _ } => {}
                                StreamedAssistantContent::Reasoning(reasoning) => {
                                    print!("{}", "[reasoning] ".cyan());
                                    for reasoning in reasoning.reasoning {
                                        print!("{}", reasoning.to_string().cyan());
                                    }
                                }
                                _ => {}
                            }
                        }
                        Ok(MultiTurnStreamItem::StreamUserItem(user_content)) => match user_content
                        {
                            StreamedUserContent::ToolResult(tool_result) => {
                                chat_history.push(Message::User {
                                    content: OneOrMany::one(UserContent::tool_result(
                                        tool_result.id,
                                        tool_result.content,
                                    )),
                                });
                            }
                        },
                        Ok(MultiTurnStreamItem::FinalResponse(_final_resp)) => {
                            println!();
                        }
                        Ok(_) => {}
                        Err(e) => {
                            print_error(anyhow::anyhow!(e));
                            break;
                        }
                    }
                }

                if !response_text.is_empty() {
                    chat_history.push(Message::assistant(&response_text));
                }
                trace!(?chat_history, "one agent loop done");
            }
        }
    }

    let _ = editor.save_history(&history_file_path);

    Ok(())
}

fn print_error(error: anyhow::Error) {
    println!("{}", format!("Error: {:?}", error).red());
}
