use crate::helpers::is_path_in_workspace;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use tracing::instrument;

#[derive(Debug, Deserialize)]
pub struct CreateFileArgs {
    pub path: String,
    pub contents: String,
}

impl std::fmt::Display for CreateFileArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "path={}, num_lines={}",
            self.path,
            self.contents.lines().count()
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CreateFileError {
    #[error("invalid input provided: {0}")]
    InvalidInput(String),
    #[error("absolute paths and parent directory traversal ('..') are not allowed")]
    PathNotAllowed,
    #[error("couldn't get metadata for path: {0}")]
    CouldntGetMetadata(std::io::Error),
    #[error("file already exists")]
    AlreadyExists,
    #[error("a directory already exists at this path")]
    IsADir,
    #[error("couldn't create directory: {0}")]
    CouldntCreateDirectory(std::io::Error),
    #[error("couldn't write to file: {0}")]
    CouldntWriteToFile(std::io::Error),
}

#[derive(Deserialize, Serialize)]
pub struct CreateFileTool;

#[derive(Debug, Serialize)]
pub struct CreateFileResponse {
    path: String,
    pub num_bytes_written: usize,
}

impl Tool for CreateFileTool {
    const NAME: &'static str = "create_file";
    type Error = CreateFileError;
    type Args = CreateFileArgs;
    type Output = CreateFileResponse;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Create a new file with contents".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "path of the file"
                    },
                    "contents": {
                        "type": "string",
                        "description": "contents to write"
                    },
                },
                "required": ["path", "contents"],
            }),
        }
    }

    #[instrument(name = "tool-call: create_file", skip(self), err)]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.path.is_empty() {
            // TODO: encode this in the type system
            return Err(CreateFileError::InvalidInput(
                "path cannot be empty".to_string(),
            ));
        }
        let contents = args.contents;

        let path = PathBuf::from(&args.path);
        if !is_path_in_workspace(&path) {
            return Err(CreateFileError::PathNotAllowed);
        }

        match tokio::fs::metadata(&path).await {
            Ok(m) => {
                if m.is_dir() {
                    Err(CreateFileError::IsADir)
                } else {
                    Err(CreateFileError::AlreadyExists)
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(CreateFileError::CouldntGetMetadata(e)),
        }?;

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(CreateFileError::CouldntCreateDirectory)?;
        }

        tokio::fs::write(&path, &contents)
            .await
            .map_err(CreateFileError::CouldntWriteToFile)?;

        Ok(CreateFileResponse {
            path: path.to_string_lossy().to_string(),
            num_bytes_written: contents.len(),
        })
    }
}

impl CreateFileTool {
    pub fn repr(args: &CreateFileArgs) -> String {
        format!("create_file: {}", args.path)
    }

    pub fn details(args: &CreateFileArgs) -> Option<String> {
        Some(args.contents.clone())
    }
}
