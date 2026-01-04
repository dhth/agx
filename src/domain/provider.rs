use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug)]
pub enum Provider {
    Anthropic,
    Gemini,
    GitHubCopilot,
    OpenAI,
    Openrouter,
}

impl FromStr for Provider {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "anthropic" => Ok(Self::Anthropic),
            "gemini" => Ok(Self::Gemini),
            "github-copilot" => Ok(Self::GitHubCopilot),
            "openai" => Ok(Self::OpenAI),
            "openrouter" => Ok(Self::Openrouter),
            _ => Err(
                "invalid provider; allowed values: [anthropic, gemini, github-copilot, openai, openrouter]",
            ),
        }
    }
}

impl Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Provider::Anthropic => "anthropic",
            Provider::Gemini => "gemini",
            Provider::GitHubCopilot => "github-copilot",
            Provider::OpenAI => "openai",
            Provider::Openrouter => "openrouter",
        };

        write!(f, "{}", name)
    }
}
