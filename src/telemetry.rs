use anyhow::Context;
use etcetera::BaseStrategy;
use etcetera::base_strategy::Xdg;
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const LOG_ENV_VAR: &str = "AGX_LOG";

pub struct TelemetryGuard {
    tracer_provider: Option<SdkTracerProvider>,
}

impl TelemetryGuard {
    pub fn shutdown(self) {
        if let Some(provider) = self.tracer_provider
            && let Err(err) = provider.shutdown()
        {
            tracing::error!(?err, "couldn't shut down otel tracer");
        }
    }
}

pub fn setup(xdg: &Xdg, otel: bool) -> anyhow::Result<TelemetryGuard> {
    if std::env::var(LOG_ENV_VAR).map_or(true, |v| v.is_empty()) {
        return Ok(TelemetryGuard {
            tracer_provider: None,
        });
    }

    let log_file_path = get_log_file_path(xdg).context("couldn't determine log file path")?;

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .context("couldn't open log file")?;

    let (tracer_provider, trace_layer) = if otel {
        let tracer_provider = init_tracer_provider()?;
        let tracer = tracer_provider.tracer("agx");

        (
            Some(tracer_provider),
            Some(tracing_opentelemetry::layer().with_tracer(tracer)),
        )
    } else {
        (None, None)
    };

    let filter_layer = EnvFilter::from_env(LOG_ENV_VAR);

    let json_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_writer(log_file);

    tracing_subscriber::registry()
        .with(filter_layer)
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

    Ok(SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build())
}
