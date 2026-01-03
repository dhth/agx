use crate::tools::AgxToolCall;
use std::collections::HashSet;
use std::str::FromStr;

#[derive(Debug, Default)]
pub struct Permissions {
    pub create_file: ApprovalLevel,
    pub edit_file: ApprovalLevel,
    pub run_cmd: ApprovedCmds,
}

impl Permissions {
    pub fn update_for_tool_call(&mut self, tool_call: &AgxToolCall) {
        match tool_call {
            AgxToolCall::CreateFile { .. } => {
                self.create_file = ApprovalLevel::ApprovedForSession;
            }
            AgxToolCall::EditFile { .. } => {
                self.edit_file = ApprovalLevel::ApprovedForSession;
            }
            AgxToolCall::RunCmd { args } => {
                // TODO: handle this error
                if let Ok(cmd_pattern) = CmdPattern::from_str(&args.command) {
                    self.run_cmd.insert(&cmd_pattern);
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct CmdPattern(String);

impl FromStr for CmdPattern {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cmd = match shlex::split(s) {
            Some(c) if !c.is_empty() => c,
            _ => return Err("command is empty"),
        };

        Ok(Self(cmd[0].clone()))
    }
}

#[derive(Debug, Default)]
pub struct ApprovedCmds(HashSet<CmdPattern>);

impl ApprovedCmds {
    pub fn contains(&mut self, pattern: &CmdPattern) -> bool {
        self.0.contains(pattern)
    }

    pub fn insert(&mut self, pattern: &CmdPattern) {
        self.0.insert(pattern.clone());
    }
}

#[derive(Debug, Default, Clone)]
pub enum ApprovalLevel {
    #[default]
    Ask,
    ApprovedForSession,
}

impl ApprovalLevel {
    pub fn is_approved(&self) -> bool {
        matches!(&self, ApprovalLevel::ApprovedForSession)
    }
}
