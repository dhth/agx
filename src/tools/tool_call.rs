use super::{
    CreateFileArgs, CreateFileTool, EditFileArgs, EditFileTool, ReadDirArgs, ReadDirTool,
    ReadFileArgs, ReadFileTool, RunCmdArgs, RunCmdTool,
};
use colored::Colorize;
use rig::message::ToolCall;
use rig::tool::Tool;
use std::io::Write;
use tokio::time::Instant;

#[derive(Debug)]
pub enum AgxToolCall {
    CreateFile { args: CreateFileArgs },
    EditFile { args: EditFileArgs },
    ReadFile { args: ReadFileArgs },
    ReadDir { args: ReadDirArgs },
    RunCmd { args: RunCmdArgs },
}

#[derive(Debug, thiserror::Error)]
pub enum AgxToolCallError {
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    #[error("invalid arguments: {0}")]
    InvalidArgs(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ToolExecutionError {
    #[error("couldn't serialise result: {0}")]
    CouldntSerialiseResult(serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ToolCallDetailsError(String);

impl ToolCallDetailsError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }

    pub fn message(&self) -> &str {
        &self.0
    }
}

impl TryFrom<ToolCall> for AgxToolCall {
    type Error = AgxToolCallError;

    fn try_from(call: ToolCall) -> Result<Self, Self::Error> {
        let name = call.function.name.as_str();
        let args = call.function.arguments;

        match name {
            "create_file" => Ok(AgxToolCall::CreateFile {
                args: serde_json::from_value(args)?,
            }),
            "edit_file" => Ok(AgxToolCall::EditFile {
                args: serde_json::from_value(args)?,
            }),
            "read_file" => Ok(AgxToolCall::ReadFile {
                args: serde_json::from_value(args)?,
            }),
            "read_dir" => Ok(AgxToolCall::ReadDir {
                args: serde_json::from_value(args)?,
            }),
            "run_cmd" => Ok(AgxToolCall::RunCmd {
                args: serde_json::from_value(args)?,
            }),
            _ => Err(AgxToolCallError::UnknownTool(name.to_string())),
        }
    }
}

impl AgxToolCall {
    pub fn repr(&self) -> String {
        match self {
            AgxToolCall::CreateFile { args, .. } => CreateFileTool::repr(args),
            AgxToolCall::EditFile { args, .. } => EditFileTool::repr(args),
            AgxToolCall::ReadFile { args, .. } => ReadFileTool::repr(args),
            AgxToolCall::ReadDir { args, .. } => ReadDirTool::repr(args),
            AgxToolCall::RunCmd { args, .. } => RunCmdTool::repr(args),
        }
    }

    pub async fn details(&self) -> Result<Option<String>, ToolCallDetailsError> {
        match self {
            AgxToolCall::EditFile { args, .. } => EditFileTool::details(args)
                .await
                .map_err(|e| ToolCallDetailsError::new(e.to_string())),
            AgxToolCall::CreateFile { args, .. } => Ok(CreateFileTool::details(args)),
            AgxToolCall::ReadFile { args, .. } => Ok(ReadFileTool::details(args)),
            AgxToolCall::ReadDir { args, .. } => Ok(ReadDirTool::details(args)),
            AgxToolCall::RunCmd { args, .. } => Ok(RunCmdTool::details(args)),
        }
    }

    pub fn needs_confirmation(&self) -> bool {
        matches!(
            self,
            AgxToolCall::EditFile { .. }
                | AgxToolCall::CreateFile { .. }
                | AgxToolCall::RunCmd { .. }
        )
    }

    pub async fn execute(self) -> Result<String, ToolExecutionError> {
        let repr = self.repr();

        match self {
            AgxToolCall::RunCmd { args, .. } => {
                print!("{} ", repr.cyan());
                let _ = std::io::stdout().flush();

                let start = Instant::now();
                let result = RunCmdTool.call(args).await;
                let elapsed_ms = start.elapsed().as_millis();

                match &result {
                    Ok(r) => println!(
                        "{}",
                        format!(
                            "✓ (took {} ms{})",
                            elapsed_ms,
                            match r.status_code {
                                Some(c) if c != 0 => format!("; exit code: {c}"),
                                _ => "".to_string(),
                            }
                        )
                        .green()
                    ),
                    Err(_) => println!("{}", format!("✗ (took {} ms)", elapsed_ms).red()),
                }

                match result {
                    Ok(r) => serde_json::to_string(&r)
                        .map_err(ToolExecutionError::CouldntSerialiseResult),
                    Err(e) => Ok(format!("error: {e}")),
                }
            }

            AgxToolCall::ReadFile { args, .. } => {
                let result = ReadFileTool.call(args).await;

                match &result {
                    Ok(contents) => {
                        println!(
                            "{} {}",
                            repr.cyan(),
                            format!("✓ (read {} bytes)", contents.len()).green()
                        );
                    }
                    Err(_) => {
                        println!("{} {}", repr.cyan(), "✗".red());
                    }
                }

                match result {
                    Ok(r) => serde_json::to_string(&r)
                        .map_err(ToolExecutionError::CouldntSerialiseResult),
                    Err(e) => Ok(format!("error: {e}")),
                }
            }

            AgxToolCall::CreateFile { args, .. } => {
                let result = CreateFileTool.call(args).await;

                match &result {
                    Ok(response) => {
                        println!(
                            "{} {}",
                            repr.cyan(),
                            format!("✓ (wrote {} bytes)", response.num_bytes_written).green()
                        );
                    }
                    Err(_) => {
                        println!("{} {}", repr.cyan(), "✗".red());
                    }
                }

                match result {
                    Ok(r) => serde_json::to_string(&r)
                        .map_err(ToolExecutionError::CouldntSerialiseResult),
                    Err(e) => Ok(format!("error: {e}")),
                }
            }

            AgxToolCall::EditFile { args, .. } => {
                let result = EditFileTool.call(args).await;

                match &result {
                    Ok(response) => {
                        println!(
                            "{} {}",
                            repr.cyan(),
                            format!("✓ (wrote {} bytes)", response.num_bytes_written).green()
                        );
                    }
                    Err(_) => {
                        println!("{} {}", repr.cyan(), "✗".red());
                    }
                }

                match result {
                    Ok(r) => serde_json::to_string(&r)
                        .map_err(ToolExecutionError::CouldntSerialiseResult),
                    Err(e) => Ok(format!("error: {e}")),
                }
            }

            AgxToolCall::ReadDir { args, .. } => {
                let result = ReadDirTool.call(args).await;

                match &result {
                    Ok(entries) => {
                        println!(
                            "{} {}",
                            repr.cyan(),
                            format!("✓ (read {} entries)", entries.len()).green()
                        );
                    }
                    Err(_) => {
                        println!("{} {}", repr.cyan(), "✗".red());
                    }
                }

                match result {
                    Ok(r) => serde_json::to_string(&r)
                        .map_err(ToolExecutionError::CouldntSerialiseResult),
                    Err(e) => Ok(format!("error: {e}")),
                }
            }
        }
    }
}
