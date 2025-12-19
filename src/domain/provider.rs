use std::fmt::Display;
use std::str::FromStr;

pub enum Provider {
    Gemini,
    Openrouter,
    Anthropic,
}

impl FromStr for Provider {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gemini" => Ok(Self::Gemini),
            "openrouter" => Ok(Self::Openrouter),
            "anthropic" => Ok(Self::Anthropic),
            _ => Err("invalid provider; allowed values: [gemini, openrouter, anthropic]"),
        }
    }
}

impl Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Provider::Gemini => "gemini",
            Provider::Openrouter => "openrouter",
            Provider::Anthropic => "anthropic",
        };

        write!(f, "{}", name)
    }
}
