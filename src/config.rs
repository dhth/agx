use crate::domain::Config;
use anyhow::Context;
use std::path::PathBuf;

pub async fn get_local_config() -> anyhow::Result<Config> {
    let config_file_path = PathBuf::from(".agx").join("config.local.json");

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
