//! Process logging: stdout text/JSON plus optional always-JSONL file sink.

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

/// Install the global subscriber. Call once from `main` before other work.
pub fn init() -> anyhow::Result<()> {
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

    let registry = tracing_subscriber::registry().with(filter).with(file_layer);
    match format {
        LogFormat::Text => registry.with(fmt::layer()).init(),
        LogFormat::Json => registry.with(fmt::layer().json()).init(),
    }
    Ok(())
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
}
