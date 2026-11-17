//! Terms of Service acceptance. The version comes from the server's `TERMS_VERSION` setting; a
//! server without it asks for no acceptance.

use crate::{Error, Result};

/// Checks the version a sign-up accepted against `current`, the server's terms version. With no
/// current version there is nothing to accept. Otherwise `None` and stale versions are a
/// `BadRequest`.
pub fn require_current_terms(current: Option<&str>, accepted: Option<&str>) -> Result<()> {
    let Some(current) = current else {
        return Ok(());
    };
    match accepted.map(str::trim) {
        Some(v) if v == current => Ok(()),
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
        let current = Some("2026-11-15");
        assert!(require_current_terms(current, Some("2026-11-15")).is_ok());
        assert!(require_current_terms(current, None).is_err());
        assert!(require_current_terms(current, Some("")).is_err());
        assert!(require_current_terms(current, Some("1999-01-01")).is_err());
    }

    #[test]
    fn a_server_without_terms_asks_for_none() {
        assert!(require_current_terms(None, None).is_ok());
        assert!(require_current_terms(None, Some("1999-01-01")).is_ok());
    }
}
