//! Outbound email: configuration, MJML templates and SMTP delivery.

mod config;
mod render;
mod send;

pub use config::{MailConfig, TlsMode};
pub use render::{render, sample, template_names, RenderError, Rendered, Site};
// pub use send::Mailer;
