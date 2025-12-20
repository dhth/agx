use colored::Colorize;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{Level, instrument};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    File,
    Dir,
}

#[derive(Debug, serde::Serialize)]
pub struct DirEntry {
    pub name: String,
    pub kind: EntryKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ReadDirArgs {
    path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadDirError {
    #[error("couldn't get metadata for path: {0}")]
    CouldntGetMetadata(#[from] std::io::Error),
    #[error("path is not a directory")]
    PathNotADir,
    #[error("couldn't read directory: {0}")]
    CouldntReadDirectory(std::io::Error),
    #[error("couldn't read entry: {0}")]
    CouldntReadEntry(std::io::Error),
    #[error("couldn't get metadata for entry: {0}")]
    CouldntGetEntryMetadata(std::io::Error),
}

#[derive(Deserialize, Serialize)]
pub struct ReadDirTool;

impl Tool for ReadDirTool {
    const NAME: &'static str = "read_dir";
    type Error = ReadDirError;
    type Args = ReadDirArgs;
    type Output = Vec<DirEntry>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "read_dir".to_string(),
            description: "Read entries in a directory on the local filesystem".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "path of the directory to read"
                    },
                },
                "required": ["path"],
            }),
        }
    }

    #[instrument(level = Level::TRACE, name = "tool-call: read_dir", ret, err(level = Level::ERROR), skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let log = format!("read directory ({})", args.path).cyan();
        let result = self.call_inner(args).await;
        let status = if result.is_ok() { "✓" } else { "❌" };
        println!("{} {}", log, status,);

        result
    }
}

impl ReadDirTool {
    async fn call_inner(&self, args: ReadDirArgs) -> Result<Vec<DirEntry>, ReadDirError> {
        let metadata = tokio::fs::metadata(&args.path).await?;
        if !metadata.is_dir() {
            return Err(ReadDirError::PathNotADir);
        }

        let mut read_dir = tokio::fs::read_dir(&args.path)
            .await
            .map_err(ReadDirError::CouldntReadDirectory)?;

        let mut entries = vec![];

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(ReadDirError::CouldntReadEntry)?
        {
            let entry_metadata = entry
                .metadata()
                .await
                .map_err(ReadDirError::CouldntGetEntryMetadata)?;

            let kind = if entry_metadata.is_dir() {
                EntryKind::Dir
            } else {
                EntryKind::File
            };

            let size = entry_metadata.is_file().then_some(entry_metadata.len());

            entries.push(DirEntry {
                name: entry.path().to_string_lossy().to_string(),
                kind,
                size,
            });
        }

        Ok(entries)
    }
}
