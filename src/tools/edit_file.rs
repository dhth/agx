use crate::helpers::{Diff, is_path_in_workspace};
use colored::Colorize;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use tracing::{Level, instrument};

#[derive(Debug, Deserialize)]
pub struct EditFileArgs {
    pub path: String,
    pub old_str: String,
    pub new_str: String,
}

impl std::fmt::Display for EditFileArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "path={}", self.path,)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EditFileError {
    #[error("invalid input provided: {0}")]
    InvalidInput(String),
    #[error("absolute paths and parent directory traversal ('..') are not allowed")]
    PathNotAllowed,
    #[error("old string and new string are the same")]
    NoChangesRequested,
    #[error("couldn't get metadata for file: {0}")]
    CouldntGetMetadata(std::io::Error),
    #[error("provided path is not a file")]
    NotAFile,
    #[error("file doesn't exist")]
    FileDoesntExist,
    #[error("couldn't read file: {0}")]
    CouldntReadFile(std::io::Error),
    #[error("couldn't write to file: {0}")]
    CouldntWriteToFile(std::io::Error),
    #[error("nothing will change in the file")]
    NothingWillChange,
}

#[derive(Deserialize, Serialize)]
pub struct EditFileTool;

#[derive(Debug)]
struct EditFileOutputInner {
    path: String,
    num_bytes_written: usize,
    old_contents: String,
    new_contents: String,
}

#[derive(Debug, Serialize)]
pub struct EditFileResponse {
    path: String,
    num_bytes_written: usize,
}

impl From<EditFileOutputInner> for EditFileResponse {
    fn from(value: EditFileOutputInner) -> Self {
        Self {
            path: value.path,
            num_bytes_written: value.num_bytes_written,
        }
    }
}

impl Tool for EditFileTool {
    const NAME: &'static str = "edit_file";
    type Error = EditFileError;
    type Args = EditFileArgs;
    type Output = EditFileResponse;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Edit an already existing file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "path of the file to edit"
                    },
                    "old_str": {
                        "type": "string",
                        "description": "Replace all occurrences of this string with new_str"
                    },
                    "new_str": {
                        "type": "string",
                        "description": "string to replace with"
                    },
                },
                "required": ["path", "old_str", "new_str"],
            }),
        }
    }

    #[instrument(level = Level::TRACE, name = "tool-call: edit_file", ret, err(level = Level::ERROR), skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let log = format!("edited file ({})", args.path).cyan();
        let result = self.call_inner(args).await;
        let status = if result.is_ok() { "✓" } else { "❌" };
        println!("{} {}", log, status,);

        if let Ok(output) = &result
            && let Some(diff) = Diff::new(&output.old_contents, &output.new_contents)
        {
            println!("{}", diff);
        }

        result.map(|o| o.into())
    }
}

impl EditFileTool {
    async fn call_inner(&self, args: EditFileArgs) -> Result<EditFileOutputInner, EditFileError> {
        if args.path.is_empty() {
            // TODO: encode this in the type system
            return Err(EditFileError::InvalidInput(
                "path cannot be empty".to_string(),
            ));
        }

        if args.old_str.is_empty() {
            // TODO: encode this in the type system
            return Err(EditFileError::InvalidInput(
                "old_str cannot be empty".to_string(),
            ));
        }

        if args.old_str == args.new_str {
            return Err(EditFileError::NoChangesRequested);
        }

        let old_str = args.old_str;
        let new_str = args.new_str;

        let path = PathBuf::from(&args.path);
        if !is_path_in_workspace(&path) {
            return Err(EditFileError::PathNotAllowed);
        }

        let metadata = tokio::fs::metadata(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                EditFileError::FileDoesntExist
            } else {
                EditFileError::CouldntGetMetadata(e)
            }
        })?;

        if !metadata.is_file() {
            return Err(EditFileError::NotAFile);
        }

        let old_contents = tokio::fs::read_to_string(&path)
            .await
            .map_err(EditFileError::CouldntReadFile)?;
        let new_contents = old_contents.replace(&old_str, &new_str);

        if old_contents == new_contents {
            return Err(EditFileError::NothingWillChange);
        }

        tokio::fs::write(&path, &new_contents)
            .await
            .map_err(EditFileError::CouldntWriteToFile)?;

        Ok(EditFileOutputInner {
            path: path.to_string_lossy().to_string(),
            num_bytes_written: new_contents.len(),
            old_contents,
            new_contents,
        })
    }
}
