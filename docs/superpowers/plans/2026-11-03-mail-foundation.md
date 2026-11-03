# Mail Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let Linkdean send email: SMTP configuration from environment variables, MJML templates rendered in Rust, a database outbox with a background sender, an Admin → Email panel with a test send, and Mailpit plus a template preview page for development.

**Architecture:** A new `crates/mail` crate owns configuration parsing, template rendering (minijinja → MJML via `mrml` → HTML, plus plain text via `html2text`) and SMTP delivery (`lettre`). `crates/db` gains an `email_outbox` table and its queries. The API holds an optional `Mail` service in `AppState`, runs a sender loop next to the existing maintenance task, and exposes `GET /api/admin/email`, `POST /api/admin/email/test` and dev-only `/api/dev/emails`. The web app adds one Admin panel.

**Tech Stack:** Rust (axum 0.8, sqlx 0.8, tokio), `mrml` 6.0.1, `minijinja` 2.24.0, `lettre` 0.11.23, `html2text` 0.17.4, Postgres 16, React + Mantine 9 + TanStack Query + zod, vitest, Mailpit v1.31.0.

Spec: [`docs/superpowers/specs/2026-11-03-mail-foundation-design.md`](../specs/2026-11-03-mail-foundation-design.md).

**Repo rules for every task** (from `CLAUDE.md` and the repo skills):

- Work on `main`. Commit each green task and push. Never commit red. Commit messages are normal prose and end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Rust tests: `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test cargo test -j 2 …` against the throwaway `pg-check` container, never the dev database.
- Rust format check: `docker run --rm -v "$PWD":/w -w /w rust:1.89 sh -c "rustup component add rustfmt >/dev/null 2>&1; cargo fmt --all -- --check"`. Fix with the same container and `cargo fmt --all`.
- Web checks from `apps/web`: `npx prettier --check src`, `npx oxlint src`, `npx tsc -b --noEmit`, `npx vitest run`.
- Pinned crate and image versions here are all at least two weeks old on 2026-11-03. Do not bump them to anything newer than two weeks.
- After changing files, run `graphify update .` from the repo root before reporting. Do not commit `graphify-out/`.

---

## File map

| File | Responsibility |
| --- | --- |
| `Cargo.toml` (modify) | Add `crates/mail` member and the four new workspace dependencies |
| `crates/mail/Cargo.toml` (create) | The mail crate manifest |
| `crates/mail/src/lib.rs` (create) | Re-exports |
| `crates/mail/src/config.rs` (create) | `MailConfig`, `TlsMode`, env parsing and refusals |
| `crates/mail/src/render.rs` (create) | Template table, `Site`, `render`, `sample` |
| `crates/mail/src/send.rs` (create) | `Mailer` over lettre's async SMTP transport |
| `crates/mail/templates/layout.mjml` (create) | Shared layout every email extends |
| `crates/mail/templates/macros.mjml` (create) | `action(url, label)` button plus printed link |
| `crates/mail/templates/test.{mjml,subject,sample.json}` (create) | The Admin test email |
| `crates/mail/tests/smtp.rs` (create) | Ignored delivery test against a throwaway Mailpit |
| `migrations/040_email_outbox.sql` (create) | Outbox table |
| `crates/db/src/repo/email_outbox.rs` (create) | Enqueue, claim, mark sent, record failure, stats, purge |
| `crates/db/src/repo/mod.rs` (modify) | Register the module |
| `crates/db/tests/email_outbox.rs` (create) | Queue tests, each in a fresh database |
| `crates/api/Cargo.toml` (modify) | Depend on `linkdean-mail` |
| `crates/api/src/config/mod.rs` (modify) | `mail`, `dev_routes` fields |
| `crates/api/src/state/mod.rs` (modify) | `mail: Option<Arc<Mail>>` |
| `crates/api/src/mail.rs` (create) | `Mail` service, `enqueue`, `deliver_due`, `run_sender` |
| `crates/api/src/main.rs` (modify) | Spawn the sender, mount dev routes |
| `crates/api/src/maintenance.rs` (modify) | Purge old outbox rows hourly |
| `crates/api/src/routes/admin/email.rs` (create) | Admin status and test send |
| `crates/api/src/routes/admin/mod.rs` (modify) | Mount the email routes |
| `crates/api/src/routes/dev.rs` (create) | Template previews |
| `crates/api/src/routes/mod.rs` (modify) | Export `dev_router` |
| `deploy/docker/docker-compose.dev.yml` (modify) | Mailpit service, mail network, SMTP env |
| `deploy/docker/docker-compose.yml` (modify) | Pass SMTP env vars through |
| `apps/web/src/schemas/mail/{mail.ts,index.ts}` (create), `apps/web/src/schemas/index.ts` (modify) | zod schemas |
| `apps/web/src/api/api.ts` (modify) | `getEmailStatus`, `sendTestEmail` |
| `apps/web/src/hooks/settings/useAdminSettings.ts`, `index.ts` (modify) | `useEmailStatus`, `useSendTestEmail` |
| `apps/web/src/pages/settings/AdminEmailPanel.tsx` (create) + test | The panel |
| `apps/web/src/pages/settings/AdminPlatformSettings.tsx` (modify) | Mount the panel |
| `README.md`, `docs/SELF_HOSTED.md`, `docs/ARCHITECTURE.md`, `docs/FEATURES.md`, `docs/ROADMAP.md` (modify) | Docs |

---

### Task 1: Mail crate and configuration parsing

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/mail/Cargo.toml`, `crates/mail/src/lib.rs`, `crates/mail/src/config.rs`

- [ ] **Step 1: Add the crate to the workspace**

In `Cargo.toml`, add `"crates/mail",` to `members` after `"crates/db",`, and add to `[workspace.dependencies]` after `linkdean-worker`:

```toml
linkdean-mail = { path = "crates/mail" }
html2text = "0.17.4"
lettre = { version = "0.11.23", default-features = false, features = ["builder", "smtp-transport", "hostname", "tokio1", "tokio1-rustls-tls"] }
minijinja = "2.24.0"
mrml = "6.0.1"
```

Create `crates/mail/Cargo.toml`:

```toml
[package]
name = "linkdean-mail"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
anyhow.workspace = true
html2text.workspace = true
lettre.workspace = true
minijinja.workspace = true
mrml.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
url.workspace = true

[dev-dependencies]
reqwest.workspace = true
tokio.workspace = true
uuid.workspace = true
```

Create `crates/mail/src/lib.rs`:

```rust
//! Outbound email: configuration, MJML templates and SMTP delivery.

mod config;
mod render;
mod send;

pub use config::{MailConfig, TlsMode};
pub use render::{render, sample, template_names, RenderError, Rendered, Site};
pub use send::Mailer;
```

Until Tasks 2 and 3 land, create `render.rs` and `send.rs` as empty files and comment out their two `pub use` lines, so this task compiles on its own. Task 2 and Task 3 restore them.

- [ ] **Step 2: Write the failing config tests**

Create `crates/mail/src/config.rs` with only the test module and an empty type, so the tests compile and fail:

```rust
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

impl MailConfig {
    pub fn from_vars(_get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Option<Self>> {
        bail!("not implemented")
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
        assert!(parse(&[("SMTP_HOST", " "), ("MAIL_FROM", "")]).unwrap().is_none());
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
        for host in ["mailpit", "localhost", "127.0.0.1", "10.0.0.5", "192.168.1.9", "::1"] {
            assert!(none(host).unwrap().is_some(), "{host} should be allowed");
        }
        let err = none("smtp.example.com").err().expect("refused").to_string();
        assert!(err.contains("SMTP_TLS"));
    }
}
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -j 2 -p linkdean-mail`
Expected: compiles (after the lettre download), and every test except none fails with `not implemented`. If cargo rejects the `tokio1-rustls-tls` feature name, open `~/.cargo/registry/src/*/lettre-0.11.23/Cargo.toml`, pick the feature that enables rustls for the tokio1 transport with bundled webpki roots, and use that name in the workspace `Cargo.toml`.

- [ ] **Step 4: Implement parsing**

Replace the `TlsMode` impl area and `from_vars` in `config.rs` (keep the tests) with:

```rust
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
        let var = |k: &str| get(k).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
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
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test -j 2 -p linkdean-mail`
Expected: 7 passed.

- [ ] **Step 6: Format check and commit**

Run the fmt container check. Then:

```bash
git add Cargo.toml Cargo.lock crates/mail
git commit -m "Add the mail crate with SMTP configuration parsing.

Mail is off unless SMTP_HOST and MAIL_FROM are both set. Half
configurations, unknown TLS modes, bad ports and unparseable senders
stop startup with a message naming the variable. Unencrypted SMTP is
refused unless the server is local.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

---

### Task 2: Templates and rendering

**Files:**
- Create: `crates/mail/templates/layout.mjml`, `macros.mjml`, `test.mjml`, `test.subject`, `test.sample.json`
- Create: `crates/mail/src/render.rs`
- Modify: `crates/mail/src/lib.rs` (restore the `render` re-export)

- [ ] **Step 1: Write the templates**

`crates/mail/templates/layout.mjml`. Brand colours come from `apps/web/src/styles/theme.ts`: violet 6 `#8458B3`, gray 0 `#F6F8FC`, gray 6 `#636772`, gray 9 `#1B1E25`. No web fonts are loaded, so no third party sees the open.

```xml
<mjml lang="en">
  <mj-head>
    <mj-title>{% block title %}{% endblock %}</mj-title>
    <mj-preview>{% block preview %}{% endblock %}</mj-preview>
    <mj-attributes>
      <mj-all font-family="Satoshi, -apple-system, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif" />
      <mj-text font-size="15px" line-height="1.6" color="#1B1E25" />
      <mj-button background-color="#8458B3" color="#FFFFFF" font-size="15px" font-weight="600" border-radius="8px" inner-padding="12px 24px" />
    </mj-attributes>
  </mj-head>
  <mj-body background-color="#F6F8FC" width="560px">
    <mj-section padding="32px 0 16px">
      <mj-column>
        <mj-text align="center" font-size="22px" font-weight="700" letter-spacing="0.5px" color="#8458B3">Linkdean</mj-text>
      </mj-column>
    </mj-section>
    <mj-wrapper background-color="#FFFFFF" border-radius="12px" padding="8px 0">
      {% block content %}{% endblock %}
    </mj-wrapper>
    <mj-section padding="24px 0 32px">
      <mj-column>
        <mj-text align="center" font-size="12px" line-height="1.5" color="#636772">
          Sent by <a href="{{ site.app_url }}" style="color:#636772">{{ site.instance }}</a>, a Linkdean instance.<br />
          {% block why %}{% endblock %}
        </mj-text>
      </mj-column>
    </mj-section>
  </mj-body>
</mjml>
```

`crates/mail/templates/macros.mjml`:

```xml
{% macro action(url, label) %}
<mj-button href="{{ url }}">{{ label }}</mj-button>
<mj-text font-size="13px" color="#636772">If the button does not work, open this link:<br /><a href="{{ url }}" style="color:#8458B3;word-break:break-all">{{ url }}</a></mj-text>
{% endmacro %}
```

`crates/mail/templates/test.mjml`:

```xml
{% extends "layout.mjml" %}
{% block title %}Linkdean test email{% endblock %}
{% block preview %}Email from {{ site.instance }} works.{% endblock %}
{% block content %}
{% from "macros.mjml" import action %}
<mj-section>
  <mj-column>
    <mj-text font-size="20px" font-weight="700">Email works</mj-text>
    <mj-text>This test was sent to {{ recipient }} from the Admin settings of {{ site.instance }}. Nothing else is needed.</mj-text>
    {{ action(site.app_url, "Open Linkdean") }}
  </mj-column>
</mj-section>
{% endblock %}
{% block why %}You got this email because an admin sent a test from the Email settings.{% endblock %}
```

`crates/mail/templates/test.subject`:

```
Test email from {{ site.instance }}
```

`crates/mail/templates/test.sample.json`:

```json
{ "recipient": "ada@example.com" }
```

- [ ] **Step 2: Write the failing render tests**

Create `crates/mail/src/render.rs` with the public API stubbed and the tests:

```rust
use minijinja::{AutoEscape, Environment, UndefinedBehavior};
use serde::Serialize;
use serde_json::Value;

/// One email: its MJML body, subject line and sample data, embedded at compile time.
struct Template {
    name: &'static str,
    file: &'static str,
    body: &'static str,
    subject_file: &'static str,
    subject: &'static str,
    sample: &'static str,
}

macro_rules! template {
    ($name:literal) => {
        Template {
            name: $name,
            file: concat!($name, ".mjml"),
            body: include_str!(concat!("../templates/", $name, ".mjml")),
            subject_file: concat!($name, ".subject"),
            subject: include_str!(concat!("../templates/", $name, ".subject")),
            sample: include_str!(concat!("../templates/", $name, ".sample.json")),
        }
    };
}

/// Every email Linkdean sends. Add new templates here.
const TEMPLATES: &[Template] = &[template!("test")];
const LAYOUT: &str = include_str!("../templates/layout.mjml");
const MACROS: &str = include_str!("../templates/macros.mjml");

/// Values every template can use as `site.*`.
#[derive(Debug, Clone, Serialize)]
pub struct Site {
    pub app_url: String,
    pub instance: String,
}

#[derive(Debug, Clone)]
pub struct Rendered {
    pub subject: String,
    pub html: String,
    pub text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("unknown email template {0}")]
    Unknown(String),
    #[error("email template {0}: {1}")]
    Template(String, String),
    #[error("email template {0} is not valid MJML: {1}")]
    Mjml(String, String),
}

impl Site {
    pub fn new(app_url: &str) -> Self {
        todo!()
    }
}

pub fn template_names() -> impl Iterator<Item = &'static str> {
    TEMPLATES.iter().map(|t| t.name)
}

pub fn sample(name: &str) -> Option<Value> {
    todo!()
}

pub fn render(name: &str, data: &Value, site: &Site) -> Result<Rendered, RenderError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn site() -> Site {
        Site::new("https://links.example.com/")
    }

    #[test]
    fn site_names_the_instance_by_host() {
        let s = site();
        assert_eq!(s.app_url, "https://links.example.com");
        assert_eq!(s.instance, "links.example.com");
    }

    #[test]
    fn every_template_renders_its_sample() {
        for name in template_names() {
            let data = sample(name).unwrap_or_else(|| panic!("{name} has no sample"));
            let r = render(name, &data, &site()).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!r.subject.is_empty() && !r.subject.contains('\n'), "{name} subject");
            assert!(r.html.contains("<table"), "{name} html is MJML output");
            assert!(!r.text.trim().is_empty(), "{name} text");
        }
    }

    #[test]
    fn test_email_fills_variables_and_prints_the_link() {
        let r = render("test", &json!({ "recipient": "ada@example.com" }), &site()).unwrap();
        assert_eq!(r.subject, "Test email from links.example.com");
        assert!(r.html.contains("ada@example.com"));
        assert!(r.html.contains("Open Linkdean"));
        assert!(r.text.contains("https://links.example.com"));
    }

    #[test]
    fn a_missing_variable_fails() {
        let err = render("test", &json!({}), &site()).unwrap_err();
        assert!(matches!(err, RenderError::Template(..)), "{err}");
    }

    #[test]
    fn values_are_html_escaped() {
        let r = render("test", &json!({ "recipient": "<b>x</b>" }), &site()).unwrap();
        assert!(!r.html.contains("<b>x</b>"));
        assert!(r.html.contains("&lt;b&gt;x&lt;/b&gt;"));
    }

    #[test]
    fn an_unknown_template_fails() {
        assert!(matches!(
            render("nope", &json!({}), &site()),
            Err(RenderError::Unknown(_))
        ));
    }
}
```

Restore `pub use render::{render, sample, template_names, RenderError, Rendered, Site};` in `lib.rs`.

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -j 2 -p linkdean-mail render`
Expected: FAIL, panics at `not yet implemented`.

- [ ] **Step 4: Implement rendering**

Replace the three `todo!()` bodies:

```rust
impl Site {
    pub fn new(app_url: &str) -> Self {
        let app_url = app_url.trim_end_matches('/').to_string();
        let instance = url::Url::parse(&app_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| app_url.clone());
        Self { app_url, instance }
    }
}

pub fn sample(name: &str) -> Option<Value> {
    let t = TEMPLATES.iter().find(|t| t.name == name)?;
    serde_json::from_str(t.sample).ok()
}

pub fn render(name: &str, data: &Value, site: &Site) -> Result<Rendered, RenderError> {
    let t = TEMPLATES
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| RenderError::Unknown(name.into()))?;
    let fail = |e: minijinja::Error| RenderError::Template(name.into(), format!("{e:#}"));

    let mut env = Environment::new();
    // A typo in a variable name fails here, in tests, not as an empty gap in a sent email.
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    // MJML is markup, so values are escaped; the subject is plain text.
    env.set_auto_escape_callback(|file| {
        if file.ends_with(".subject") {
            AutoEscape::None
        } else {
            AutoEscape::Html
        }
    });
    env.add_template("layout.mjml", LAYOUT).map_err(fail)?;
    env.add_template("macros.mjml", MACROS).map_err(fail)?;
    env.add_template(t.file, t.body).map_err(fail)?;
    env.add_template(t.subject_file, t.subject).map_err(fail)?;

    let mut ctx = data.as_object().cloned().unwrap_or_default();
    ctx.insert(
        "site".into(),
        serde_json::to_value(site).expect("site serialises"),
    );
    let ctx = Value::Object(ctx);

    let mjml = env.get_template(t.file).map_err(fail)?.render(&ctx).map_err(fail)?;
    let subject = env
        .get_template(t.subject_file)
        .map_err(fail)?
        .render(&ctx)
        .map_err(fail)?
        .trim()
        .to_string();

    let mjml_err = |e: String| RenderError::Mjml(name.into(), e);
    let parsed = mrml::parse(&mjml).map_err(|e| mjml_err(e.to_string()))?;
    let html = parsed
        .element
        .render(&mrml::prelude::render::RenderOptions::default())
        .map_err(|e| mjml_err(e.to_string()))?;
    let text = html2text::from_read(html.as_bytes(), 78).map_err(|e| mjml_err(e.to_string()))?;
    Ok(Rendered {
        subject,
        html,
        text,
    })
}
```

If `mrml` 6.0.1's `parse` or `render` signatures differ from the two calls above, read `~/.cargo/registry/src/*/mrml-6.0.1/src/lib.rs` (its doc example shows parse → render) and change only those two calls.

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test -j 2 -p linkdean-mail`
Expected: 13 passed (7 config, 6 render). If `values_are_html_escaped` fails because mrml re-encodes entities differently (for example `&#60;`), assert instead that the HTML does not contain `<b>x</b>`, which is the property that matters.

- [ ] **Step 6: Format check and commit**

```bash
git add crates/mail
git commit -m "Render email from MJML templates.

Templates are embedded at compile time. minijinja fills them in strict
mode with HTML escaping, mrml turns the MJML into client-safe HTML, and
the plain-text part is generated from that HTML. Every template must
render its sample data in tests. Ships the Admin test email.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

---

### Task 3: SMTP delivery

**Files:**
- Create: `crates/mail/src/send.rs`, `crates/mail/tests/smtp.rs`
- Modify: `crates/mail/src/lib.rs` (restore the `send` re-export)

- [ ] **Step 1: Write the ignored delivery test**

Create `crates/mail/tests/smtp.rs`:

```rust
//! Delivers a real message to Mailpit. Ignored by default. Run with a throwaway Mailpit:
//!
//! docker run -d --rm --name ld-mailpit -p 127.0.0.1:11025:1025 -p 127.0.0.1:18025:8025 axllent/mailpit:v1.31.0
//! MAILPIT_SMTP_PORT=11025 MAILPIT_API=http://127.0.0.1:18025 cargo test -j 2 -p linkdean-mail --test smtp -- --ignored

use linkdean_mail::{render, MailConfig, Mailer, Site};
use serde_json::{json, Value};

#[tokio::test]
#[ignore = "needs a Mailpit container"]
async fn delivers_html_and_text_from_the_configured_sender() {
    let port = std::env::var("MAILPIT_SMTP_PORT").expect("MAILPIT_SMTP_PORT");
    let api = std::env::var("MAILPIT_API").expect("MAILPIT_API");
    let cfg = MailConfig::from_vars(|k| match k {
        "SMTP_HOST" => Some("127.0.0.1".into()),
        "SMTP_PORT" => Some(port.clone()),
        "SMTP_TLS" => Some("none".into()),
        "MAIL_FROM" => Some("Linkdean Test <linkdean@example.com>".into()),
        _ => None,
    })
    .unwrap()
    .unwrap();
    let to = format!("probe-{}@example.com", uuid::Uuid::new_v4().simple());
    let mail = render("test", &json!({ "recipient": to }), &Site::new("http://localhost:5173")).unwrap();

    Mailer::new(&cfg).unwrap().send(&to, &mail).await.expect("sent");

    let list: Value = reqwest::get(format!("{api}/api/v1/search?query=to:{to}"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let msg = &list["messages"][0];
    assert_eq!(msg["From"]["Address"], "linkdean@example.com");
    assert_eq!(msg["Subject"], "Test email from localhost");
    let id = msg["ID"].as_str().expect("message id");
    let full: Value = reqwest::get(format!("{api}/api/v1/message/{id}"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(full["HTML"].as_str().unwrap().contains(&to));
    assert!(full["Text"].as_str().unwrap().contains("http://localhost:5173"));
}
```

- [ ] **Step 2: Implement the mailer**

Create `crates/mail/src/send.rs`:

```rust
use crate::config::{MailConfig, TlsMode};
use crate::render::Rendered;
use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::time::Duration;

/// How long one SMTP conversation may take before it counts as a failed attempt.
const SMTP_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone)]
pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl Mailer {
    pub fn new(cfg: &MailConfig) -> anyhow::Result<Self> {
        let builder = match cfg.tls {
            TlsMode::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)?,
            TlsMode::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)?,
            // Config parsing only allows this for local servers.
            TlsMode::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.host),
        };
        let mut builder = builder.port(cfg.port).timeout(Some(SMTP_TIMEOUT));
        if let Some((user, pass)) = &cfg.credentials {
            builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
        }
        Ok(Self {
            transport: builder.build(),
            from: cfg.from.clone(),
        })
    }

    /// Deliver one message. The error text is what the Admin panel shows.
    pub async fn send(&self, to: &str, mail: &Rendered) -> Result<(), String> {
        let to: Mailbox = to
            .parse()
            .map_err(|e| format!("invalid recipient {to}: {e}"))?;
        let message = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(mail.subject.clone())
            .multipart(MultiPart::alternative_plain_html(
                mail.text.clone(),
                mail.html.clone(),
            ))
            .map_err(|e| e.to_string())?;
        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
```

Restore `pub use send::Mailer;` in `lib.rs`.

- [ ] **Step 3: Run the delivery test against a throwaway Mailpit**

```bash
docker run -d --rm --name ld-mailpit -p 127.0.0.1:11025:1025 -p 127.0.0.1:18025:8025 axllent/mailpit:v1.31.0
MAILPIT_SMTP_PORT=11025 MAILPIT_API=http://127.0.0.1:18025 cargo test -j 2 -p linkdean-mail --test smtp -- --ignored
docker stop ld-mailpit
```

Expected: 1 passed. Also run `cargo test -j 2 -p linkdean-mail`: 13 passed, 1 ignored.

- [ ] **Step 4: Format check and commit**

```bash
git add crates/mail
git commit -m "Deliver email over SMTP.

Mailer wraps lettre's async transport for the three TLS modes, sends
an HTML part with its plain-text alternative, and returns SMTP errors
as text for the Admin panel. An ignored test delivers to Mailpit.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

---

### Task 4: Outbox table and queries

**Files:**
- Create: `migrations/040_email_outbox.sql` (check `ls migrations | tail -1` first; if 040 is taken, use the next free number)
- Create: `crates/db/src/repo/email_outbox.rs`
- Modify: `crates/db/src/repo/mod.rs`
- Create: `crates/db/tests/email_outbox.rs`

- [ ] **Step 1: Write the migration**

```sql
-- Emails waiting to be sent, and a short history of sent and failed ones.
-- Bodies are rendered at enqueue time, so a later template change never alters a queued email.
CREATE TABLE email_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipient TEXT NOT NULL,
    template TEXT NOT NULL,
    subject TEXT NOT NULL,
    html TEXT NOT NULL,
    text TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'sent', 'failed')),
    attempts INT NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- A sender holds a row until this time; a crashed sender's rows come back after it.
    claimed_until TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX email_outbox_due ON email_outbox (next_attempt_at) WHERE status = 'queued';
CREATE INDEX email_outbox_status_updated ON email_outbox (status, updated_at);
```

- [ ] **Step 2: Write the failing database tests**

Create `crates/db/tests/email_outbox.rs`. Each test gets its own database because the queue is instance-wide (same pattern as `crates/db/tests/instance_races.rs`):

```rust
//! The email outbox: claiming, retries, giving up, stats and retention.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset. Each test runs in its own new
//! database, because claiming takes every due row in the instance.

use linkdean_db::{self as db, EmailRetry, NewOutboxEmail};
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

struct FreshDb {
    pool: PgPool,
    admin_url: String,
    name: String,
}

impl FreshDb {
    async fn drop_db(self) {
        self.pool.close().await;
        let mut admin = PgConnection::connect(&self.admin_url).await.expect("admin");
        sqlx::query(&format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", self.name))
            .execute(&mut admin)
            .await
            .expect("drop database");
    }
}

async fn fresh_db_or_skip() -> Option<FreshDb> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping email outbox tests: DATABASE_URL unset");
        return None;
    };
    let name = format!("mail_{}", Uuid::new_v4().simple());
    let mut admin = PgConnection::connect(&url).await.expect("admin");
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&mut admin)
        .await
        .expect("create database");
    let base = url.rsplit_once('/').expect("database in url").0;
    let pool = db::connect(&format!("{base}/{name}")).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(FreshDb { pool, admin_url: url, name })
}

fn email(to: &str) -> NewOutboxEmail {
    NewOutboxEmail {
        recipient: to.into(),
        template: "test".into(),
        subject: "Hello".into(),
        html: "<p>Hi</p>".into(),
        text: "Hi".into(),
    }
}

#[tokio::test]
async fn two_senders_never_claim_the_same_email() {
    let Some(fresh) = fresh_db_or_skip().await else { return };
    let pool = fresh.pool.clone();
    for i in 0..10 {
        db::enqueue_email(&pool, &email(&format!("u{i}@example.com"))).await.unwrap();
    }
    let (a, b) = tokio::join!(db::claim_due_emails(&pool, 10), db::claim_due_emails(&pool, 10));
    let mut ids: Vec<Uuid> = a.unwrap().into_iter().chain(b.unwrap()).map(|e| e.id).collect();
    assert_eq!(ids.len(), 10, "every email claimed once");
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 10, "no email claimed twice");
    // A claimed email is not handed out again while its claim lasts.
    assert!(db::claim_due_emails(&pool, 10).await.unwrap().is_empty());
    fresh.drop_db().await;
}

#[tokio::test]
async fn a_sent_email_is_done() {
    let Some(fresh) = fresh_db_or_skip().await else { return };
    let pool = fresh.pool.clone();
    let id = db::enqueue_email(&pool, &email("a@example.com")).await.unwrap();
    let claimed = db::claim_due_emails(&pool, 5).await.unwrap();
    assert_eq!(claimed[0].id, id);
    assert_eq!(claimed[0].recipient, "a@example.com");
    db::mark_email_sent(&pool, id).await.unwrap();
    sqlx::query("UPDATE email_outbox SET claimed_until = NULL, next_attempt_at = now() - interval '1 hour'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(db::claim_due_emails(&pool, 5).await.unwrap().is_empty());
    let stats = db::email_stats(&pool).await.unwrap();
    assert_eq!((stats.sent, stats.queued, stats.failed), (1, 0, 0));
    fresh.drop_db().await;
}

#[tokio::test]
async fn a_failure_backs_off_then_gives_up_after_a_day() {
    let Some(fresh) = fresh_db_or_skip().await else { return };
    let pool = fresh.pool.clone();
    let id = db::enqueue_email(&pool, &email("b@example.com")).await.unwrap();
    db::claim_due_emails(&pool, 5).await.unwrap();

    let retry = db::record_email_failure(&pool, id, "connection refused").await.unwrap();
    assert_eq!(retry, EmailRetry::Retrying);
    let (delay,): (f64,) = sqlx::query_as(
        "SELECT EXTRACT(EPOCH FROM next_attempt_at - now())::float8 FROM email_outbox WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!((25.0..=31.0).contains(&delay), "first retry after about 30 s, got {delay}");
    // Not due yet.
    assert!(db::claim_due_emails(&pool, 5).await.unwrap().is_empty());

    // Later attempts double, capped at an hour.
    sqlx::query("UPDATE email_outbox SET attempts = 9 WHERE id = $1").bind(id).execute(&pool).await.unwrap();
    db::record_email_failure(&pool, id, "connection refused").await.unwrap();
    let (delay,): (f64,) = sqlx::query_as(
        "SELECT EXTRACT(EPOCH FROM next_attempt_at - now())::float8 FROM email_outbox WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!((3590.0..=3601.0).contains(&delay), "capped at an hour, got {delay}");

    // A day after it was queued, the next failure is final.
    sqlx::query("UPDATE email_outbox SET created_at = now() - interval '25 hours' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let retry = db::record_email_failure(&pool, id, "mailbox unavailable").await.unwrap();
    assert_eq!(retry, EmailRetry::GaveUp);
    let stats = db::email_stats(&pool).await.unwrap();
    assert_eq!((stats.queued, stats.failed), (0, 1));
    assert_eq!(stats.last_error.as_deref(), Some("mailbox unavailable"));
    assert!(stats.last_error_at.is_some());
    fresh.drop_db().await;
}

#[tokio::test]
async fn a_crashed_senders_claim_expires() {
    let Some(fresh) = fresh_db_or_skip().await else { return };
    let pool = fresh.pool.clone();
    let id = db::enqueue_email(&pool, &email("c@example.com")).await.unwrap();
    db::claim_due_emails(&pool, 5).await.unwrap();
    sqlx::query("UPDATE email_outbox SET claimed_until = now() - interval '1 second' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let again = db::claim_due_emails(&pool, 5).await.unwrap();
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].id, id);
    fresh.drop_db().await;
}

#[tokio::test]
async fn old_rows_are_purged_by_status() {
    let Some(fresh) = fresh_db_or_skip().await else { return };
    let pool = fresh.pool.clone();
    let mut ids = Vec::new();
    for to in ["sent-old", "sent-new", "failed-old", "failed-mid", "queued-old"] {
        ids.push(db::enqueue_email(&pool, &email(&format!("{to}@example.com"))).await.unwrap());
    }
    let set = |id: Uuid, status: &'static str, days: i32| {
        let pool = pool.clone();
        async move {
            sqlx::query("UPDATE email_outbox SET status = $2, updated_at = now() - make_interval(days => $3) WHERE id = $1")
                .bind(id)
                .bind(status)
                .bind(days)
                .execute(&pool)
                .await
                .unwrap();
        }
    };
    set(ids[0], "sent", 8).await;
    set(ids[1], "sent", 1).await;
    set(ids[2], "failed", 31).await;
    set(ids[3], "failed", 8).await;
    set(ids[4], "queued", 40).await;

    assert_eq!(db::purge_old_emails(&pool).await.unwrap(), 2);
    let (left,): (i64,) = sqlx::query_as("SELECT count(*) FROM email_outbox").fetch_one(&pool).await.unwrap();
    assert_eq!(left, 3);
    fresh.drop_db().await;
}
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test cargo test -j 2 -p linkdean-db --test email_outbox`
Expected: compile errors, `enqueue_email` / `NewOutboxEmail` / `EmailRetry` not found.

- [ ] **Step 4: Implement the queries**

Create `crates/db/src/repo/email_outbox.rs`:

```rust
//! The email outbox. Request handlers enqueue; the API's sender claims, sends and settles.

use chrono::{DateTime, Utc};
use linkdean_core::{Error, Result};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

pub struct NewOutboxEmail {
    pub recipient: String,
    pub template: String,
    pub subject: String,
    pub html: String,
    pub text: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OutboxEmail {
    pub id: Uuid,
    pub recipient: String,
    pub subject: String,
    pub html: String,
    pub text: String,
    pub attempts: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailRetry {
    Retrying,
    GaveUp,
}

/// Sent and failed counts and the latest error cover the last 24 hours; queued is everything
/// still waiting.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EmailStats {
    pub sent: i64,
    pub queued: i64,
    pub failed: i64,
    pub last_error: Option<String>,
    pub last_error_at: Option<DateTime<Utc>>,
}

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

pub async fn enqueue_email(pool: &PgPool, email: &NewOutboxEmail) -> Result<Uuid> {
    let (id,): (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO email_outbox (recipient, template, subject, html, text)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(&email.recipient)
    .bind(&email.template)
    .bind(&email.subject)
    .bind(&email.html)
    .bind(&email.text)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;
    Ok(id)
}

/// Hand out up to `limit` due emails, each to one caller, for five minutes.
pub async fn claim_due_emails(pool: &PgPool, limit: i64) -> Result<Vec<OutboxEmail>> {
    sqlx::query_as::<_, OutboxEmail>(
        r#"
        UPDATE email_outbox
        SET claimed_until = now() + interval '5 minutes', updated_at = now()
        WHERE id IN (
            SELECT id FROM email_outbox
            WHERE status = 'queued'
              AND next_attempt_at <= now()
              AND (claimed_until IS NULL OR claimed_until < now())
            ORDER BY next_attempt_at
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, recipient, subject, html, text, attempts
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

pub async fn mark_email_sent(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE email_outbox
        SET status = 'sent', attempts = attempts + 1, claimed_until = NULL,
            last_error = NULL, updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

/// Schedule the next try (30 s, doubling, at most an hour apart), or give up once the email
/// has waited a day.
pub async fn record_email_failure(pool: &PgPool, id: Uuid, error: &str) -> Result<EmailRetry> {
    let (status,): (String,) = sqlx::query_as(
        r#"
        UPDATE email_outbox
        SET attempts = attempts + 1,
            last_error = $2,
            claimed_until = NULL,
            updated_at = now(),
            status = CASE WHEN created_at + interval '24 hours' <= now() THEN 'failed' ELSE 'queued' END,
            next_attempt_at = now() + LEAST(
                make_interval(secs => 30 * power(2, LEAST(attempts, 10))),
                interval '1 hour'
            )
        WHERE id = $1
        RETURNING status
        "#,
    )
    .bind(id)
    .bind(error)
    .fetch_optional(pool)
    .await
    .map_err(db_err)?
    .ok_or(Error::NotFound)?;
    Ok(if status == "failed" {
        EmailRetry::GaveUp
    } else {
        EmailRetry::Retrying
    })
}

pub async fn email_stats(pool: &PgPool) -> Result<EmailStats> {
    sqlx::query_as::<_, EmailStats>(
        r#"
        SELECT
            count(*) FILTER (WHERE status = 'sent' AND updated_at > now() - interval '24 hours') AS sent,
            count(*) FILTER (WHERE status = 'queued') AS queued,
            count(*) FILTER (WHERE status = 'failed' AND updated_at > now() - interval '24 hours') AS failed,
            latest.last_error,
            latest.updated_at AS last_error_at
        FROM email_outbox
        LEFT JOIN LATERAL (
            SELECT last_error, updated_at FROM email_outbox
            WHERE last_error IS NOT NULL AND updated_at > now() - interval '24 hours'
            ORDER BY updated_at DESC
            LIMIT 1
        ) latest ON TRUE
        GROUP BY latest.last_error, latest.updated_at
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(db_err)
    .map(|row| {
        row.unwrap_or(EmailStats {
            sent: 0,
            queued: 0,
            failed: 0,
            last_error: None,
            last_error_at: None,
        })
    })
}

/// Rows hold rendered bodies, which can carry one-time links, so they are not kept long.
pub async fn purge_old_emails(pool: &PgPool) -> Result<u64> {
    let done = sqlx::query(
        r#"
        DELETE FROM email_outbox
        WHERE (status = 'sent' AND updated_at < now() - interval '7 days')
           OR (status = 'failed' AND updated_at < now() - interval '30 days')
        "#,
    )
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(done.rows_affected())
}
```

In `crates/db/src/repo/mod.rs`, add `mod email_outbox;` in the alphabetical `mod` list (after `dashboards`) and `pub use email_outbox::*;` in the `pub use` list at the same position.

Note on `email_stats`: with an empty table the aggregate query returns no row because of `GROUP BY`, which the `unwrap_or` covers.

- [ ] **Step 5: Run the tests to see them pass**

Run: `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test cargo test -j 2 -p linkdean-db --test email_outbox`
Expected: 5 passed. Then check nothing was left behind:
`docker exec pg-check psql -U linkdean -d linkdean_test -tAc "SELECT count(*) FROM pg_database WHERE datname LIKE 'mail_%'"` → `0`.

- [ ] **Step 6: Format check and commit**

```bash
git add migrations/040_email_outbox.sql crates/db
git commit -m "Add the email outbox.

Emails are stored rendered and claimed with FOR UPDATE SKIP LOCKED
under a five-minute lease, so two senders never send the same email
and a crashed sender's emails come back. Failures retry after 30 s,
doubling to at most an hour, and give up a day after queueing. Sent
rows are purged after 7 days and failed ones after 30.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

---

### Task 5: Wire mail into the API

**Files:**
- Modify: `crates/api/Cargo.toml`, `crates/api/src/config/mod.rs`, `crates/api/src/state/mod.rs`, `crates/api/src/main.rs`, `crates/api/src/maintenance.rs`
- Create: `crates/api/src/mail.rs`

- [ ] **Step 1: Configuration**

`crates/api/Cargo.toml`: add `linkdean-mail.workspace = true` after `linkdean-db.workspace = true`.

`crates/api/src/config/mod.rs`: add to `Config` after `bootstrap_token`:

```rust
    /// SMTP settings. `None` keeps mail off and every feature that needs it on its fallback.
    pub mail: Option<linkdean_mail::MailConfig>,
    /// Development-only routes such as `/api/dev/emails`. On with `ALLOW_DEFAULT_SECRETS`.
    pub dev_routes: bool,
```

and in `from_env`'s `Ok(Self { … })`, after `bootstrap_token`:

```rust
            mail: linkdean_mail::MailConfig::from_vars(|k| std::env::var(k).ok())?,
            dev_routes: env_bool("ALLOW_DEFAULT_SECRETS", false),
```

If a test module in `config/mod.rs` builds a `Config` literal, add `mail: None, dev_routes: false` there.

- [ ] **Step 2: The mail service**

Create `crates/api/src/mail.rs`:

```rust
//! Queues rendered email and delivers it in the background.

use crate::state::AppState;
use linkdean_db::{self as db, EmailRetry, NewOutboxEmail};
use linkdean_mail::{render, MailConfig, Mailer, Rendered, Site};
use serde_json::Value;
use sqlx::PgPool;
use std::time::Duration;

/// How often the sender looks for due email.
const POLL: Duration = Duration::from_secs(5);
/// How many emails one pass claims.
const BATCH: i64 = 20;

pub struct Mail {
    pub cfg: MailConfig,
    pub mailer: Mailer,
    pub site: Site,
}

impl Mail {
    pub fn new(cfg: &MailConfig, app_url: &str) -> anyhow::Result<Self> {
        Ok(Self {
            cfg: cfg.clone(),
            mailer: Mailer::new(cfg)?,
            site: Site::new(app_url),
        })
    }
}

#[allow(dead_code)] // First used by password reset (outbound email part B).
#[derive(Debug, PartialEq, Eq)]
pub enum Queued {
    Queued,
    /// Mail is off. Callers that need email fall back to today's behaviour.
    Disabled,
}

/// Render `template` with `data` and queue it for `to`. Never sends inline.
#[allow(dead_code)] // First used by password reset (outbound email part B).
pub async fn enqueue(
    state: &AppState,
    to: &str,
    template: &str,
    data: &Value,
) -> linkdean_core::Result<Queued> {
    let Some(mail) = state.mail.as_deref() else {
        return Ok(Queued::Disabled);
    };
    let r = render(template, data, &mail.site).map_err(|e| linkdean_core::Error::msg(e.to_string()))?;
    db::enqueue_email(
        &state.pool,
        &NewOutboxEmail {
            recipient: to.into(),
            template: template.into(),
            subject: r.subject,
            html: r.html,
            text: r.text,
        },
    )
    .await?;
    Ok(Queued::Queued)
}

/// Deliver due email until the process ends. Returns at once when mail is off.
pub async fn run_sender(state: AppState) {
    let Some(mail) = state.mail.clone() else {
        return;
    };
    let mut tick = tokio::time::interval(POLL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        deliver_due(&state.pool, &mail.mailer).await;
    }
}

/// One pass: claim due email, send each, record the outcome. Returns how many it tried.
pub async fn deliver_due(pool: &PgPool, mailer: &Mailer) -> usize {
    let batch = match db::claim_due_emails(pool, BATCH).await {
        Ok(batch) => batch,
        Err(e) => {
            tracing::warn!(error = %e, "claiming queued email failed");
            return 0;
        }
    };
    for email in &batch {
        let rendered = Rendered {
            subject: email.subject.clone(),
            html: email.html.clone(),
            text: email.text.clone(),
        };
        match mailer.send(&email.recipient, &rendered).await {
            Ok(()) => {
                if let Err(e) = db::mark_email_sent(pool, email.id).await {
                    tracing::warn!(email_id = %email.id, error = %e, "marking email sent failed");
                }
            }
            Err(err) => match db::record_email_failure(pool, email.id, &err).await {
                Ok(EmailRetry::GaveUp) => {
                    tracing::warn!(email_id = %email.id, error = %err, "gave up sending email")
                }
                Ok(EmailRetry::Retrying) => {
                    tracing::info!(email_id = %email.id, attempt = email.attempts + 1, error = %err, "email send failed, will retry")
                }
                Err(e) => tracing::warn!(email_id = %email.id, error = %e, "recording email failure failed"),
            },
        }
    }
    batch.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs the throwaway Postgres and a throwaway Mailpit; see crates/mail/tests/smtp.rs.
    #[tokio::test]
    #[ignore = "needs Postgres and a Mailpit container"]
    async fn queued_email_is_delivered_and_marked_sent() {
        let pool = db::connect(&std::env::var("DATABASE_URL").unwrap()).await.unwrap();
        db::migrate(&pool).await.unwrap();
        let port = std::env::var("MAILPIT_SMTP_PORT").unwrap();
        let cfg = MailConfig::from_vars(|k| match k {
            "SMTP_HOST" => Some("127.0.0.1".into()),
            "SMTP_PORT" => Some(port.clone()),
            "SMTP_TLS" => Some("none".into()),
            "MAIL_FROM" => Some("a@example.com".into()),
            _ => None,
        })
        .unwrap()
        .unwrap();
        let mail = Mail::new(&cfg, "http://localhost:5173").unwrap();
        let to = format!("outbox-{}@example.com", uuid::Uuid::new_v4().simple());
        let r = render("test", &serde_json::json!({ "recipient": to }), &mail.site).unwrap();
        let id = db::enqueue_email(
            &pool,
            &NewOutboxEmail { recipient: to, template: "test".into(), subject: r.subject, html: r.html, text: r.text },
        )
        .await
        .unwrap();
        assert!(deliver_due(&pool, &mail.mailer).await >= 1);
        let (status,): (String,) = sqlx::query_as("SELECT status FROM email_outbox WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "sent");
        sqlx::query("DELETE FROM email_outbox WHERE id = $1").bind(id).execute(&pool).await.unwrap();
    }
}
```

- [ ] **Step 3: State, startup and maintenance**

`crates/api/src/state/mod.rs`: add the field `pub mail: Option<Arc<crate::mail::Mail>>,` after `limiter`, and in `new` before `Ok(Self {`:

```rust
        let mail = match &cfg.mail {
            Some(mail_cfg) => Some(Arc::new(crate::mail::Mail::new(mail_cfg, &cfg.app_url)?)),
            None => None,
        };
        match &mail {
            Some(m) => tracing::info!(host = %m.cfg.host, port = m.cfg.port, tls = m.cfg.tls.as_str(), "outbound email on"),
            None => tracing::info!("outbound email off: SMTP_HOST and MAIL_FROM unset"),
        }
```

and `mail,` in the struct literal.

`crates/api/src/main.rs`: add `mod mail;` to the module list (alphabetical, after `maintenance`), and after `tokio::spawn(maintenance::run(state.clone()));`:

```rust
    tokio::spawn(mail::run_sender(state.clone()));
```

`crates/api/src/maintenance.rs`: in `run_once`, after the unused tag reap block:

```rust
    match linkdean_db::purge_old_emails(&state.pool).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(emails = n, "purged old outbox email"),
        Err(e) => tracing::warn!(error = %e, "outbox purge failed"),
    }
```

Update the `INTERVAL` doc comment to list "the old email purge".

- [ ] **Step 4: Build and run the delivery test**

```bash
cargo build -j 2 -p linkdean-api
docker run -d --rm --name ld-mailpit -p 127.0.0.1:11025:1025 -p 127.0.0.1:18025:8025 axllent/mailpit:v1.31.0
DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test MAILPIT_SMTP_PORT=11025 cargo test -j 2 -p linkdean-api mail::tests -- --ignored
docker stop ld-mailpit
```

Expected: builds with no warnings; 1 passed. Add `uuid` to `crates/api` dev usage only if not already a dependency (it is: `uuid.workspace = true`).

- [ ] **Step 5: Full Rust suite, format, commit**

Run: `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test cargo test -j 2 --workspace` → all pass. Run the fmt check.

```bash
git add crates/api
git commit -m "Send queued email from the API.

AppState carries the mail service when SMTP is configured. A sender
loop claims due outbox rows every five seconds and records each
outcome, and hourly maintenance purges old rows. enqueue() renders a
template and queues it, or reports that mail is off.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

---

### Task 6: Admin email routes

**Files:**
- Create: `crates/api/src/routes/admin/email.rs`
- Modify: `crates/api/src/routes/admin/mod.rs`

- [ ] **Step 1: Write the routes**

Create `crates/api/src/routes/admin/email.rs`:

```rust
//! Settings → Admin → Email: configuration status, delivery counts and a test send.

use crate::auth::AdminUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_core::Error;
use linkdean_db::{self as db};
use serde_json::{json, Value};

/// Test sends per admin per rate-limit window (15 minutes).
const TEST_SENDS: u32 = 5;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/email", get(status))
        .route("/admin/email/test", post(send_test))
}

async fn status(State(state): State<AppState>, AdminUser(_admin): AdminUser) -> ApiResult<Json<Value>> {
    let stats = db::email_stats(&state.pool).await?;
    // Never the password.
    let config = state.mail.as_deref().map(|m| {
        json!({
            "host": m.cfg.host,
            "port": m.cfg.port,
            "tls": m.cfg.tls.as_str(),
            "from": m.cfg.from.to_string(),
        })
    });
    Ok(Json(json!({ "enabled": config.is_some(), "config": config, "stats": stats })))
}

/// Sends straight away, skipping the outbox, so the admin sees the SMTP result at once.
async fn send_test(State(state): State<AppState>, AdminUser(admin): AdminUser) -> ApiResult<Json<Value>> {
    let Some(mail) = state.mail.as_deref() else {
        return Err(Error::BadRequest("mail is off: set SMTP_HOST and MAIL_FROM".into()).into());
    };
    let limits = [(format!("mail-test:{}", admin.id), TEST_SENDS)];
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    let rendered = linkdean_mail::render("test", &json!({ "recipient": admin.email }), &mail.site)
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(Json(match mail.mailer.send(&admin.email, &rendered).await {
        Ok(()) => json!({ "ok": true, "to": admin.email }),
        Err(error) => json!({ "ok": false, "to": admin.email, "error": error }),
    }))
}
```

In `crates/api/src/routes/admin/mod.rs`: add `mod email;` at the top, and `.merge(email::router())` at the end of the `router()` chain.

If `state.limiter.check(&limits)?` does not convert into `ApiError`, follow how `crates/api/src/routes/auth` calls `limiter.check` and do the same.

- [ ] **Step 2: Build and smoke over HTTP**

Build, then run the API against the throwaway `smoke` database and a throwaway Mailpit (as in Task 5), with:

```
SMTP_HOST=127.0.0.1 SMTP_PORT=11025 SMTP_TLS=none MAIL_FROM="Linkdean Smoke <smoke@example.com>"
```

Sign in as the QA admin `uiqa-theme@smoke.test` / `uiqa-password-1` with curl (cookie jar, `X-Linkdean-Client: web` on POSTs) and check:

- `GET /api/admin/email` → `enabled: true`, `config.tls: "none"`, no password field anywhere in the body.
- `POST /api/admin/email/test` → `{"ok":true,"to":"uiqa-theme@smoke.test"}`, and Mailpit's `GET /api/v1/search?query=to:uiqa-theme@smoke.test` lists it.
- Six more POSTs → the sixth within 15 minutes answers 429.
- Restart the API without the SMTP vars: `GET` → `enabled: false, config: null`; `POST /test` → 400.
- As a non-admin user both routes answer 403.

- [ ] **Step 3: Format check and commit**

```bash
git add crates/api/src/routes/admin
git commit -m "Show email status to admins and let them send a test.

GET /api/admin/email reports whether mail is on, the server, TLS mode
and sender (never the password), and the last day's delivery counts
and latest error. POST /api/admin/email/test sends the test template
to the admin straight away and returns the SMTP result, five times per
15 minutes.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

---

### Task 7: Dev template previews

**Files:**
- Create: `crates/api/src/routes/dev.rs`
- Modify: `crates/api/src/routes/mod.rs`, `crates/api/src/main.rs`

- [ ] **Step 1: Write the routes**

Create `crates/api/src/routes/dev.rs`:

```rust
//! Development-only pages. Mounted only when `ALLOW_DEFAULT_SECRETS=true`.

use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use linkdean_core::Error;
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dev/emails", get(index))
        .route("/dev/emails/{name}", get(preview))
}

async fn index() -> Html<String> {
    let items: String = linkdean_mail::template_names()
        .map(|n| {
            format!(
                r#"<li><a href="/api/dev/emails/{n}">{n}</a> · <a href="/api/dev/emails/{n}?format=text">text</a></li>"#
            )
        })
        .collect();
    Html(format!(
        "<!doctype html><meta charset=utf-8><title>Email templates</title>\
         <body style=\"font-family:system-ui;padding:2rem\"><h1>Email templates</h1>\
         <p>Each renders with its <code>.sample.json</code>. Edit a template, wait for the API to rebuild, reload.</p>\
         <ul>{items}</ul></body>"
    ))
}

#[derive(Deserialize)]
struct PreviewQuery {
    format: Option<String>,
}

async fn preview(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<PreviewQuery>,
) -> ApiResult<Response> {
    let data = linkdean_mail::sample(&name).ok_or(Error::NotFound)?;
    let site = linkdean_mail::Site::new(&state.cfg.app_url);
    let r = linkdean_mail::render(&name, &data, &site).map_err(|e| Error::BadRequest(e.to_string()))?;
    Ok(if q.format.as_deref() == Some("text") {
        format!("Subject: {}\n\n{}", r.subject, r.text).into_response()
    } else {
        Html(r.html).into_response()
    })
}
```

`crates/api/src/routes/mod.rs`: add `mod dev;` to the module list and, below the `use` lines, `pub use dev::router as dev_router;`.

`crates/api/src/main.rs`: replace `.merge(routes::router())` in the `api` router with a prepared router:

```rust
    let mut api_routes = routes::router();
    if cfg.dev_routes {
        api_routes = api_routes.merge(routes::dev_router());
    }
    let api = Router::new()
        .merge(api_routes)
```

(keep the rest of the `api` chain unchanged).

- [ ] **Step 2: Check both modes**

Run the built API against the `smoke` database twice:
- With `ALLOW_DEFAULT_SECRETS=true`: `curl -s localhost:18080/api/dev/emails` lists `test`; `/api/dev/emails/test` returns HTML containing `ada@example.com`; `?format=text` starts with `Subject: Test email from`; `/api/dev/emails/nope` → 404.
- Without it (and with non-default secrets): `/api/dev/emails` → 404.

- [ ] **Step 3: Format check and commit**

```bash
git add crates/api
git commit -m "Preview email templates in development.

/api/dev/emails lists every template and renders each with its sample
data, as HTML or as the plain-text part. The routes exist only when
ALLOW_DEFAULT_SECRETS is on.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

---

### Task 8: Compose files

**Files:**
- Modify: `deploy/docker/docker-compose.dev.yml`, `deploy/docker/docker-compose.yml`

- [ ] **Step 1: Dev stack**

In `docker-compose.dev.yml`, add a service before `api`:

```yaml
  # Catches every email the dev API sends; nothing leaves the machine.
  # Web UI: http://127.0.0.1:8025
  mailpit:
    image: axllent/mailpit:v1.31.0
    networks: [mail]
    ports:
      - "127.0.0.1:8025:8025"
```

In `api`: change `networks: [data, browser, web]` to `networks: [data, browser, web, mail]`; add to `environment` after `BROWSER_WS_URL`:

```yaml
      SMTP_HOST: mailpit
      SMTP_PORT: "1025"
      SMTP_TLS: none
      MAIL_FROM: Linkdean Dev <linkdean@dev.localhost>
```

and to `depends_on`:

```yaml
      mailpit:
        condition: service_started
```

Add `mail:` under the top-level `networks:`.

- [ ] **Step 2: Production stack**

In `docker-compose.yml`, `api.environment`, after `BOOTSTRAP_TOKEN`:

```yaml
      SMTP_HOST: ${SMTP_HOST:-}
      SMTP_PORT: ${SMTP_PORT:-}
      SMTP_USERNAME: ${SMTP_USERNAME:-}
      SMTP_PASSWORD: ${SMTP_PASSWORD:-}
      SMTP_TLS: ${SMTP_TLS:-}
      MAIL_FROM: ${MAIL_FROM:-}
```

Empty values count as unset (Task 1 trims and drops empty strings), so mail stays off by default.

- [ ] **Step 3: Validate and bring up the dev stack**

```bash
POSTGRES_PASSWORD=x MEILI_MASTER_KEY=x SESSION_SECRET=xxxxxxxxxxxxxxxx docker compose -f deploy/docker/docker-compose.yml config >/dev/null && echo PROD_OK
docker compose -f deploy/docker/docker-compose.dev.yml config >/dev/null && echo DEV_OK
docker compose -f deploy/docker/docker-compose.dev.yml up -d mailpit api
```

Expected: `PROD_OK`, `DEV_OK`; after the API rebuilds, `docker compose -f deploy/docker/docker-compose.dev.yml logs api | grep "outbound email on"` shows `host=mailpit`, and `curl -s http://127.0.0.1:8025/livez` answers. This recreates the dev API container only; volumes stay.

- [ ] **Step 4: Commit**

```bash
git add deploy/docker
git commit -m "Run Mailpit in the dev stack and pass SMTP settings in production.

The dev API sends every email to Mailpit (UI on 127.0.0.1:8025) over
its own network. Production compose passes SMTP_* and MAIL_FROM
through, empty by default so mail stays off.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

---

### Task 9: Admin Email panel in the web app

**Files:**
- Create: `apps/web/src/schemas/mail/mail.ts`, `apps/web/src/schemas/mail/index.ts`
- Modify: `apps/web/src/schemas/index.ts`, `apps/web/src/api/api.ts`, `apps/web/src/hooks/settings/useAdminSettings.ts`, `apps/web/src/hooks/settings/index.ts`
- Create: `apps/web/src/pages/settings/AdminEmailPanel.tsx`, `apps/web/src/pages/settings/AdminEmailPanel.test.tsx`
- Modify: `apps/web/src/pages/settings/AdminPlatformSettings.tsx`

- [ ] **Step 1: Schemas, API and hooks**

`apps/web/src/schemas/mail/mail.ts`:

```ts
import { z } from 'zod'

export const emailStatusSchema = z.object({
  enabled: z.boolean(),
  config: z
    .object({
      host: z.string(),
      port: z.number(),
      tls: z.enum(['starttls', 'tls', 'none']),
      from: z.string(),
    })
    .nullable(),
  stats: z.object({
    sent: z.number(),
    queued: z.number(),
    failed: z.number(),
    last_error: z.string().nullable(),
    last_error_at: z.string().nullable(),
  }),
})

export type EmailStatus = z.infer<typeof emailStatusSchema>

export const testEmailResultSchema = z.object({
  ok: z.boolean(),
  to: z.string(),
  error: z.string().optional(),
})

export type TestEmailResult = z.infer<typeof testEmailResultSchema>
```

`apps/web/src/schemas/mail/index.ts`: `export * from './mail'`. In `apps/web/src/schemas/index.ts`, add `export * from './mail'` in alphabetical position.

`apps/web/src/api/api.ts`: add `emailStatusSchema` and `testEmailResultSchema` to the schema import list and `EmailStatus`, `TestEmailResult` to the type imports, then after `reindexSearch`:

```ts
  getEmailStatus: (): Promise<EmailStatus> => req('/admin/email', {}, emailStatusSchema),

  sendTestEmail: (): Promise<TestEmailResult> =>
    req('/admin/email/test', { method: 'POST' }, testEmailResultSchema),
```

`apps/web/src/hooks/settings/useAdminSettings.ts`: add `EmailStatus, TestEmailResult` to the `@/schemas` type import and append:

```ts
export function useEmailStatus(enabled = true): UseQueryResult<EmailStatus> {
  return useQuery({
    queryKey: ['admin', 'email'],
    queryFn: () => api.getEmailStatus(),
    enabled,
    refetchInterval: 30_000,
  })
}

export function useSendTestEmail(): UseMutationResult<TestEmailResult, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.sendTestEmail(),
    onSettled: () => void qc.invalidateQueries({ queryKey: ['admin', 'email'] }),
  })
}
```

Add `useEmailStatus, useSendTestEmail,` to the first `export { … } from './useAdminSettings'` list in `apps/web/src/hooks/settings/index.ts`.

- [ ] **Step 2: Write the failing panel test**

`apps/web/src/pages/settings/AdminEmailPanel.test.tsx`:

```tsx
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { EmailStatus, TestEmailResult } from '@/schemas'
import { AdminEmailPanel } from './AdminEmailPanel'

const state: {
  status: EmailStatus | undefined
  test: { isPending: boolean; data?: TestEmailResult; error: Error | null; mutate: () => void }
} = { status: undefined, test: { isPending: false, error: null, mutate: vi.fn() } }

vi.mock('@/hooks', () => ({
  useEmailStatus: () => ({ data: state.status, isError: false, error: null, refetch: vi.fn() }),
  useSendTestEmail: () => state.test,
}))

const on: EmailStatus = {
  enabled: true,
  config: { host: 'smtp.example.com', port: 587, tls: 'starttls', from: 'Linkdean <no-reply@example.com>' },
  stats: { sent: 12, queued: 1, failed: 2, last_error: '550 mailbox unavailable', last_error_at: '2026-11-03T10:00:00Z' },
}

function renderPanel(): void {
  render(
    <MantineProvider>
      <AdminEmailPanel />
    </MantineProvider>
  )
}

describe('AdminEmailPanel', () => {
  beforeEach(() => {
    state.test = { isPending: false, error: null, mutate: vi.fn() }
  })
  afterEach(cleanup)

  it('explains how to turn mail on when it is off', () => {
    state.status = { enabled: false, config: null, stats: { sent: 0, queued: 0, failed: 0, last_error: null, last_error_at: null } }
    renderPanel()
    expect(screen.getByText('Off')).toBeTruthy()
    expect(screen.getByText('SMTP_HOST')).toBeTruthy()
    expect(screen.queryByRole('button', { name: /send test email/i })).toBeNull()
  })

  it('shows the server, counts and latest error, and sends a test', async () => {
    state.status = on
    renderPanel()
    expect(screen.getByText('On')).toBeTruthy()
    expect(screen.getByText(/smtp\.example\.com:587/)).toBeTruthy()
    expect(screen.getByText('12')).toBeTruthy()
    expect(screen.getByText(/550 mailbox unavailable/)).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: /send test email/i }))
    expect(state.test.mutate).toHaveBeenCalledTimes(1)
  })

  it('shows the SMTP error of a failed test', () => {
    state.status = on
    state.test = { isPending: false, error: null, mutate: vi.fn(), data: { ok: false, to: 'a@example.com', error: 'connection refused' } }
    renderPanel()
    expect(screen.getByText(/connection refused/)).toBeTruthy()
  })

  it('confirms a sent test', () => {
    state.status = on
    state.test = { isPending: false, error: null, mutate: vi.fn(), data: { ok: true, to: 'a@example.com' } }
    renderPanel()
    expect(screen.getByText(/test email sent to a@example\.com/i)).toBeTruthy()
  })
})
```

Run: `cd apps/web && npx vitest run src/pages/settings/AdminEmailPanel.test.tsx` → FAIL, module not found.

- [ ] **Step 3: Write the panel**

`apps/web/src/pages/settings/AdminEmailPanel.tsx`:

```tsx
import { type ReactElement } from 'react'
import { Alert, Anchor, Badge, Button, Code, Group, Paper, SimpleGrid, Stack, Text } from '@mantine/core'
import { IconMail, IconSend } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components'
import { useEmailStatus, useSendTestEmail } from '@/hooks'

const SETUP_GUIDE = 'https://github.com/arsamsarabi/linkdean/blob/main/docs/SELF_HOSTED.md#email'

function Stat({ label, value }: { label: string; value: number }): ReactElement {
  return (
    <Stack gap={0}>
      <Text size="xl" fw={700}>
        {value}
      </Text>
      <Text size="xs" c="dimmed">
        {label}
      </Text>
    </Stack>
  )
}

export function AdminEmailPanel(): ReactElement {
  const status = useEmailStatus()
  const test = useSendTestEmail()

  if (status.isError) {
    return <QueryErrorAlert error={status.error} onRetry={() => void status.refetch()} title="Email status failed" />
  }

  return (
    <Paper p="xl" withBorder>
      <Stack gap="md">
        <Group justify="space-between">
          <Group gap="xs">
            <IconMail size={18} stroke={1.5} />
            <Text fw={500}>Email</Text>
          </Group>
          {status.data ? (
            <Badge color={status.data.enabled ? 'green' : 'gray'} variant="light">
              {status.data.enabled ? 'On' : 'Off'}
            </Badge>
          ) : null}
        </Group>

        {!status.data ? (
          <Text size="sm" c="dimmed">
            Loading email status…
          </Text>
        ) : status.data.enabled && status.data.config ? (
          <>
            <Text size="sm" c="dimmed">
              Sending as {status.data.config.from} through {status.data.config.host}:{status.data.config.port} (
              {status.data.config.tls}).
            </Text>
            <SimpleGrid cols={3}>
              <Stat label="Sent" value={status.data.stats.sent} />
              <Stat label="Queued" value={status.data.stats.queued} />
              <Stat label="Failed" value={status.data.stats.failed} />
            </SimpleGrid>
            <Text size="xs" c="dimmed">
              Sent and failed cover the last 24 hours. Queued is every email still waiting.
            </Text>
            {status.data.stats.last_error ? (
              <Alert color="red" title="Latest error">
                {status.data.stats.last_error}
                {status.data.stats.last_error_at
                  ? ` · ${new Date(status.data.stats.last_error_at).toLocaleString()}`
                  : ''}
              </Alert>
            ) : null}
            <Group>
              <Button
                leftSection={<IconSend size={16} stroke={1.5} />}
                loading={test.isPending}
                onClick={() => test.mutate()}
              >
                Send test email
              </Button>
            </Group>
            {test.data?.ok ? <Alert color="green">Test email sent to {test.data.to}.</Alert> : null}
            {test.data && !test.data.ok ? (
              <Alert color="red" title="Test email failed">
                {test.data.error}
              </Alert>
            ) : null}
            {test.error ? <Alert color="red">{test.error.message}</Alert> : null}
          </>
        ) : (
          <Text size="sm" c="dimmed">
            Mail is off. Set <Code>SMTP_HOST</Code> and{' '}
            <Code>MAIL_FROM</Code> (usually with <Code>SMTP_USERNAME</Code> and <Code>SMTP_PASSWORD</Code>), then restart
            Linkdean.{' '}
            <Anchor href={SETUP_GUIDE} target="_blank" rel="noreferrer">
              Email setup guide
            </Anchor>
          </Text>
        )}
      </Stack>
    </Paper>
  )
}
```

Parts B and C extend "Mail is off." with what is unavailable without it (password reset, email verification).

In `AdminPlatformSettings.tsx`, import `AdminEmailPanel` from `./AdminEmailPanel` and render `<AdminEmailPanel />` in the `left` fragment, right after `<AdminInvitesPanel />`.

- [ ] **Step 4: Run web checks**

From `apps/web`: `npx vitest run` (all pass, 4 new), `npx prettier --write src/pages/settings/AdminEmailPanel*.tsx src/schemas/mail src/api/api.ts src/hooks/settings` then `npx prettier --check src`, `npx oxlint src`, `npx tsc -b --noEmit`.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src
git commit -m "Add the Admin email panel.

Settings → Admin shows whether mail is on, which server and sender it
uses, the last day's sent, queued and failed counts with the latest
error, and a Send test email button that shows the SMTP result. When
mail is off it names the variables that turn it on.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

---

### Task 10: Docs

**Files:**
- Modify: `README.md`, `docs/SELF_HOSTED.md`, `docs/ARCHITECTURE.md`, `docs/FEATURES.md`, `docs/ROADMAP.md`

Follow the `linkdean-self-host-docs` skill: derive every value from code, and do not Prettier-reformat a doc unless `git show HEAD:<file>` was already Prettier-clean.

- [ ] **Step 1: README**

Configuration table: add rows after `BOOTSTRAP_TOKEN`:

```markdown
| `SMTP_HOST` | no | Mail server. Mail is on when this and `MAIL_FROM` are set; off, nothing is emailed |
| `SMTP_PORT` | no | Default 587, or 465 with `SMTP_TLS=tls` |
| `SMTP_USERNAME` / `SMTP_PASSWORD` | no | Set both or neither |
| `SMTP_TLS` | no | `starttls` (default), `tls`, or `none` (local servers only) |
| `MAIL_FROM` | no | Sender, e.g. `Linkdean <no-reply@example.com>` |
```

Services table: add `| mailpit | Dev only: catches outgoing email | Dev: 127.0.0.1:8025 |` matching the table's existing columns.

- [ ] **Step 2: SELF_HOSTED.md**

Add an `## Email` section (anchor `#email`, used by the Admin panel link) under Production deployment:

- Mail is optional; without it the instance runs as before.
- The variables and the startup refusals (from `crates/mail/src/config.rs`).
- Examples:
  - Gmail: `SMTP_HOST=smtp.gmail.com`, port 587, STARTTLS, an app password (needs 2-Step Verification), `MAIL_FROM` must be the Gmail address or a verified alias.
  - Fastmail: `smtp.fastmail.com`, 465 with `SMTP_TLS=tls`, an app password.
  - Postmark: `smtp.postmarkapp.com`, 587, username and password both the server API token, sender signature verified.
- Check it works: Settings → Admin → Email → Send test email.
- Delivery: queued, retried for a day, sent rows kept 7 days and failed 30.
- SPF, DKIM and DMARC are set up with the mail provider, not in Linkdean.

- [ ] **Step 3: ARCHITECTURE.md, FEATURES.md, ROADMAP.md**

ARCHITECTURE: add an `## Email` section: `crates/mail` (MJML via mrml, minijinja strict, html2text, lettre), outbox table, sender loop in the API (5 s poll, 20 per pass, five-minute claim, backoff 30 s doubling to 1 h, give up after 24 h), retention in hourly maintenance, `/api/admin/email`, dev-only `/api/dev/emails`. Add `crates/mail` wherever the crates are listed.

FEATURES: under Ops & self-host add: "Outbound email over SMTP (optional): an outbox with retries, Settings → Admin → Email with status, delivery counts and a test send; Mailpit catches dev email and `/api/dev/emails` previews templates".

ROADMAP item 1: replace "SMTP config with [react-email](…) templates." with "The mail foundation ships (SMTP, MJML templates, outbox, Admin panel). Still to build on it:" and keep the five sub-bullets.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/SELF_HOSTED.md docs/ARCHITECTURE.md docs/FEATURES.md docs/ROADMAP.md
git commit -m "Document outbound email.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

---

### Task 11: Browser check and close-out

- [ ] **Step 1: Browser flow** (skill `browser-ui-verification`)

With the dev stack up (Task 8), use the Playwright scratch install at `/tmp/claude-502/ldsmoke/pw`. Point it at the dev web app only for reading; do not create data in the dev database. Instead run the built API against the throwaway `smoke` database with Mailpit vars and Vite on :5199 as in earlier smoke runs. Flow, as `uiqa-theme@smoke.test`:

1. Open `/settings?tab=admin`; the Email panel shows On, the Mailpit host and the sender.
2. Click Send test email; "Test email sent to uiqa-theme@smoke.test." appears.
3. Mailpit API lists the message for that address, with HTML and text parts.
4. Restart the API without SMTP vars, reload: the panel shows Off and names `SMTP_HOST`.
5. Count responses ≥ 400 and console errors; both empty or explained. Take one screenshot of the panel and look at it.

- [ ] **Step 2: Full checks**

Rust suite, fmt, web suite, prettier, oxlint, tsc, as in the repo rules. Stop the smoke servers and the throwaway Mailpit.

- [ ] **Step 3: Close out**

Run `graphify update .`. Report what shipped, the commits, and the next sub-project (B, password reset), which gets its own spec.
