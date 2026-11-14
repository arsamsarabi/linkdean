//! Process logging: stdout text/JSON plus optional always-JSONL file sink, and optional
//! OpenTelemetry trace export.
//!
//! Tracing is on when `OTEL_EXPORTER_OTLP_ENDPOINT` (or `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`)
//! is set. Spans go to that collector over OTLP/HTTP (protobuf). The standard `OTEL_*`
//! variables apply: `OTEL_SERVICE_NAME` (default `linkdean-api`), `OTEL_EXPORTER_OTLP_HEADERS`,
//! `OTEL_TRACES_SAMPLER` and its argument. Every request span carries the `request_id` that is
//! also in the logs and the `X-Request-Id` response header.

use anyhow::{bail, Context};
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Text,
    Json,
}

impl LogFormat {
    pub fn parse(raw: Option<&str>) -> anyhow::Result<Self> {
        match raw.map(str::trim).filter(|s| !s.is_empty()) {
            None => Ok(Self::Text),
            Some("text") => Ok(Self::Text),
            Some("json") => Ok(Self::Json),
            Some(other) => bail!("LOG_FORMAT={other:?}: expected text or json"),
        }
    }
}

/// Flushes buffered spans when dropped. Keep it alive until the server has stopped.
pub struct TracingGuard(Option<opentelemetry_sdk::trace::SdkTracerProvider>);

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.0.take() {
            // The exporter's HTTP client blocks; keep it off the async runtime's threads.
            let _ = std::thread::spawn(move || {
                if let Err(e) = provider.shutdown() {
                    eprintln!("OpenTelemetry shutdown: {e}");
                }
            })
            .join();
        }
    }
}

/// Whether trace export is configured.
pub fn otel_enabled(get: impl Fn(&str) -> Option<String>) -> bool {
    [
        "OTEL_EXPORTER_OTLP_ENDPOINT",
        "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
    ]
    .iter()
    .any(|k| get(k).is_some_and(|v| !v.trim().is_empty()))
}

fn service_name(get: impl Fn(&str) -> Option<String>) -> String {
    get("OTEL_SERVICE_NAME")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "linkdean-api".into())
}

fn tracer_provider() -> anyhow::Result<opentelemetry_sdk::trace::SdkTracerProvider> {
    use opentelemetry_otlp::WithExportConfig;
    // reqwest's blocking client must not be built on an async runtime thread.
    let exporter = std::thread::spawn(|| {
        opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
            .build()
    })
    .join()
    .map_err(|_| anyhow::anyhow!("building the OTLP exporter panicked"))?
    .context("OTEL_EXPORTER_OTLP_ENDPOINT: building the OTLP exporter")?;
    let resource = opentelemetry_sdk::Resource::builder()
        .with_service_name(service_name(|k| std::env::var(k).ok()))
        .build();
    Ok(opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build())
}

/// Install the global subscriber. Call once from `main` before other work.
pub fn init() -> anyhow::Result<TracingGuard> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let format = LogFormat::parse(std::env::var("LOG_FORMAT").ok().as_deref())?;
    let file_path = log_file_path()?;

    let file_layer = match file_path {
        Some(path) => {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .with_context(|| format!("opening LOG_FILE={}", path.display()))?;
            Some(fmt::layer().json().with_writer(Mutex::new(file)))
        }
        None => None,
    };

    let provider = if otel_enabled(|k| std::env::var(k).ok()) {
        Some(tracer_provider()?)
    } else {
        None
    };
    let otel_layer = provider.as_ref().map(|p| {
        use opentelemetry::trace::TracerProvider as _;
        tracing_opentelemetry::layer().with_tracer(p.tracer("linkdean-api"))
    });

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(otel_layer);
    match format {
        LogFormat::Text => registry.with(fmt::layer()).init(),
        LogFormat::Json => registry.with(fmt::layer().json()).init(),
    }
    if provider.is_some() {
        tracing::info!("OpenTelemetry trace export on");
    }
    Ok(TracingGuard(provider))
}

fn log_file_path() -> anyhow::Result<Option<PathBuf>> {
    match std::env::var("LOG_FILE") {
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(e) => Err(e).context("reading LOG_FILE"),
        Ok(s) if s.trim().is_empty() => Ok(None),
        Ok(s) => Ok(Some(PathBuf::from(s))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_defaults_to_text() {
        assert_eq!(LogFormat::parse(None).unwrap(), LogFormat::Text);
        assert_eq!(LogFormat::parse(Some("")).unwrap(), LogFormat::Text);
        assert_eq!(LogFormat::parse(Some("  ")).unwrap(), LogFormat::Text);
    }

    #[test]
    fn format_accepts_text_and_json() {
        assert_eq!(LogFormat::parse(Some("text")).unwrap(), LogFormat::Text);
        assert_eq!(LogFormat::parse(Some("json")).unwrap(), LogFormat::Json);
        assert_eq!(LogFormat::parse(Some(" json ")).unwrap(), LogFormat::Json);
    }

    #[test]
    fn format_rejects_unknown() {
        let err = LogFormat::parse(Some("yaml")).unwrap_err().to_string();
        assert!(err.contains("LOG_FORMAT"), "{err}");
        assert!(err.contains("text or json"), "{err}");
    }

    #[test]
    fn otel_is_on_only_with_an_endpoint() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| v.to_string())
            }
        };
        assert!(!otel_enabled(env(&[])));
        assert!(!otel_enabled(env(&[("OTEL_EXPORTER_OTLP_ENDPOINT", " ")])));
        assert!(otel_enabled(env(&[(
            "OTEL_EXPORTER_OTLP_ENDPOINT",
            "http://collector:4318"
        )])));
        assert!(otel_enabled(env(&[(
            "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
            "http://collector:4318/v1/traces"
        )])));
        assert_eq!(service_name(env(&[])), "linkdean-api");
        assert_eq!(
            service_name(env(&[("OTEL_SERVICE_NAME", "ld-eu")])),
            "ld-eu"
        );
    }
}
