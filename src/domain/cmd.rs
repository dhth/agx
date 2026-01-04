use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CmdPattern {
    binary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_arg: Option<String>,
}

impl FromStr for CmdPattern {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cmd = match shlex::split(s) {
            Some(c) if !c.is_empty() => c,
            _ => return Err("command is empty"),
        };

        Ok(Self {
            binary: cmd[0].clone(),
            first_arg: cmd.get(1).cloned(),
        })
    }
}

impl Display for CmdPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.first_arg {
            Some(a) => write!(f, "{} {} .*", self.binary, a),
            None => write!(f, "{} .*", self.binary),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ApprovedCmds(HashSet<CmdPattern>);

impl ApprovedCmds {
    pub fn is_approved(&self, cmd: &str) -> bool {
        CmdPattern::from_str(cmd)
            .map(|p| self.0.contains(&p))
            .unwrap_or(false)
    }

    pub fn insert(&mut self, pattern: &CmdPattern) {
        self.0.insert(pattern.clone());
    }
}

impl Display for ApprovedCmds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return write!(f, "none");
        }

        let lines = self
            .0
            .iter()
            .map(|cmd| cmd.to_string())
            .collect::<Vec<_>>()
            .join("\n  - ");

        write!(f, "\n  - {}", lines)
    }
}
