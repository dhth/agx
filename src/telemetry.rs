use crate::env::get_optional_env_var;
use anyhow::Context;
use etcetera::BaseStrategy;
use etcetera::base_strategy::Xdg;
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::path::PathBuf;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

const LOG_ENV_VAR: &str = "AGX_LOG";
const OTEL_ENV_VAR: &str = "AGX_OTEL";

pub struct TelemetryGuard {
    tracer_provider: Option<SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = &self.tracer_provider {
            let _ = provider.shutdown();
        }
    }
}

pub fn setup(xdg: &Xdg) -> anyhow::Result<TelemetryGuard> {
    let log = get_optional_env_var(LOG_ENV_VAR)?.is_some_and(|v| !v.is_empty());

    let json_layer = if log {
        let log_file_path = get_log_file_path(xdg).context("couldn't determine log file path")?;

        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
            .context("couldn't open log file")?;
        Some(
            tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(false)
                .with_writer(log_file)
                .with_filter(EnvFilter::from_env(LOG_ENV_VAR)),
        )
    } else {
        None
    };

    let otel = get_optional_env_var(OTEL_ENV_VAR)?.is_some_and(|v| v == "1");

    let (tracer_provider, trace_layer) = if otel {
        let tracer_provider = init_tracer_provider()?;
        let tracer = tracer_provider.tracer("agx");

        (
            Some(tracer_provider),
            Some(
                tracing_opentelemetry::layer()
                    .with_tracer(tracer)
                    .with_filter(LevelFilter::INFO),
            ),
        )
    } else {
        (None, None)
    };

    tracing_subscriber::registry()
        .with(trace_layer)
        .with(json_layer)
        .init();

    Ok(TelemetryGuard { tracer_provider })
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

fn init_tracer_provider() -> anyhow::Result<SdkTracerProvider> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .context("couldn't build OTEL span exporter")?;

    let resource = Resource::builder().with_service_name("agx").build();

    Ok(SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build())
}
