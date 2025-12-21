use anyhow::Context;
use tokio::io::AsyncReadExt;

const AGENTS_CONTEXT_FILE_MAX_SIZE: u64 = 50 * 1024;
const AGENTS_CONTEXT_FILE: &str = "AGENTS.md";

// TODO: read other context files, and follow links in the main file
pub async fn get_project_context() -> anyhow::Result<Option<String>> {
    let file = match tokio::fs::File::open(AGENTS_CONTEXT_FILE).await {
        Ok(f) => Ok(f),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => Err(e),
    }?;

    let mut buf = Vec::with_capacity(AGENTS_CONTEXT_FILE_MAX_SIZE as usize + 1);
    let bytes_read = file
        .take(AGENTS_CONTEXT_FILE_MAX_SIZE + 1)
        .read_to_end(&mut buf)
        .await
        .context("couldn't read AGENTS.md")?;

    if bytes_read as u64 > AGENTS_CONTEXT_FILE_MAX_SIZE {
        anyhow::bail!("AGENTS.md is too large; max size allowed is 50KB");
    }

    if buf.is_empty() {
        return Ok(None);
    }

    let contents = String::from_utf8(buf).context("AGENTS.md is not valid utf-8")?;

    Ok(Some(contents))
}
