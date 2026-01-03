use crate::tools::AgxToolCall;
use std::collections::HashSet;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Default)]
pub struct Approvals {
    pub fs_changes: bool,
    pub approved_cmds: ApprovedCmds,
}

impl Approvals {
    pub fn is_tool_call_approved(&self, tool_call: &AgxToolCall) -> bool {
        match tool_call {
            AgxToolCall::CreateFile { .. } | AgxToolCall::EditFile { .. } => self.fs_changes,
            AgxToolCall::RunCmd { args } => self.approved_cmds.is_approved(&args.command),
            _ => true,
        }
    }

    pub fn save_approval(&mut self, tool_call: &AgxToolCall) -> Option<String> {
        match tool_call {
            AgxToolCall::CreateFile { .. } | AgxToolCall::EditFile { .. } => {
                self.fs_changes = true;
                Some(
                    "will not ask for confirmation for creating/editing files from now on"
                        .to_string(),
                )
            }
            AgxToolCall::RunCmd { args } => {
                if let Ok(cmd_pattern) = CmdPattern::from_str(&args.command) {
                    self.approved_cmds.insert(&cmd_pattern);
                    Some(format!(
                        r#"will not ask for confirmation for running "{cmd_pattern}" commands from now on"#,
                    ))
                } else {
                    // TODO: this error shouldn't happen this deep in the call stack
                    None
                }
            }
            _ => None,
        }
    }
}

impl Display for Approvals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"approvals:
- create/edit files: {}
- approved commands: {}
"#,
            self.fs_changes, self.approved_cmds
        )
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct CmdPattern {
    binary: String,
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

#[derive(Debug, Default)]
pub struct ApprovedCmds(HashSet<CmdPattern>);

impl ApprovedCmds {
    pub fn is_approved(&self, cmd: &str) -> bool {
        if let Ok(cmd_pattern) = CmdPattern::from_str(cmd)
            && self.0.contains(&cmd_pattern)
        {
            true
        } else {
            // TODO: this error shouldn't happen this deep in the call stack
            false
        }
    }

    fn insert(&mut self, pattern: &CmdPattern) {
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
