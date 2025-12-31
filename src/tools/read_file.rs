use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{Level, instrument, trace};

#[derive(Debug, Deserialize)]
pub struct ReadFileArgs {
    pub path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadFileError {
    #[error("couldn't read file: {0}")]
    CouldntReadFile(#[from] std::io::Error),
}

#[derive(Deserialize, Serialize)]
pub struct ReadFileTool;

impl Tool for ReadFileTool {
    const NAME: &'static str = "read_file";
    type Error = ReadFileError;
    type Args = ReadFileArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read a file on the local filesystem".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "path of the file to read"
                    },
                },
                "required": ["path"],
            }),
        }
    }

    #[instrument(level = Level::TRACE, name = "tool-call: read_file", err(level = Level::ERROR), skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let contents = tokio::fs::read_to_string(&args.path).await?;

        trace!(bytes_read = contents.len(), "file read successfully");

        Ok(contents)
    }
}

impl ReadFileTool {
    pub fn repr(args: &ReadFileArgs) -> String {
        format!("read_file: {}", args.path)
    }

    pub fn details(_args: &ReadFileArgs) -> Option<String> {
        None
    }
}
