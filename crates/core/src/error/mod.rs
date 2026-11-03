use chrono::{DateTime, Utc};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Msg(String),
    #[error("unauthorized")]
    Unauthorized,
    /// No usable session or API key, unlike `Unauthorized`, which is a wrong password.
    #[error("session expired")]
    SessionExpired,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("banned")]
    Banned {
        reason: String,
        ends_at: Option<DateTime<Utc>>,
    },
    /// The instance requires a verified email and this account has not verified its address.
    #[error("verify your email address to continue")]
    EmailUnverified,
    #[error("vault locked")]
    VaultLocked,
    #[error("vault unlock failed")]
    VaultUnlockFailed,
    #[error("too many attempts, try again later")]
    TooManyRequests,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Msg(s.into())
    }
}
