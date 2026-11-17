//! Archived copies of bookmarks: formats, queue status and what the API returns about them.

use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Most bookmarks one bulk archive request may name.
pub const MAX_BULK_ARCHIVE_IDS: usize = 500;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFormat {
    /// One HTML file with its images, styles and fonts inlined.
    Html,
    Pdf,
    /// The URL itself was a file (a PDF, image, audio or video), kept as served.
    File,
}

impl ArchiveFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Pdf => "pdf",
            Self::File => "file",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "html" => Ok(Self::Html),
            "pdf" => Ok(Self::Pdf),
            "file" => Ok(Self::File),
            _ => Err(Error::BadRequest(format!("unknown archive format: {s}"))),
        }
    }

    /// Binds an encrypted copy to its format, so one cannot be served as another.
    pub fn aad_tag(self) -> u8 {
        match self {
            Self::Html => b'h',
            Self::Pdf => b'p',
            Self::File => b'f',
        }
    }
}

/// The formats a person may ask for: HTML and PDF, at least one, each once, in a fixed order.
pub fn clean_archive_formats(formats: &[ArchiveFormat]) -> Result<Vec<ArchiveFormat>> {
    let mut out: Vec<ArchiveFormat> = formats.to_vec();
    out.sort();
    out.dedup();
    if out.is_empty() {
        return Err(Error::BadRequest("pick at least one archive format".into()));
    }
    if out.contains(&ArchiveFormat::File) {
        return Err(Error::BadRequest(
            "archive formats are html and pdf; files are kept as they are".into(),
        ));
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveStatus {
    Queued,
    Running,
    Done,
    Failed,
}

impl ArchiveStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            _ => Err(Error::msg(format!("unknown archive status: {s}"))),
        }
    }
}

/// One copy of a bookmark, stored or on its way.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BookmarkArchiveView {
    pub format: ArchiveFormat,
    pub status: ArchiveStatus,
    /// Size of the stored copy; 0 when there is none.
    pub bytes: i64,
    pub content_type: Option<String>,
    /// When the stored copy was taken.
    pub captured_at: Option<DateTime<Utc>>,
    /// Why the latest attempt failed. Set with `available` when an earlier copy is kept.
    pub error: Option<String>,
    /// Whether a copy can be opened now.
    pub available: bool,
}

/// What a bookmark card needs to know about its copies.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArchiveSummary {
    /// Formats with a stored copy.
    pub formats: Vec<ArchiveFormat>,
    /// When the newest stored copy was taken.
    pub captured_at: Option<DateTime<Utc>>,
    /// Some format is queued or being captured.
    pub pending: bool,
    /// The latest attempt of some format failed.
    pub failed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RequestArchive {
    /// `html`, `pdf` or both. A URL that serves a file is kept as that file instead.
    pub formats: Vec<ArchiveFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkArchive {
    pub ids: Vec<Uuid>,
    pub formats: Vec<ArchiveFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkArchiveFailure {
    pub id: Uuid,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkArchiveResult {
    /// Bookmarks queued.
    pub ok: usize,
    pub failed: Vec<BulkArchiveFailure>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_are_cleaned() {
        use ArchiveFormat::*;
        assert_eq!(
            clean_archive_formats(&[Pdf, Html, Pdf]).unwrap(),
            vec![Html, Pdf]
        );
        assert!(clean_archive_formats(&[]).is_err());
        assert!(clean_archive_formats(&[File]).is_err());
    }

    #[test]
    fn names_round_trip() {
        for f in [ArchiveFormat::Html, ArchiveFormat::Pdf, ArchiveFormat::File] {
            assert_eq!(ArchiveFormat::parse(f.as_str()).unwrap(), f);
        }
        for s in [
            ArchiveStatus::Queued,
            ArchiveStatus::Running,
            ArchiveStatus::Done,
            ArchiveStatus::Failed,
        ] {
            assert_eq!(ArchiveStatus::parse(s.as_str()).unwrap(), s);
        }
    }
}
