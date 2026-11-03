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
    let mail = render(
        "test",
        &json!({ "recipient": to }),
        &Site::new("http://localhost:5173"),
    )
    .unwrap();

    Mailer::new(&cfg)
        .unwrap()
        .send(&to, &mail)
        .await
        .expect("sent");

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
    assert!(full["Text"]
        .as_str()
        .unwrap()
        .contains("http://localhost:5173"));
}
