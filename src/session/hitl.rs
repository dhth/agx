use crate::tools::AgxToolCall;
use std::collections::HashSet;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Default)]
pub struct Approvals {
    pub create_file: ApprovalLevel,
    pub edit_file: ApprovalLevel,
    pub approved_cmds: ApprovedCmds,
}

impl Approvals {
    pub fn save_approval_for_session(&mut self, tool_call: &AgxToolCall) -> Option<String> {
        match tool_call {
            AgxToolCall::CreateFile { .. } => {
                self.create_file = ApprovalLevel::ApprovedForSession;
                Some("will not ask for confirmation for creating files from now on".to_string())
            }
            AgxToolCall::EditFile { .. } => {
                self.edit_file = ApprovalLevel::ApprovedForSession;
                Some("will not ask for confirmation for editing files from now on".to_string())
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
- create files: {}
- edit files: {}
- approved commands: {}
"#,
            self.create_file, self.edit_file, self.approved_cmds
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
    pub fn is_approved(&mut self, cmd: &str) -> bool {
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

        write!(f, "{}", lines)
    }
}

#[derive(Debug, Default, Clone)]
pub enum ApprovalLevel {
    #[default]
    Ask,
    ApprovedForSession,
}

impl Display for ApprovalLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            ApprovalLevel::Ask => "ask",
            ApprovalLevel::ApprovedForSession => "approved for session",
        };

        write!(f, "{}", value)
    }
}

impl ApprovalLevel {
    pub fn is_approved(&self) -> bool {
        matches!(&self, ApprovalLevel::ApprovedForSession)
    }
}
