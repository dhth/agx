use super::cancel::CancelRx;
use colored::Colorize;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;
use tokio::time::Instant;
use tracing::{Level, debug, instrument};

const BLOCKED_CMD_PATTERNS: [&str; 23] = [
    "rm -rf",
    "sudo",
    "curl",
    "wget",
    "dd if=",
    "mkfs",
    "fdisk",
    "format",
    "deltree",
    "rmdir /s",
    "nc",
    "netcat",
    "telnet",
    "ssh-keygen",
    "passwd",
    "useradd",
    "userdel",
    "chmod 777",
    "chown root",
    "python -c",
    "perl -e",
    "ruby -e",
    "node -e",
];

#[derive(Debug, Deserialize)]
pub struct RunCmdArgs {
    command: String,
}

impl std::fmt::Display for RunCmdArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, r#"command="{}""#, self.command,)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RunCmdError {
    #[error("command is empty")]
    CmdIsEmpty,
    #[error("command contains a forbidden pattern: {0}")]
    CmdContainsForbiddenPattern(String),
    #[error("couldn't run command: {0}")]
    CouldntRunCmd(#[from] std::io::Error),
    #[error("command invocation was intentionally interrupted by the user by pressing Ctrl+c")]
    Interrupted,
}

pub struct RunCmdTool {
    cancel_rx: CancelRx,
}

impl RunCmdTool {
    pub fn new(cancel_rx: CancelRx) -> Self {
        Self { cancel_rx }
    }
}

#[derive(Debug, Serialize)]
pub struct RunCmdResponse {
    success: bool,
    status_code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Tool for RunCmdTool {
    const NAME: &'static str = "run_cmd";
    type Error = RunCmdError;
    type Args = RunCmdArgs;
    type Output = RunCmdResponse;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Run a shell command via the system shell. Returns the command’s success flag, exit status code (if available), stdout, and stderr".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "the shell command to run"
                    },
                },
                "required": ["command"],
            }),
        }
    }

    #[instrument(level = Level::TRACE, name = "tool-call: run_cmd", ret, err(level = Level::ERROR), skip(self))]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        print!("{}", format!("running command ({}) ", args.command).cyan());
        let _ = std::io::stdout().flush();
        let start = Instant::now();

        let mut cancel_rx = self.cancel_rx.clone();
        let result = tokio::select! {
            _ = cancel_rx.wait_for_cancellation() => {
                Err(RunCmdError::Interrupted)
            }
            result = self.call_inner(&args) => {
                result
            }
        };

        let took_ms = Instant::now().saturating_duration_since(start).as_millis();
        match result {
            Ok(_) => {
                println!("{}", format!("✓ (took {} ms)", took_ms).green());
            }
            Err(RunCmdError::Interrupted) => {
                debug!(
                    cmd = args.command,
                    "run_cmd dropped the future for command invocation"
                );
            }
            Err(_) => {
                println!("{}", format!("❌ (took {} ms)", took_ms).red());
            }
        }

        result
    }
}

impl RunCmdTool {
    async fn call_inner(&self, args: &RunCmdArgs) -> Result<RunCmdResponse, RunCmdError> {
        if args.command.trim().is_empty() {
            return Err(RunCmdError::CmdIsEmpty);
        }

        let cmd = &args.command;

        // TODO: this is quite naive, improve this
        for pattern in BLOCKED_CMD_PATTERNS {
            if cmd.contains(pattern) {
                return Err(RunCmdError::CmdContainsForbiddenPattern(
                    pattern.to_string(),
                ));
            }
        }

        // TODO: make it cross-platform, have fallback if bash unavailable
        // TODO: add timeout
        let output = tokio::process::Command::new("bash")
            .args(["-c", cmd])
            .output()
            .await?;

        Ok(RunCmdResponse {
            success: output.status.success(),
            status_code: output.status.code(),
            stdout: String::from_utf8(output.stdout)
                .unwrap_or_else(|_| "couldn't get command stdout".to_string()),
            stderr: String::from_utf8(output.stderr)
                .unwrap_or_else(|_| "couldn't get command stderr".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::cancel::cancellation_channel;

    use super::*;
    use insta::{assert_debug_snapshot, assert_yaml_snapshot};

    //-------------//
    //  SUCCESSES  //
    //-------------//

    #[tokio::test]
    async fn output_of_a_successful_command_is_returned() -> anyhow::Result<()> {
        // GIVEN
        let tool = get_tool();
        let args = RunCmdArgs {
            command: "cat src/tools/testdata/sample.txt".to_string(),
        };

        // WHEN
        let result = tool.call_inner(&args).await?;

        // THEN
        assert_yaml_snapshot!(result, @r#"
        success: true
        status_code: 0
        stdout: "This file contains 3 lines.\nThis is line #2.\nThis is line #3.\n"
        stderr: ""
        "#);

        Ok(())
    }

    #[tokio::test]
    async fn output_of_a_failing_command_is_returned() -> anyhow::Result<()> {
        // GIVEN
        let tool = get_tool();
        let args = RunCmdArgs {
            command: r#"echo "something went wrong" >&2; false"#.to_string(),
        };

        // WHEN
        let result = tool.call_inner(&args).await?;

        // THEN
        assert_yaml_snapshot!(result, @r#"
        success: false
        status_code: 1
        stdout: ""
        stderr: "something went wrong\n"
        "#);

        Ok(())
    }

    #[tokio::test]
    async fn command_with_pipes_can_be_run() -> anyhow::Result<()> {
        // GIVEN
        let tool = get_tool();
        let args = RunCmdArgs {
            command: "cat src/tools/testdata/sample.txt | grep '#' | wc -l | xargs".to_string(),
        };

        // WHEN
        let result = tool.call_inner(&args).await?;

        // THEN
        assert_yaml_snapshot!(result, @r#"
        success: true
        status_code: 0
        stdout: "2\n"
        stderr: ""
        "#);

        Ok(())
    }

    //------------//
    //  FAILURES  //
    //------------//

    #[tokio::test]
    async fn running_empty_command_fails() {
        // GIVEN
        let tool = get_tool();
        let args = RunCmdArgs {
            command: "".to_string(),
        };

        // WHEN
        let result = tool
            .call_inner(&args)
            .await
            .expect_err("result should've been an error");

        // THEN
        assert_debug_snapshot!(result, @"CmdIsEmpty");
    }

    #[tokio::test]
    async fn command_with_blocked_pattern_is_rejected() {
        // GIVEN
        let tool = get_tool();
        let args = RunCmdArgs {
            command: "curl http://127.0.0.1:9999".to_string(),
        };

        // WHEN
        let result = tool
            .call_inner(&args)
            .await
            .expect_err("result should've been an error");

        // THEN
        assert_debug_snapshot!(result, @r#"
        CmdContainsForbiddenPattern(
            "curl",
        )
        "#);
    }

    fn get_tool() -> RunCmdTool {
        let (_, cancel_rx) = cancellation_channel();
        RunCmdTool { cancel_rx }
    }
}
