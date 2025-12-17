use crate::env::{get_env_var, get_optional_env_var};
use crate::tools::{CreateFile, EditFile, ReadDir, ReadFile, RunCmd};
use anyhow::Context;
use colored::Colorize;
use rig::agent::Agent;
use rig::client::{Client, CompletionClient};
use rig::completion::{CompletionModel, Prompt};
use rig::message::{AssistantContent, Message};
use rig::providers::anthropic::client::AnthropicExt;
use rig::providers::gemini::client::GeminiExt;
use rig::providers::openrouter::client::OpenRouterExt;
use rig::providers::{anthropic, gemini, openrouter};
use std::collections::HashSet;
use std::path::PathBuf;

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
    M: CompletionModel,
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
    let mut seen_messages_by_id = HashSet::new();
    let mut seen_messages_by_content = HashSet::new();

    loop {
        let query = editor.readline("\n>>> ").context("couldn't read input")?;

        match query.trim() {
            "" => {}
            "bye" | "exit" | "quit" | ":q" => {
                break;
            }
            q => {
                _ = editor.add_history_entry(q);
                if let Err(err) = agent
                    .prompt(q)
                    .with_history(&mut chat_history)
                    .multi_turn(10)
                    .await
                {
                    print_error(anyhow::anyhow!(err));
                    continue;
                }

                for message in chat_history.iter() {
                    if let Message::Assistant { id, content } = message {
                        if let Some(msg_id) = id {
                            if seen_messages_by_id.contains(msg_id) {
                                continue;
                            }
                            seen_messages_by_id.insert(msg_id.to_string());
                        }

                        if let AssistantContent::Text(text) = content.first() {
                            let txt = text.text;
                            if seen_messages_by_content.contains(&txt) {
                                continue;
                            }

                            println!("{}", txt.purple());
                            seen_messages_by_content.insert(txt);
                        }

                        for cont in content.rest() {
                            if let AssistantContent::Text(text) = cont {
                                println!("{}", text.text.purple())
                            }
                        }
                    }
                }
            }
        }
    }

    let _ = editor.save_history(&history_file_path);

    Ok(())
}

fn print_error(error: anyhow::Error) {
    println!("{}", format!("Error: {:?}", error).red());
}
