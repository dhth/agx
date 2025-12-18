use anyhow::Context;
use etcetera::BaseStrategy;
use etcetera::base_strategy::Xdg;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

const LOG_ENV_VAR: &str = "AGX_LOG";

pub fn setup(xdg: &Xdg) -> anyhow::Result<()> {
    if std::env::var(LOG_ENV_VAR).map_or(true, |v| v.is_empty()) {
        return Ok(());
    }

    let log_file_path = get_log_file_path(xdg).context("couldn't determine log file path")?;

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .context("couldn't open log file")?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env(LOG_ENV_VAR))
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_writer(log_file)
        .init();

    Ok(())
}

fn get_log_file_path(xdg: &Xdg) -> anyhow::Result<PathBuf> {
    let log_dir = get_log_dir(xdg);
    std::fs::create_dir_all(&log_dir).context("couldn't create log directory")?;

    // TODO: add clean up for long log files
    Ok(log_dir.join("agx.log"))
}

#[cfg(not(target_os = "windows"))]
pub fn get_log_dir(xdg: &Xdg) -> PathBuf {
    // XDG spec suggests using XDG_STATE_HOME for logs
    // https://specifications.freedesktop.org/basedir/latest/#variables

    xdg.state_dir() // this always returns Some on unix, but adding a fallback regardless
        .map(|d| d.join("agx"))
        .unwrap_or_else(|| xdg.home_dir().join(".agx"))
}

#[cfg(target_os = "windows")]
pub fn get_log_dir(xdg: &Xdg) -> PathBuf {
    xdg.cache_dir().join("agx")
}
