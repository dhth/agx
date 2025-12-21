use anyhow::Context;
use std::path::Path;
use tokio::io::AsyncReadExt;

const AGENTS_CONTEXT_FILE_MAX_SIZE: u64 = 50 * 1024;
const AGENTS_CONTEXT_FILE: &str = "AGENTS.md";

pub async fn get_project_context() -> anyhow::Result<Option<String>> {
    // TODO: read other context files, and follow links in the main file
    read_file_with_limit(AGENTS_CONTEXT_FILE, AGENTS_CONTEXT_FILE_MAX_SIZE)
        .await
        .with_context(|| format!("couldn't read context from {}", AGENTS_CONTEXT_FILE))
}

async fn read_file_with_limit<P>(path: P, limit: u64) -> anyhow::Result<Option<String>>
where
    P: AsRef<Path>,
{
    let file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).context("couldn't open file"),
    };

    let mut buf = Vec::with_capacity(limit as usize + 1);
    let bytes_read = file
        .take(limit + 1)
        .read_to_end(&mut buf)
        .await
        .context("couldn't read file")?;

    if bytes_read as u64 > limit {
        anyhow::bail!("file is too large; max size allowed is {} bytes", limit);
    }

    if buf.is_empty() {
        return Ok(None);
    }

    let contents = String::from_utf8(buf).context("file contents are not valid utf-8")?;

    Ok(Some(contents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::{assert_debug_snapshot, assert_snapshot};

    //-------------//
    //  SUCCESSES  //
    //-------------//

    #[tokio::test]
    async fn read_file_with_limit_works_when_limit_is_equal_to_size() -> anyhow::Result<()> {
        // GIVEN
        let path = "src/helpers/testdata/sample.txt";
        let limit = 18; // wc -c src/helpers/testdata/sample.txt

        // WHEN
        let result = read_file_with_limit(path, limit)
            .await?
            .expect("result should've been some");

        // THEN
        assert_snapshot!(result, @"context goes here");

        Ok(())
    }

    #[tokio::test]
    async fn read_file_with_limit_works_when_limit_is_greater_than_size() -> anyhow::Result<()> {
        // GIVEN
        let path = "src/helpers/testdata/sample.txt";
        let limit = 20;

        // WHEN
        let result = read_file_with_limit(path, limit)
            .await?
            .expect("result should've been some");

        // THEN
        assert_snapshot!(result, @"context goes here");

        Ok(())
    }
    #[tokio::test]
    async fn read_file_with_limit_works_for_empty_file() -> anyhow::Result<()> {
        // GIVEN
        let path = "src/helpers/testdata/empty.txt";
        let limit = 20;

        // WHEN
        let result = read_file_with_limit(path, limit).await?;

        // THEN
        assert!(result.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn read_file_with_limit_returns_none_for_nonexistent_file() -> anyhow::Result<()> {
        // GIVEN
        let path = "src/helpers/testdata/nonexistent.txt";
        let limit = 20;

        // WHEN
        let result = read_file_with_limit(path, limit).await?;

        // THEN
        assert!(result.is_none());

        Ok(())
    }

    //------------//
    //  FAILURES  //
    //------------//

    #[tokio::test]
    async fn read_file_with_limit_fails_when_limit_is_less_than_size() {
        // GIVEN
        let path = "src/helpers/testdata/sample.txt";
        let limit = 1;

        // WHEN
        let result = read_file_with_limit(path, limit)
            .await
            .expect_err("result should've been an error");

        // THEN
        assert_debug_snapshot!(result, @r#""file is too large; max size allowed is 1 bytes""#);
    }
}
