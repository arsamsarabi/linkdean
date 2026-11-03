use anyhow::{anyhow, bail};
use lettre::message::Mailbox;
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlsMode {
    StartTls,
    Tls,
    None,
}

#[derive(Clone)]
pub struct MailConfig {
    pub host: String,
    pub port: u16,
    pub tls: TlsMode,
    pub credentials: Option<(String, String)>,
    pub from: Mailbox,
}

impl TlsMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StartTls => "starttls",
            Self::Tls => "tls",
            Self::None => "none",
        }
    }
}

/// Never prints the password.
impl std::fmt::Debug for MailConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MailConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("tls", &self.tls)
            .field("username", &self.credentials.as_ref().map(|(u, _)| u))
            .field("from", &self.from.to_string())
            .finish()
    }
}

impl MailConfig {
    /// `Ok(None)` when mail is off. `get` reads one variable, so tests need no process env.
    pub fn from_vars(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Option<Self>> {
        let var = |k: &str| {
            get(k)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let (host, from) = match (var("SMTP_HOST"), var("MAIL_FROM")) {
            (None, None) => return Ok(None),
            (Some(_), None) => bail!("MAIL_FROM is required when SMTP_HOST is set"),
            (None, Some(_)) => bail!("SMTP_HOST is required when MAIL_FROM is set"),
            (Some(host), Some(from)) => (host, from),
        };
        let tls = match var("SMTP_TLS").as_deref() {
            None | Some("starttls") => TlsMode::StartTls,
            Some("tls") => TlsMode::Tls,
            Some("none") => TlsMode::None,
            Some(other) => bail!("SMTP_TLS must be starttls, tls or none, not {other:?}"),
        };
        if tls == TlsMode::None && !is_local_host(&host) {
            bail!(
                "SMTP_TLS=none sends mail unencrypted, so it is only allowed for a local server \
                 (loopback, a private address, or a single-label host such as mailpit), not {host}"
            );
        }
        let port = match var("SMTP_PORT") {
            Some(p) => p
                .parse()
                .map_err(|_| anyhow!("SMTP_PORT must be a port number, not {p:?}"))?,
            None if tls == TlsMode::Tls => 465,
            None => 587,
        };
        // The password is taken as given: spaces can be part of it.
        let password = get("SMTP_PASSWORD").filter(|p| !p.is_empty());
        let credentials = match (var("SMTP_USERNAME"), password) {
            (Some(user), Some(pass)) => Some((user, pass)),
            (None, None) => None,
            _ => bail!("SMTP_USERNAME and SMTP_PASSWORD must be set together"),
        };
        let from: Mailbox = from
            .parse()
            .map_err(|e| anyhow!("MAIL_FROM is not a valid mailbox ({e}): {from:?}"))?;
        Ok(Some(Self {
            host,
            port,
            tls,
            credentials,
            from,
        }))
    }
}

/// Loopback, private addresses and single-label names (Docker service names such as `mailpit`).
fn is_local_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => ip.is_loopback() || ip.is_private(),
        Ok(IpAddr::V6(ip)) => ip.is_loopback() || (ip.segments()[0] & 0xfe00) == 0xfc00,
        Err(_) => !host.contains('.') && !host.contains(':'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn parse(pairs: &[(&str, &str)]) -> anyhow::Result<Option<MailConfig>> {
        let vars: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        MailConfig::from_vars(|k| vars.get(k).cloned())
    }

    fn refusal(pairs: &[(&str, &str)]) -> String {
        parse(pairs).err().expect("refused").to_string()
    }

    const BASE: [(&str, &str); 2] = [
        ("SMTP_HOST", "smtp.example.com"),
        ("MAIL_FROM", "Linkdean <no-reply@example.com>"),
    ];

    #[test]
    fn mail_is_off_without_host_and_sender() {
        assert!(parse(&[]).unwrap().is_none());
        assert!(parse(&[("SMTP_HOST", " "), ("MAIL_FROM", "")])
            .unwrap()
            .is_none());
    }

    #[test]
    fn defaults_to_starttls_on_587() {
        let cfg = parse(&BASE).unwrap().expect("on");
        assert_eq!(cfg.host, "smtp.example.com");
        assert_eq!(cfg.port, 587);
        assert_eq!(cfg.tls, TlsMode::StartTls);
        assert!(cfg.credentials.is_none());
        assert_eq!(cfg.from.email.to_string(), "no-reply@example.com");
    }

    #[test]
    fn implicit_tls_defaults_to_465_and_port_overrides() {
        let mut vars = BASE.to_vec();
        vars.push(("SMTP_TLS", "tls"));
        assert_eq!(parse(&vars).unwrap().unwrap().port, 465);
        vars.push(("SMTP_PORT", "2465"));
        assert_eq!(parse(&vars).unwrap().unwrap().port, 2465);
    }

    #[test]
    fn reads_credentials_as_a_pair() {
        let mut vars = BASE.to_vec();
        vars.extend([("SMTP_USERNAME", "bot"), ("SMTP_PASSWORD", "s3cret")]);
        let cfg = parse(&vars).unwrap().unwrap();
        assert_eq!(cfg.credentials, Some(("bot".into(), "s3cret".into())));
    }

    #[test]
    fn debug_output_hides_the_password() {
        let mut vars = BASE.to_vec();
        vars.extend([("SMTP_USERNAME", "bot"), ("SMTP_PASSWORD", "s3cret")]);
        let cfg = parse(&vars).unwrap().unwrap();
        let printed = format!("{cfg:?}");
        assert!(printed.contains("bot"));
        assert!(!printed.contains("s3cret"), "{printed}");
    }

    #[test]
    fn refuses_half_configurations() {
        assert!(refusal(&[("SMTP_HOST", "smtp.example.com")]).contains("MAIL_FROM"));
        assert!(refusal(&[("MAIL_FROM", "a@example.com")]).contains("SMTP_HOST"));
        let mut vars = BASE.to_vec();
        vars.push(("SMTP_USERNAME", "bot"));
        assert!(refusal(&vars).contains("SMTP_PASSWORD"));
    }

    #[test]
    fn refuses_bad_values_naming_the_variable() {
        let with = |k: &'static str, v: &'static str| {
            let mut vars = BASE.to_vec();
            vars.retain(|(name, _)| *name != k);
            vars.push((k, v));
            refusal(&vars)
        };
        assert!(with("MAIL_FROM", "not a mailbox").contains("MAIL_FROM"));
        assert!(with("SMTP_TLS", "ssl").contains("SMTP_TLS"));
        assert!(with("SMTP_PORT", "abc").contains("SMTP_PORT"));
    }

    #[test]
    fn plain_smtp_only_for_local_servers() {
        let none = |host: &'static str| {
            parse(&[
                ("SMTP_HOST", host),
                ("MAIL_FROM", "a@example.com"),
                ("SMTP_TLS", "none"),
            ])
        };
        for host in [
            "mailpit",
            "localhost",
            "127.0.0.1",
            "10.0.0.5",
            "192.168.1.9",
            "::1",
        ] {
            assert!(none(host).unwrap().is_some(), "{host} should be allowed");
        }
        for host in ["smtp.example.com", "8.8.8.8", "::ffff:127.0.0.1"] {
            let err = none(host).err().expect("refused").to_string();
            assert!(err.contains("SMTP_TLS"), "{host}: {err}");
        }
    }
}
