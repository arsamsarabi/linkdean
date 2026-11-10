//! Terms of Service versioning. Sign-up must present this exact version; the web app ships the
//! same string (`apps/web/src/lib/legal`), so bump both together.

use crate::{Error, Result};

pub const TERMS_VERSION: &str = "2026-11-10";

/// Accepts only the current terms version. `None` and stale versions are a `BadRequest`.
pub fn require_current_terms(version: Option<&str>) -> Result<()> {
    match version.map(str::trim) {
        Some(v) if v == TERMS_VERSION => Ok(()),
        Some("") | None => Err(Error::BadRequest(
            "you must accept the Terms of Service to create an account".into(),
        )),
        Some(_) => Err(Error::BadRequest(
            "the Terms of Service changed; reload the page and accept the current terms".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_current_version_passes() {
        assert!(require_current_terms(Some(TERMS_VERSION)).is_ok());
        assert!(require_current_terms(None).is_err());
        assert!(require_current_terms(Some("")).is_err());
        assert!(require_current_terms(Some("1999-01-01")).is_err());
    }
}
