//! GitHub sign-in. GitHub is OAuth 2.0 only (no OpenID Connect), so the identity comes from its
//! REST API: `/user` for the stable numeric id and name, `/user/emails` for a verified address.
//! The authorization code flow still uses state and PKCE like the OIDC providers.

use super::providers::Provider;
use linkdean_core::Error;
use serde::Deserialize;

const AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const USER_URL: &str = "https://api.github.com/user";
const EMAILS_URL: &str = "https://api.github.com/user/emails";

/// What GitHub vouches for.
pub(super) struct GitHubIdentity {
    /// The numeric user id, which survives login renames.
    pub subject: String,
    pub email: String,
    pub name: Option<String>,
}

/// The authorization URL. `read:user` and `user:email` are the least that yields a verified
/// address.
pub(super) fn authorize_url(
    p: &Provider,
    redirect: &str,
    state: &str,
    code_challenge: &str,
) -> Result<String, Error> {
    let mut url = url::Url::parse(AUTHORIZE_URL).map_err(|e| Error::msg(e.to_string()))?;
    url.query_pairs_mut()
        .append_pair("client_id", &p.client_id)
        .append_pair("redirect_uri", redirect)
        .append_pair("scope", "read:user user:email")
        .append_pair("state", state)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("allow_signup", "false");
    Ok(url.into())
}

#[derive(Deserialize, utoipa::ToSchema)]
struct TokenAnswer {
    access_token: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct GitHubUser {
    id: u64,
    name: Option<String>,
    login: String,
}

#[derive(Deserialize, Clone, utoipa::ToSchema)]
pub(super) struct GitHubEmail {
    pub email: String,
    pub primary: bool,
    pub verified: bool,
}

/// Exchange the code and read the identity.
pub(super) async fn identity(
    http: &reqwest::Client,
    p: &Provider,
    redirect: &str,
    code: &str,
    verifier: &str,
) -> Result<GitHubIdentity, Error> {
    let answer: TokenAnswer = http
        .post(TOKEN_URL)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&[
            ("client_id", p.client_id.as_str()),
            ("client_secret", p.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| {
            tracing::error!(error = %e, "GitHub token exchange failed");
            Error::BadRequest("token exchange failed".into())
        })?
        .json()
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let token = match (answer.access_token, answer.error) {
        (Some(t), _) if !t.is_empty() => t,
        (_, err) => {
            tracing::warn!(error = ?err, "GitHub refused the code");
            return Err(Error::BadRequest("token exchange failed".into()));
        }
    };
    let user: GitHubUser = api_get(http, USER_URL, &token).await?;
    let emails: Vec<GitHubEmail> = api_get(http, EMAILS_URL, &token).await?;
    let email = pick_email(&emails).ok_or_else(|| {
        Error::BadRequest("your GitHub account has no verified email address".into())
    })?;
    Ok(GitHubIdentity {
        subject: user.id.to_string(),
        email,
        name: user
            .name
            .filter(|n| !n.trim().is_empty())
            .or(Some(user.login)),
    })
}

async fn api_get<T: serde::de::DeserializeOwned>(
    http: &reqwest::Client,
    url: &str,
    token: &str,
) -> Result<T, Error> {
    http.get(url)
        .bearer_auth(token)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header(reqwest::header::USER_AGENT, "Linkdean")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| {
            tracing::error!(error = %e, url, "GitHub API request failed");
            Error::BadRequest("GitHub did not share your account details".into())
        })?
        .json()
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

/// The primary address when it is verified, else the first verified one. Unverified addresses
/// never count: anyone can add one to a GitHub account.
pub(super) fn pick_email(emails: &[GitHubEmail]) -> Option<String> {
    emails
        .iter()
        .find(|e| e.primary && e.verified)
        .or_else(|| emails.iter().find(|e| e.verified))
        .map(|e| e.email.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::auth::providers::{ProviderId, GITHUB_ISSUER};

    fn email(email: &str, primary: bool, verified: bool) -> GitHubEmail {
        GitHubEmail {
            email: email.into(),
            primary,
            verified,
        }
    }

    #[test]
    fn picks_the_verified_primary_then_any_verified_never_unverified() {
        assert_eq!(
            pick_email(&[
                email("a@x.test", false, true),
                email("p@x.test", true, true)
            ])
            .as_deref(),
            Some("p@x.test")
        );
        assert_eq!(
            pick_email(&[
                email("p@x.test", true, false),
                email("b@x.test", false, true)
            ])
            .as_deref(),
            Some("b@x.test")
        );
        assert_eq!(pick_email(&[email("p@x.test", true, false)]), None);
        assert_eq!(pick_email(&[]), None);
    }

    #[test]
    fn authorize_url_sends_state_pkce_and_minimal_scopes() {
        let p = Provider {
            id: ProviderId::GitHub,
            label: "GitHub".into(),
            issuer: GITHUB_ISSUER.into(),
            client_id: "cid".into(),
            client_secret: "secret".into(),
        };
        let url = authorize_url(&p, "https://ld.test/api/auth/oidc/callback", "st", "ch").unwrap();
        let parsed = url::Url::parse(&url).unwrap();
        let q: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
        assert_eq!(parsed.host_str(), Some("github.com"));
        assert_eq!(q["client_id"], "cid");
        assert_eq!(q["state"], "st");
        assert_eq!(q["code_challenge"], "ch");
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["scope"], "read:user user:email");
        assert_eq!(q["redirect_uri"], "https://ld.test/api/auth/oidc/callback");
        assert!(!url.contains("secret"));
    }
}
