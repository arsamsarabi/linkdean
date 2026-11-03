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
        // Some servers add no Message-ID, and spam filters score its absence.
        let message_id = format!(
            "<{}@{}>",
            uuid::Uuid::new_v4().simple(),
            self.from.email.domain()
        );
        let message = Message::builder()
            .message_id(Some(message_id))
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
