use anyhow::Context;

const AGENTS_FILE_MAX_SIZE: u64 = 50 * 1024;
const AGENTS_CONTEXT_FILE: &str = "AGENTS.md";

// TODO: read other context files, and follow links in the main file
pub async fn get_project_context() -> anyhow::Result<Option<String>> {
    match tokio::fs::metadata(AGENTS_CONTEXT_FILE).await {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Ok(None);
            }

            if metadata.len() > AGENTS_FILE_MAX_SIZE {
                anyhow::bail!("AGENTS.md is too large; max size allowed is 50KB");
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).context("couldn't determine metadata for AGENTS.md"),
    };

    let contents = tokio::fs::read_to_string(AGENTS_CONTEXT_FILE)
        .await
        .with_context(|| "couldn't read AGENTS.md")?;

    Ok(Some(contents))
}
