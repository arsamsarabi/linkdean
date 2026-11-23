//! DNS TXT challenge helpers for organization verified domains.

use hickory_resolver::proto::rr::RData;
use hickory_resolver::TokioResolver;
use linkdean_core::Error;

const TXT_PREFIX: &str = "linkdean-domain-verification=";

/// Consumer mail hosts that must not be claimed as organization domains.
const CONSUMER_MAIL_DOMAINS: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "outlook.com",
    "hotmail.com",
    "live.com",
    "msn.com",
    "yahoo.com",
    "ymail.com",
    "icloud.com",
    "me.com",
    "mac.com",
    "aol.com",
    "proton.me",
    "protonmail.com",
];

pub fn is_consumer_mail_domain(domain: &str) -> bool {
    let d = domain.to_ascii_lowercase();
    CONSUMER_MAIL_DOMAINS
        .iter()
        .any(|blocked| d == *blocked || d.ends_with(&format!(".{blocked}")))
}

pub fn challenge_name(domain: &str) -> String {
    format!("_linkdean-challenge.{}", domain.trim_end_matches('.'))
}

pub fn challenge_value(token: &str) -> String {
    format!("{TXT_PREFIX}{token}")
}

/// True when any TXT record string equals or contains the expected challenge value.
pub fn txt_contains_token(records: &[String], token: &str) -> bool {
    let want = challenge_value(token);
    records.iter().any(|r| {
        r.split(|c: char| c.is_whitespace() || c == '"')
            .any(|part| part == want)
            || r.contains(&want)
    })
}

/// Look up TXT records at `name` (usually [`challenge_name`]).
pub async fn lookup_txt(name: &str) -> Result<Vec<String>, Error> {
    let fqdn = if name.ends_with('.') {
        name.to_string()
    } else {
        format!("{name}.")
    };
    let resolver = TokioResolver::builder_tokio()
        .map_err(|e| Error::msg(format!("dns resolver: {e}")))?
        .build()
        .map_err(|e| Error::msg(format!("dns resolver: {e}")))?;
    let lookup = resolver
        .txt_lookup(fqdn)
        .await
        .map_err(|e| Error::BadRequest(format!("DNS lookup failed: {e}")))?;
    let mut out = Vec::new();
    for record in lookup.answers() {
        if let RData::TXT(txt) = &record.data {
            let value: String = txt
                .txt_data
                .iter()
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .collect();
            out.push(value);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_shapes() {
        assert_eq!(challenge_name("Acme.COM"), "_linkdean-challenge.Acme.COM");
        assert_eq!(challenge_value("abc"), "linkdean-domain-verification=abc");
    }

    #[test]
    fn txt_match_accepts_exact_and_embedded() {
        let token = "deadbeef";
        assert!(txt_contains_token(&[challenge_value(token).into()], token));
        assert!(txt_contains_token(
            &[
                "v=spf1 include:_spf.google.com".into(),
                challenge_value(token)
            ],
            token
        ));
        assert!(!txt_contains_token(&["other".into()], token));
    }

    #[test]
    fn blocks_consumer_mail() {
        assert!(is_consumer_mail_domain("gmail.com"));
        assert!(is_consumer_mail_domain("mail.gmail.com"));
        assert!(!is_consumer_mail_domain("acme.com"));
    }
}
