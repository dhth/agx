use crate::domain::Config;
use anyhow::Context;
use std::path::PathBuf;

const AGX_DIR: &str = ".agx";
const LOCAL_CONFIG_FILE: &str = "config.local.json";

pub async fn get_local_config() -> anyhow::Result<Config> {
    let config_file_path = PathBuf::from(AGX_DIR).join(LOCAL_CONFIG_FILE);

    let config = match tokio::fs::read(".agx/config.local.json").await {
        Ok(bytes) => {
            let config: Config =
                serde_json::from_slice(bytes.as_slice()).context("couldn't parse file contents")?;
            Ok(config)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).context("couldn't read file"),
    }
    .with_context(|| {
        format!(
            r#"couldn't get config from local config file ("{}")"#,
            config_file_path.to_string_lossy()
        )
    })?;

    Ok(config)
}

pub async fn save_local_config(config: &Config) -> anyhow::Result<()> {
    let contents =
        serde_json::to_string_pretty(config).context("couldn't serialize config to JSON")?;

    tokio::fs::create_dir_all(AGX_DIR)
        .await
        .with_context(|| format!("couldn't create directory: {AGX_DIR}"))?;

    let config_file_path = PathBuf::from(AGX_DIR).join(LOCAL_CONFIG_FILE);
    tokio::fs::write(&config_file_path, contents)
        .await
        .with_context(|| {
            format!(
                r#"couldn't write contents to file "{}""#,
                config_file_path.to_string_lossy()
            )
        })?;

    Ok(())
}
