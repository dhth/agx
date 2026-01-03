use crate::domain::{ApprovedCmds, CmdPattern};
use crate::tools::AgxToolCall;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Default)]
pub struct Approvals {
    pub fs_changes: bool,
    pub approved_commands: ApprovedCmds,
}

impl Approvals {
    pub fn is_tool_call_approved(&self, tool_call: &AgxToolCall) -> bool {
        match tool_call {
            AgxToolCall::CreateFile { .. } | AgxToolCall::EditFile { .. } => self.fs_changes,
            AgxToolCall::RunCmd { args } => self.approved_commands.is_approved(&args.command),
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
                    self.approved_commands.insert(&cmd_pattern);
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
            self.fs_changes, self.approved_commands
        )
    }
}
