//! Rules for connecting the browser extension: which redirect addresses may receive a code,
//! and the PKCE check that ties the code to the extension that asked for it.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use url::{Host, Url};

/// `base64url(SHA-256(verifier))` without padding: the PKCE `S256` challenge for `verifier`.
pub fn pkce_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// A PKCE verifier as RFC 7636 allows: 43 to 128 unreserved characters.
pub fn valid_verifier(verifier: &str) -> bool {
    (43..=128).contains(&verifier.len())
        && verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'))
}

/// An `S256` challenge: 43 base64url characters, the length of an encoded SHA-256.
pub fn valid_challenge(challenge: &str) -> bool {
    challenge.len() == 43
        && challenge
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

/// The browser whose extension sign-in flow owns `redirect_uri`, or `None` for any other
/// address. Only these hosts can receive a code: browsers route them back to the extension
/// that opened the sign-in window and never load them as web pages.
///
/// - Chrome and Edge: `https://<32 letters a–p>.chromiumapp.org/…` (`chrome.identity`)
/// - Firefox: `https://<40 hex>.extensions.allizom.org/…` (`browser.identity`)
pub fn extension_browser(redirect_uri: &str) -> Option<&'static str> {
    let url = Url::parse(redirect_uri).ok()?;
    if url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    let Some(Host::Domain(host)) = url.host() else {
        return None;
    };
    let (label, rest) = host.split_once('.')?;
    match rest {
        "chromiumapp.org"
            if label.len() == 32 && label.bytes().all(|b| (b'a'..=b'p').contains(&b)) =>
        {
            Some("Chrome")
        }
        "extensions.allizom.org"
            if label.len() == 40
                && label
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) =>
        {
            Some("Firefox")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_the_rfc_example() {
        // RFC 7636, appendix B.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert!(valid_verifier(verifier));
        assert_eq!(
            pkce_challenge(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        assert!(valid_challenge(&pkce_challenge(verifier)));
        assert!(!valid_verifier("short"));
        assert!(!valid_verifier(&"a".repeat(129)));
        assert!(!valid_verifier(&format!("{}+", "a".repeat(50))));
        assert!(!valid_challenge(
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM="
        ));
    }

    #[test]
    fn only_extension_redirects_are_allowed() {
        let chrome = format!("https://{}.chromiumapp.org/", "a".repeat(32));
        let firefox = format!("https://{}.extensions.allizom.org/cb", "0f".repeat(20));
        assert_eq!(extension_browser(&chrome), Some("Chrome"));
        assert_eq!(extension_browser(&firefox), Some("Firefox"));
        for bad in [
            "https://evil.example/".to_string(),
            format!("http://{}.chromiumapp.org/", "a".repeat(32)),
            format!("https://{}.chromiumapp.org/", "z".repeat(32)),
            format!("https://{}.chromiumapp.org/", "a".repeat(31)),
            format!("https://{}.chromiumapp.org.evil.example/", "a".repeat(32)),
            format!("https://x.{}.chromiumapp.org/", "a".repeat(32)),
            format!("https://{}.chromiumapp.org:8443/", "a".repeat(32)),
            format!("https://u:p@{}.chromiumapp.org/", "a".repeat(32)),
            format!("https://{}.chromiumapp.org/#frag", "a".repeat(32)),
            format!("https://{}.extensions.allizom.org/", "g".repeat(40)),
            "javascript:alert(1)".to_string(),
        ] {
            assert_eq!(extension_browser(&bad), None, "{bad}");
        }
    }
}
