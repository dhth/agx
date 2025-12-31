use crate::debug::DebugServer;
use crate::domain::{DebugEvent, DebugEventReceiver, DebugEventSender, Provider};
use crate::env::{get_env_var, get_optional_env_var};
use crate::helpers::{get_project_context, path_to_dirname};
use crate::providers::copilot;
use crate::session::Session;
use crate::tools::{CreateFileTool, EditFileTool, ReadDirTool, ReadFileTool, RunCmdTool};
use anyhow::Context;
use colored::Colorize;
use rig::client::{Client, CompletionClient};
use rig::providers::anthropic::client::AnthropicExt;
use rig::providers::gemini::client::GeminiExt;
use rig::providers::openai::OpenAICompletionsExt;
use rig::providers::openrouter::client::OpenRouterExt;
use rig::providers::{anthropic, gemini, openai, openrouter};
use std::borrow::Cow;
use std::str::FromStr;

const SYSTEM_PROMPT: &str = include_str!("assets/system-prompt.txt");

pub async fn run() -> anyhow::Result<()> {
    let xdg = etcetera::choose_base_strategy().context("couldn't determine your home directory")?;
    crate::telemetry::setup(&xdg).context("couldn't set up logging")?;

    let provider =
        Provider::from_str(get_env_var("PROVIDER")?.as_str()).map_err(|e| anyhow::anyhow!(e))?;
    let api_key = get_env_var("API_KEY")?;
    let model_name = get_env_var("MODEL_NAME")?;
    let base_url = get_optional_env_var("BASE_URL")?;

    let cwd = std::env::current_dir().context("couldnt' determine current working directory")?;
    let agx_log_dir = crate::telemetry::get_log_dir(&xdg);
    let project_log_dir = agx_log_dir.join("projects").join(path_to_dirname(&cwd));

    let system_prompt = match get_project_context().await? {
        Some(p) => Cow::Owned(format!(
            "{}

The following is context specific to this project:

{}",
            SYSTEM_PROMPT, p
        )),
        None => Cow::Borrowed(SYSTEM_PROMPT),
    };

    tokio::fs::create_dir_all(&project_log_dir)
        .await
        .with_context(|| {
            format!(
                "failed to create directory for logs: {:?}",
                &project_log_dir,
            )
        })?;

    let enable_debug_server = get_optional_env_var("AGX_DEBUG_SERVER")?
        .map(|v| v == "1")
        .unwrap_or(false);

    let debug_tx = if enable_debug_server {
        let (tx, _) = tokio::sync::broadcast::channel::<DebugEvent>(64);
        let debug_rx = DebugEventReceiver::new(tx.clone());
        let debug_tx = DebugEventSender::new(tx);

        tokio::spawn(async move {
            let server = DebugServer::new(debug_rx);
            if let Err(e) = server.run().await {
                eprintln!("\n{}", format!("couldn't run debug server: {:?}", e).red());
            }
        });

        Some(debug_tx)
    } else {
        None
    };

    match provider {
        Provider::Gemini => {
            let mut builder = gemini::Client::builder().api_key(api_key);
            if let Some(u) = base_url {
                builder = builder.base_url(u);
            }
            let client: Client<GeminiExt> = builder.build().context("couldn't build client")?;

            let agent = client
                .agent(&model_name)
                .preamble(&system_prompt)
                .tool(CreateFileTool)
                .tool(EditFileTool)
                .tool(ReadDirTool)
                .tool(ReadFileTool)
                .tool(RunCmdTool)
                .build();

            let mut session =
                Session::new(agent, cwd, project_log_dir, provider, &model_name, debug_tx)?;
            session.run().await?;
        }
        Provider::Openrouter => {
            let mut builder = openrouter::Client::builder().api_key(api_key);
            if let Some(u) = base_url {
                builder = builder.base_url(u);
            }
            let client: Client<OpenRouterExt> = builder.build().context("couldn't build client")?;

            let agent = client
                .agent(&model_name)
                .preamble(&system_prompt)
                .tool(CreateFileTool)
                .tool(EditFileTool)
                .tool(ReadDirTool)
                .tool(ReadFileTool)
                .tool(RunCmdTool)
                .build();

            let mut session =
                Session::new(agent, cwd, project_log_dir, provider, &model_name, debug_tx)?;
            session.run().await?;
        }
        Provider::Anthropic => {
            let mut builder = anthropic::Client::builder().api_key(api_key);
            if let Some(u) = base_url {
                builder = builder.base_url(u);
            }
            let client: Client<AnthropicExt> = builder.build().context("couldn't build client")?;

            let agent = client
                .agent(&model_name)
                .preamble(&system_prompt)
                .max_tokens(50000)
                .tool(CreateFileTool)
                .tool(EditFileTool)
                .tool(ReadDirTool)
                .tool(ReadFileTool)
                .tool(RunCmdTool)
                .build();

            let mut session =
                Session::new(agent, cwd, project_log_dir, provider, &model_name, debug_tx)?;
            session.run().await?;
        }
        Provider::GitHubCopilot => {
            let client: Client<OpenAICompletionsExt> = {
                let http_client = reqwest::Client::builder()
                    .default_headers(copilot::get_headers())
                    .build()
                    .context("couldn't build http client for copilot API calls")?;

                let copilot_auth = copilot::get_auth_token(&http_client, &api_key)
                    .await
                    .context("couldn't get a short lived GitHub Copilot token")?;

                let builder = openai::Client::<reqwest::Client>::builder()
                    .base_url(&copilot_auth.endpoints.api)
                    .api_key(&copilot_auth.token)
                    .http_client(http_client);

                builder
                    .build()
                    .context("couldn't build client")?
                    .completions_api() // This is to maintain consistency with the other clients
            };

            let agent = client
                .agent(&model_name)
                .preamble(&system_prompt)
                .tool(CreateFileTool)
                .tool(EditFileTool)
                .tool(ReadDirTool)
                .tool(ReadFileTool)
                .tool(RunCmdTool)
                .build();

            let mut session =
                Session::new(agent, cwd, project_log_dir, provider, &model_name, debug_tx)?;
            session.run().await?;
        }
    }

    Ok(())
}
