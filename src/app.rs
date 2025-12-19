use crate::domain::Provider;
use crate::env::{get_env_var, get_optional_env_var};
use crate::helpers::path_to_dirname;
use crate::providers::copilot;
use crate::session::Session;
use crate::tools::{CreateFile, EditFile, ReadDir, ReadFile, RunCmd};
use anyhow::Context;
use rig::client::{Client, CompletionClient};
use rig::providers::anthropic::client::AnthropicExt;
use rig::providers::gemini::client::GeminiExt;
use rig::providers::openai::OpenAICompletionsExt;
use rig::providers::openrouter::client::OpenRouterExt;
use rig::providers::{anthropic, gemini, openai, openrouter};
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
    tokio::fs::create_dir_all(&project_log_dir)
        .await
        .with_context(|| {
            format!(
                "failed to create directory for logs: {:?}",
                &project_log_dir,
            )
        })?;

    match provider {
        Provider::Gemini => {
            let mut builder = gemini::Client::builder().api_key(api_key);
            if let Some(u) = base_url {
                builder = builder.base_url(u);
            }
            let client: Client<GeminiExt> = builder.build().context("couldn't build client")?;

            let agent = client
                .agent(&model_name)
                .preamble(SYSTEM_PROMPT)
                .tool(CreateFile)
                .tool(EditFile)
                .tool(ReadDir)
                .tool(ReadFile)
                .tool(RunCmd)
                .build();

            let mut session = Session::new(agent, project_log_dir, provider, &model_name);
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
                .preamble(SYSTEM_PROMPT)
                .tool(CreateFile)
                .tool(EditFile)
                .tool(ReadDir)
                .tool(ReadFile)
                .tool(RunCmd)
                .build();

            let mut session = Session::new(agent, project_log_dir, provider, &model_name);
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
                .preamble(SYSTEM_PROMPT)
                .max_tokens(50000)
                .tool(CreateFile)
                .tool(EditFile)
                .tool(ReadDir)
                .tool(ReadFile)
                .tool(RunCmd)
                .build();

            let mut session = Session::new(agent, project_log_dir, provider, &model_name);
            session.run().await?;
        }
        Provider::GitHubCopilot => {
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

            let client: Client<OpenAICompletionsExt> = builder
                .build()
                .context("couldn't build client")?
                .completions_api(); // This is to maintain consistency with the other clients

            let agent = client
                .agent(&model_name)
                .preamble(SYSTEM_PROMPT)
                .max_tokens(50000)
                .tool(CreateFile)
                .tool(EditFile)
                .tool(ReadDir)
                .tool(ReadFile)
                .tool(RunCmd)
                .build();

            let mut session = Session::new(agent, project_log_dir, provider, &model_name);
            session.run().await?;
        }
    }

    Ok(())
}
