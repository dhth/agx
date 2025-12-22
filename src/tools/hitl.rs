use super::{CreateFileTool, EditFileTool, RunCmdTool};
use rig::tool::Tool;

pub fn needs_confirmation(tool_name: &str) -> bool {
    matches!(
        tool_name,
        CreateFileTool::NAME | EditFileTool::NAME | RunCmdTool::NAME
    )
}
