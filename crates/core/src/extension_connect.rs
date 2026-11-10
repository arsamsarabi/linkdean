//! Rules for connecting the browser extension: which extension addresses may ask for a code,
//! and the PKCE check that ties the code to the extension that asked for it.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

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

/// The browser of an extension's base address, `chrome-extension://<id>/` (Chrome, Edge) or
/// `moz-extension://<uuid>/` (Firefox), exactly as `runtime.getURL("/")` returns it. Anything
/// else is `None`. The address only binds a code to the extension that asked for it; the code
/// itself reaches the extension inside the sign-in window, never through this address.
pub fn extension_browser(extension_origin: &str) -> Option<&'static str> {
    if let Some(id) = extension_origin
        .strip_prefix("chrome-extension://")
        .and_then(|rest| rest.strip_suffix('/'))
    {
        if id.len() == 32 && id.bytes().all(|b| (b'a'..=b'p').contains(&b)) {
            return Some("Chrome");
        }
        return None;
    }
    let uuid = extension_origin
        .strip_prefix("moz-extension://")
        .and_then(|rest| rest.strip_suffix('/'))?;
    let groups: Vec<&str> = uuid.split('-').collect();
    let lengths = [8, 4, 4, 4, 12];
    let ok = groups.len() == 5
        && groups.iter().zip(lengths).all(|(g, n)| {
            g.len() == n
                && g.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        });
    ok.then_some("Firefox")
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
    fn only_extension_base_addresses_are_allowed() {
        let chrome = format!("chrome-extension://{}/", "a".repeat(32));
        let firefox = "moz-extension://0f1e2d3c-4b5a-6978-8a9b-0c1d2e3f4a5b/";
        assert_eq!(extension_browser(&chrome), Some("Chrome"));
        assert_eq!(extension_browser(firefox), Some("Firefox"));
        for bad in [
            "https://evil.example/".to_string(),
            format!("chrome-extension://{}", "a".repeat(32)),
            format!("chrome-extension://{}/page.html", "a".repeat(32)),
            format!("chrome-extension://{}/", "z".repeat(32)),
            format!("chrome-extension://{}/", "a".repeat(31)),
            "moz-extension://0F1E2D3C-4B5A-6978-8A9B-0C1D2E3F4A5B/".to_string(),
            "moz-extension://0f1e2d3c4b5a69788a9b0c1d2e3f4a5b/".to_string(),
            "moz-extension://0f1e2d3c-4b5a-6978-8a9b-0c1d2e3f4a5b/x".to_string(),
            "javascript:alert(1)".to_string(),
        ] {
            assert_eq!(extension_browser(&bad), None, "{bad}");
        }
    }
}
