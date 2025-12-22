use super::{CreateFileArgs, CreateFileTool, EditFileTool, ReadDirArgs, ReadDirTool, RunCmdTool};
use crate::tools::RunCmdArgs;
use rig::tool::Tool;

pub fn get_tool_repr(tool_name: &str, args: &str) -> String {
    let args_to_show = match tool_name {
        CreateFileTool::NAME => match serde_json::from_str::<CreateFileArgs>(args) {
            Ok(a) => Some(a.to_string()),
            Err(_) => None,
        },
        EditFileTool::NAME => match serde_json::from_str::<ReadDirArgs>(args) {
            Ok(a) => Some(a.to_string()),
            Err(_) => None,
        },
        ReadDirTool::NAME => match serde_json::from_str::<ReadDirArgs>(args) {
            Ok(a) => Some(a.to_string()),
            Err(_) => None,
        },
        RunCmdTool::NAME => match serde_json::from_str::<RunCmdArgs>(args) {
            Ok(a) => Some(a.to_string()),
            Err(_) => None,
        },
        _ => None,
    };

    format!(
        "{}{}",
        tool_name,
        args_to_show
            .map(|a| format!(" ({})", a))
            .unwrap_or_default()
    )
}
