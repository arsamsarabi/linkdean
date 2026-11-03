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
        let app_url = app_url.trim_end_matches('/').to_string();
        let instance = url::Url::parse(&app_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| app_url.clone());
        Self { app_url, instance }
    }
}

pub fn template_names() -> impl Iterator<Item = &'static str> {
    TEMPLATES.iter().map(|t| t.name)
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

    let mjml = env
        .get_template(t.file)
        .map_err(fail)?
        .render(&ctx)
        .map_err(fail)?;
    let subject = env
        .get_template(t.subject_file)
        .map_err(fail)?
        .render(&ctx)
        .map_err(fail)?
        .trim()
        .to_string();

    let mjml_err = |e: String| RenderError::Mjml(name.into(), e);
    let parsed = mrml::parse(&mjml).map_err(|e| mjml_err(e.to_string()))?;
    // mrml links Google Fonts for Roboto by default; an empty font map keeps the open private.
    let options = mrml::prelude::render::RenderOptions {
        fonts: Default::default(),
        ..Default::default()
    };
    let html = parsed
        .element
        .render(&options)
        .map_err(|e| mjml_err(e.to_string()))?;
    let text = html2text::from_read(html.as_bytes(), 78).map_err(|e| mjml_err(e.to_string()))?;
    Ok(Rendered {
        subject,
        html,
        text,
    })
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
            assert!(
                !r.subject.is_empty() && !r.subject.contains('\n'),
                "{name} subject"
            );
            assert!(r.html.contains("<table"), "{name} html is MJML output");
            assert!(
                !r.html.contains("fonts.googleapis"),
                "{name} loads no web fonts"
            );
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
        // minijinja also escapes `/` (as `&#x2f;`), so only the opening tag is checked.
        assert!(r.html.contains("&lt;b&gt;x"));
    }

    #[test]
    fn an_unknown_template_fails() {
        assert!(matches!(
            render("nope", &json!({}), &site()),
            Err(RenderError::Unknown(_))
        ));
    }
}
