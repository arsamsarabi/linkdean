//! Embedded disposable-email domain denylist (v1: no live download).

use linkdean_core::Error;

/// Common throwaway providers. Lowercase host only (no subdomain matching beyond the
/// registered domain as typed after `@`).
const DISPOSABLE_DOMAINS: &[&str] = &[
    "mailinator.com",
    "guerrillamail.com",
    "guerrillamail.org",
    "sharklasers.com",
    "grr.la",
    "guerrillamailblock.com",
    "pokemail.net",
    "spam4.me",
    "yopmail.com",
    "yopmail.fr",
    "trashmail.com",
    "trashmail.me",
    "tempmail.com",
    "temp-mail.org",
    "tempmailo.com",
    "10minutemail.com",
    "10minutemail.net",
    "throwaway.email",
    "getnada.com",
    "maildrop.cc",
    "dispostable.com",
    "mailnesia.com",
    "fakeinbox.com",
    "emailondeck.com",
    "mintemail.com",
    "moakt.com",
    "tmpmail.org",
    "tmpmail.net",
    "discard.email",
    "mailcatch.com",
    "mailnull.com",
    "spamgourmet.com",
    "mail-temp.com",
    "tempr.email",
];

/// Extract the domain from an email address (lowercase). `None` if there is no `@`.
pub fn email_domain(email: &str) -> Option<&str> {
    let email = email.trim();
    let at = email.rfind('@')?;
    let domain = email[at + 1..].trim();
    if domain.is_empty() {
        return None;
    }
    Some(domain)
}

pub fn is_disposable_domain(domain: &str) -> bool {
    let d = domain.to_ascii_lowercase();
    DISPOSABLE_DOMAINS
        .iter()
        .any(|blocked| d == *blocked || d.ends_with(&format!(".{blocked}")))
}

/// Refuse a disposable address when blocking is on.
pub fn refuse_if_disposable(email: &str, block: bool) -> Result<(), Error> {
    if !block {
        return Ok(());
    }
    let Some(domain) = email_domain(email) else {
        return Ok(());
    };
    if is_disposable_domain(domain) {
        return Err(Error::BadRequest("Use a lasting email address.".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_known_disposable_domains() {
        assert!(is_disposable_domain("mailinator.com"));
        assert!(is_disposable_domain("Mailinator.COM"));
        assert!(is_disposable_domain("foo.mailinator.com"));
        assert!(!is_disposable_domain("gmail.com"));
        assert!(!is_disposable_domain("notmailinator.com"));
    }

    #[test]
    fn refuse_respects_the_flag() {
        assert!(refuse_if_disposable("a@mailinator.com", false).is_ok());
        assert!(refuse_if_disposable("a@mailinator.com", true).is_err());
        assert!(refuse_if_disposable("a@example.com", true).is_ok());
    }
}
