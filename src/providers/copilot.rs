use anyhow::Context;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;

const USER_AGENT: &str = "GitHubCopilotChat/0.32.4";
const EDITOR_VERSION: &str = "vscode/1.105.1";
const EDITOR_PLUGIN_VERSION: &str = "copilot-chat/0.32.4";
const INTEGRATION_ID: &str = "vscode-chat";

pub fn get_headers() -> HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", HeaderValue::from_static(USER_AGENT));
    headers.insert("Editor-Version", HeaderValue::from_static(EDITOR_VERSION));
    headers.insert(
        "Editor-Plugin-Version",
        HeaderValue::from_static(EDITOR_PLUGIN_VERSION),
    );
    headers.insert(
        "Copilot-Integration-Id",
        HeaderValue::from_static(INTEGRATION_ID),
    );

    headers
}

#[derive(Debug, Deserialize)]
pub struct CopilotEndpoints {
    pub api: String,
}

#[derive(Debug, Deserialize)]
pub struct CopilotAuth {
    pub endpoints: CopilotEndpoints,
    pub token: String,
    #[allow(unused)]
    pub expires_at: u64, // TODO: make use of this to refresh token
}

pub async fn get_auth_token(client: &Client, oauth_token: &str) -> anyhow::Result<CopilotAuth> {
    let response = client
        .get("https://api.github.com/copilot_internal/v2/token")
        .bearer_auth(oauth_token)
        .send()
        .await
        .context("couldn't send request")?;

    let token: CopilotAuth = response
        .json()
        .await
        .context("couldn't deserialize response")?;

    Ok(token)
}
