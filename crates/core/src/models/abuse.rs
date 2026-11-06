//! Abuse reports: what a signed-in user may file and what an admin may do with it.

use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Reasons the report form offers, in display order.
pub const ABUSE_REASONS: &[&str] = &["spam", "harassment", "illegal", "impersonation", "other"];
pub const MAX_ABUSE_DETAILS_CHARS: usize = 2000;
pub const MAX_ABUSE_NOTE_CHARS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbuseTargetType {
    PublicList,
    Profile,
    Comment,
}

impl AbuseTargetType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PublicList => "public_list",
            Self::Profile => "profile",
            Self::Comment => "comment",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbuseAction {
    Dismiss,
    Hide,
    Warn,
    Ban,
}

impl AbuseAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dismiss => "dismiss",
            Self::Hide => "hide",
            Self::Warn => "warn",
            Self::Ban => "ban",
        }
    }
}

/// `target_id` is a public list's slug or category id, a user id, or a comment id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAbuseReport {
    pub target_type: AbuseTargetType,
    pub target_id: String,
    pub reason: String,
    pub details: Option<String>,
}

impl CreateAbuseReport {
    /// The trimmed target id, a known reason and the details (None when blank), or `BadRequest`.
    pub fn validate(&self) -> Result<(String, String, Option<String>)> {
        let target_id = self.target_id.trim();
        if target_id.is_empty() || target_id.chars().count() > 200 {
            return Err(Error::BadRequest("target_id is required".into()));
        }
        let reason = self.reason.trim();
        if !ABUSE_REASONS.contains(&reason) {
            return Err(Error::BadRequest(format!(
                "reason must be one of: {}",
                ABUSE_REASONS.join(", ")
            )));
        }
        let details = self
            .details
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty());
        if details.is_some_and(|d| d.chars().count() > MAX_ABUSE_DETAILS_CHARS) {
            return Err(Error::BadRequest(format!(
                "details must be at most {MAX_ABUSE_DETAILS_CHARS} characters"
            )));
        }
        Ok((
            target_id.to_string(),
            reason.to_string(),
            details.map(str::to_string),
        ))
    }
}

/// A trimmed, plausible address for the abuse contact; blank clears it.
pub fn clean_abuse_contact_email(email: Option<&str>) -> Result<Option<String>> {
    let Some(email) = email.map(str::trim).filter(|e| !e.is_empty()) else {
        return Ok(None);
    };
    let ok = email.chars().count() <= 254
        && !email.chars().any(char::is_whitespace)
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    if !ok {
        return Err(Error::BadRequest(
            "abuse contact must be an email address".into(),
        ));
    }
    Ok(Some(email.to_string()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveAbuseReport {
    pub action: AbuseAction,
    pub note: Option<String>,
}

impl ResolveAbuseReport {
    /// The trimmed note (None when blank). `warn` needs one: it is what the user is told.
    pub fn validate(&self) -> Result<Option<String>> {
        let note = self
            .note
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty());
        if note.is_some_and(|n| n.chars().count() > MAX_ABUSE_NOTE_CHARS) {
            return Err(Error::BadRequest(format!(
                "note must be at most {MAX_ABUSE_NOTE_CHARS} characters"
            )));
        }
        if self.action == AbuseAction::Warn && note.is_none() {
            return Err(Error::BadRequest("a warning needs a note".into()));
        }
        Ok(note.map(str::to_string))
    }
}

/// One report as admins see it.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AbuseReport {
    pub id: Uuid,
    pub reporter_user_id: Uuid,
    pub reporter_email: Option<String>,
    pub target_type: String,
    pub target_id: String,
    pub target_user_id: Option<Uuid>,
    pub target_label: String,
    pub reason: String,
    pub details: Option<String>,
    pub status: String,
    pub resolution_action: Option<String>,
    pub resolution_note: Option<String>,
    pub resolved_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(reason: &str, details: Option<&str>) -> CreateAbuseReport {
        CreateAbuseReport {
            target_type: AbuseTargetType::Comment,
            target_id: " abc ".into(),
            reason: reason.into(),
            details: details.map(str::to_string),
        }
    }

    #[test]
    fn create_report_trims_and_checks_inputs() {
        let (id, reason, details) = report("spam", Some("  ")).validate().unwrap();
        assert_eq!(
            (id.as_str(), reason.as_str(), details),
            ("abc", "spam", None)
        );
        assert!(report("rude", None).validate().is_err());
        let long = "x".repeat(MAX_ABUSE_DETAILS_CHARS + 1);
        assert!(report("spam", Some(&long)).validate().is_err());
        let mut blank = report("spam", None);
        blank.target_id = "   ".into();
        assert!(blank.validate().is_err());
    }

    #[test]
    fn contact_email_is_trimmed_checked_or_cleared() {
        assert_eq!(clean_abuse_contact_email(None).unwrap(), None);
        assert_eq!(clean_abuse_contact_email(Some("  ")).unwrap(), None);
        assert_eq!(
            clean_abuse_contact_email(Some(" abuse@example.com "))
                .unwrap()
                .as_deref(),
            Some("abuse@example.com")
        );
        assert!(clean_abuse_contact_email(Some("nope")).is_err());
        assert!(clean_abuse_contact_email(Some("a b@example.com")).is_err());
    }

    #[test]
    fn a_warning_needs_a_note() {
        let warn = |note: Option<&str>| ResolveAbuseReport {
            action: AbuseAction::Warn,
            note: note.map(str::to_string),
        };
        assert!(warn(None).validate().is_err());
        assert!(warn(Some("  ")).validate().is_err());
        assert_eq!(
            warn(Some(" stop ")).validate().unwrap().as_deref(),
            Some("stop")
        );
    }
}
