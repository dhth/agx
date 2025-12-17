use colored::Colorize;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use tracing::{Level, instrument};

#[derive(Debug, Deserialize)]
pub struct EditFileArgs {
    path: String,
    old_str: Option<String>,
    new_str: String,
}

#[derive(Debug, thiserror::Error)]
pub enum EditFileError {
    #[error("old string and new string are the same")]
    NoChangesRequested,
    #[error("couldn't get metadata for file: {0}")]
    CouldntGetMetadata(#[from] std::io::Error),
    #[error("provided path is not a file")]
    NotAFile,
    #[error("couldn't create directory for file: {0}")]
    CouldntCreateDirectory(std::io::Error),
    #[error("couldn't read file: {0}")]
    CouldntReadFile(std::io::Error),
    #[error("couldn't write to file: {0}")]
    CouldntWriteToFile(std::io::Error),
    #[error("nothing changed in the file")]
    NothingChanged,
    #[error("{0}")]
    InvalidExpectation(String),
}

#[derive(Deserialize, Serialize)]
pub struct EditFile;

#[derive(Debug, Serialize)]
pub enum EditFileResponse {
    Created,
    PartiallyModified,
    FullyModified,
}

impl Tool for EditFile {
    const NAME: &'static str = "edit_file";
    type Error = EditFileError;
    type Args = EditFileArgs;
    type Output = EditFileResponse;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "edit_file".to_string(),
            description: "Create or edit a file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "path of the file to create/edit"
                    },
                    "old_str": {
                        "type": ["string", "null"],
                        "description": "If provided, replace all occurrences of this string with new_str. Should be set to null if a new file is to be created, or completely modified with new_str"
                    },
                    "new_str": {
                        "type": "string",
                        "description": "string to replace with"
                    },
                },
                "required": ["path", "new_str"],
            }),
        }
    }

    #[instrument(level = Level::TRACE, name = "tool-call: edit_file", ret, err(level = Level::ERROR), skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        println!(
            "{}",
            format!("[tool-call] edit_file '{}'", args.path).yellow()
        );
        let path = args.path;
        if path.is_empty() {
            // TODO: encode this in the type system
            return Err(EditFileError::InvalidExpectation(
                "path cannot be empty".to_string(),
            ));
        }
        let old_str = args.old_str;
        let new_str = args.new_str;

        if let Some(o) = &old_str {
            if o.is_empty() {
                // TODO: encode this in the type system
                return Err(EditFileError::InvalidExpectation(
                    "old_str cannot be empty".to_string(),
                ));
            }

            if o == &new_str {
                return Err(EditFileError::NoChangesRequested);
            }
        }

        let path = PathBuf::from(path);
        match tokio::fs::metadata(&path).await {
            Ok(m) => {
                if !m.is_file() {
                    return Err(EditFileError::NotAFile);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if old_str.is_some() {
                    return Err(EditFileError::InvalidExpectation(
                        "path doesn't exist and old_str is not null".to_string(),
                    ));
                }

                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(EditFileError::CouldntCreateDirectory)?;
                }
                tokio::fs::write(path, new_str)
                    .await
                    .map_err(EditFileError::CouldntWriteToFile)?;

                return Ok(EditFileResponse::Created);
            }

            Err(e) => return Err(EditFileError::CouldntGetMetadata(e)),
        }

        match &old_str {
            Some(old_str) => {
                let old_contents = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(EditFileError::CouldntReadFile)?;
                let new_contents = old_contents.replace(old_str, &new_str);

                if old_contents == new_contents {
                    return Err(EditFileError::NothingChanged);
                }

                tokio::fs::write(path, new_contents)
                    .await
                    .map_err(EditFileError::CouldntWriteToFile)?;

                Ok(EditFileResponse::PartiallyModified)
            }
            None => {
                tokio::fs::write(path, new_str)
                    .await
                    .map_err(EditFileError::CouldntWriteToFile)?;
                Ok(EditFileResponse::FullyModified)
            }
        }
    }
}
