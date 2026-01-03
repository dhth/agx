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
            first_arg: if cmd.len() > 1 {
                Some(cmd[1].clone())
            } else {
                None
            },
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

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ApprovedCmds(HashSet<CmdPattern>);

impl ApprovedCmds {
    pub fn is_approved(&self, cmd: &str) -> bool {
        if let Ok(cmd_pattern) = CmdPattern::from_str(cmd)
            && self.0.contains(&cmd_pattern)
        {
            true
        } else {
            false
        }
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
