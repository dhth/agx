use crate::domain::Config;
use anyhow::Context;
use std::path::{Path, PathBuf};

const AGX_DIR: &str = ".agx";
const LOCAL_CONFIG_FILE: &str = "config.local.json";

pub async fn get_local_config() -> anyhow::Result<Config> {
    let config_file_path = PathBuf::from(AGX_DIR).join(LOCAL_CONFIG_FILE);

    let config = get_config(&config_file_path).await.with_context(|| {
        format!(
            r#"couldn't get local config (from "{}")"#,
            config_file_path.to_string_lossy()
        )
    })?;

    Ok(config)
}

async fn get_config<P>(path: P) -> anyhow::Result<Config>
where
    P: AsRef<Path>,
{
    let config = match tokio::fs::read(path).await {
        Ok(bytes) => {
            let config: Config =
                serde_json::from_slice(bytes.as_slice()).context("couldn't parse file contents")?;
            Ok(config)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).context("couldn't read file"),
    }?;

    Ok(config)
}

pub async fn save_local_config(config: &Config) -> anyhow::Result<()> {
    let contents =
        serde_json::to_string_pretty(config).context("couldn't serialize config to JSON")?;

    let config_file_path = PathBuf::from(AGX_DIR).join(LOCAL_CONFIG_FILE);
    save_config(AGX_DIR, &config_file_path, &contents)
        .await
        .with_context(|| {
            format!(
                r#"couldn't save local config (to "{}")"#,
                config_file_path.to_string_lossy()
            )
        })?;

    Ok(())
}

async fn save_config<P>(dir: &str, file_path: P, contents: &str) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    tokio::fs::create_dir_all(dir)
        .await
        .context("couldn't create directory")?;

    tokio::fs::write(file_path, contents)
        .await
        .context("couldn't write contents to file")?;

    Ok(())
}
