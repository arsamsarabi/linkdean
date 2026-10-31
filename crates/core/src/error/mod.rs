use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Msg(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("vault locked")]
    VaultLocked,
    #[error("vault unlock failed")]
    VaultUnlockFailed,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Msg(s.into())
    }
}
