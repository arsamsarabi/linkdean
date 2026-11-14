//! The single sign-on providers an instance offers: a generic OpenID Connect issuer (Authentik,
//! Keycloak and the like) plus presets for Google, Microsoft and GitHub. Google and Microsoft are
//! OpenID Connect issuers with fixed addresses; GitHub is plain OAuth 2.0 with its own user API.
//!
//! An account holds at most one provider identity, stored as issuer and subject. GitHub has no
//! issuer, so its identities use [`GITHUB_ISSUER`].

use crate::config::Config;
use linkdean_core::Error;

/// The issuer stored for GitHub identities. GitHub is not an OpenID Connect issuer; this only
/// keeps its subjects (numeric user ids) apart from other providers'.
pub const GITHUB_ISSUER: &str = "https://github.com";
const GOOGLE_ISSUER: &str = "https://accounts.google.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderId {
    /// The issuer in `OIDC_ISSUER`.
    Oidc,
    Google,
    Microsoft,
    GitHub,
}

impl ProviderId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Oidc => "oidc",
            Self::Google => "google",
            Self::Microsoft => "microsoft",
            Self::GitHub => "github",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "oidc" => Some(Self::Oidc),
            "google" => Some(Self::Google),
            "microsoft" => Some(Self::Microsoft),
            "github" => Some(Self::GitHub),
            _ => None,
        }
    }
}

/// A configured provider.
#[derive(Debug, Clone)]
pub struct Provider {
    pub id: ProviderId,
    /// The name on its sign-in button.
    pub label: String,
    /// OpenID Connect issuer, or [`GITHUB_ISSUER`].
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
}

impl Provider {
    /// GitHub speaks OAuth 2.0 only: no discovery, no ID token.
    pub fn is_oauth_only(&self) -> bool {
        self.id == ProviderId::GitHub
    }
}

/// Every configured provider, generic OIDC first, in the order the sign-in page shows them.
pub fn configured(cfg: &Config) -> Vec<Provider> {
    let mut out = Vec::new();
    if let (Some(issuer), Some(client_id), Some(client_secret)) = (
        cfg.oidc_issuer.clone(),
        cfg.oidc_client_id.clone(),
        cfg.oidc_client_secret.clone(),
    ) {
        out.push(Provider {
            id: ProviderId::Oidc,
            label: cfg
                .oidc_name
                .clone()
                .unwrap_or_else(|| "single sign-on".into()),
            issuer,
            client_id,
            client_secret,
        });
    }
    if let Some((client_id, client_secret)) = cfg.google_sso.clone() {
        out.push(Provider {
            id: ProviderId::Google,
            label: "Google".into(),
            issuer: GOOGLE_ISSUER.into(),
            client_id,
            client_secret,
        });
    }
    if let Some(ms) = cfg.microsoft_sso.clone() {
        out.push(Provider {
            id: ProviderId::Microsoft,
            label: "Microsoft".into(),
            issuer: microsoft_issuer(&ms.tenant_id),
            client_id: ms.client_id,
            client_secret: ms.client_secret,
        });
    }
    if let Some((client_id, client_secret)) = cfg.github_sso.clone() {
        out.push(Provider {
            id: ProviderId::GitHub,
            label: "GitHub".into(),
            issuer: GITHUB_ISSUER.into(),
            client_id,
            client_secret,
        });
    }
    out
}

fn microsoft_issuer(tenant_id: &str) -> String {
    format!("https://login.microsoftonline.com/{tenant_id}/v2.0")
}

/// The provider asked for, or the first configured one when `requested` is empty.
pub fn pick(cfg: &Config, requested: Option<&str>) -> Result<Provider, Error> {
    let all = configured(cfg);
    match requested.map(str::trim).filter(|s| !s.is_empty()) {
        None => all
            .into_iter()
            .next()
            .ok_or_else(|| Error::BadRequest("single sign-on is not set up".into())),
        Some(raw) => {
            let id = ProviderId::parse(raw)
                .ok_or_else(|| Error::BadRequest("unknown sign-in provider".into()))?;
            find(cfg, id)
        }
    }
}

/// The configured provider with `id`.
pub fn find(cfg: &Config, id: ProviderId) -> Result<Provider, Error> {
    configured(cfg)
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| Error::BadRequest(format!("{} sign-in is not set up", id.as_str())))
}

/// The configured provider that issued identities with `issuer`, if it is still set up.
pub fn for_issuer(cfg: &Config, issuer: &str) -> Option<Provider> {
    configured(cfg).into_iter().find(|p| p.issuer == issuer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_and_unknown_ids_are_refused() {
        for id in [
            ProviderId::Oidc,
            ProviderId::Google,
            ProviderId::Microsoft,
            ProviderId::GitHub,
        ] {
            assert_eq!(ProviderId::parse(id.as_str()), Some(id));
        }
        assert_eq!(ProviderId::parse("GitHub"), None);
        assert_eq!(ProviderId::parse(""), None);
    }

    #[test]
    fn microsoft_issuer_is_per_tenant() {
        assert_eq!(
            microsoft_issuer("72f988bf-86f1-41af-91ab-2d7cd011db47"),
            "https://login.microsoftonline.com/72f988bf-86f1-41af-91ab-2d7cd011db47/v2.0"
        );
    }
}
