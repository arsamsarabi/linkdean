//! API key permission scopes (`resource:op`).

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Every scope Phase A knows about.
pub const ALL_SCOPES: &[&str] = &[
    "library:read",
    "library:write",
    "archive:read",
    "archive:write",
    "import_export:read",
    "import_export:write",
    "social:read",
    "social:write",
    "account:read",
    "account:write",
];

/// Named create-time template (Phase B). UI may also offer `custom`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiKeyTemplate {
    pub id: &'static str,
    pub label: &'static str,
    pub scopes: &'static [&'static str],
}

/// Presets when creating a key. Keep web `ApiKeysPanel` in sync.
pub const API_KEY_TEMPLATES: &[ApiKeyTemplate] = &[
    ApiKeyTemplate {
        id: "read_only_library",
        label: "Read-only library",
        scopes: &["library:read"],
    },
    ApiKeyTemplate {
        id: "save_bookmarks",
        label: "Save bookmarks",
        scopes: &["library:read", "library:write"],
    },
    ApiKeyTemplate {
        id: "full_access",
        label: "Full access",
        scopes: ALL_SCOPES,
    },
];

/// If `scopes` exactly matches a named template (order-independent), return its id.
pub fn matching_template(scopes: &[String]) -> Option<&'static str> {
    let set: BTreeSet<&str> = scopes.iter().map(String::as_str).collect();
    for t in API_KEY_TEMPLATES {
        let want: BTreeSet<&str> = t.scopes.iter().copied().collect();
        if set == want {
            return Some(t.id);
        }
    }
    None
}

/// Validated set of scopes for one API key.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiScopes(BTreeSet<String>);

impl ApiScopes {
    pub fn all() -> Self {
        Self(ALL_SCOPES.iter().map(|s| (*s).to_string()).collect())
    }

    pub fn from_strings(raw: &[String]) -> Result<Self> {
        if raw.is_empty() {
            return Err(Error::BadRequest("at least one scope is required".into()));
        }
        let mut set = BTreeSet::new();
        for s in raw {
            let t = s.trim();
            if t.is_empty() {
                continue;
            }
            if !ALL_SCOPES.contains(&t) {
                return Err(Error::BadRequest(format!("unknown scope `{t}`")));
            }
            set.insert(t.to_string());
        }
        if set.is_empty() {
            return Err(Error::BadRequest("at least one scope is required".into()));
        }
        Ok(Self(set))
    }

    pub fn contains(&self, scope: &str) -> bool {
        self.0.contains(scope)
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    pub fn to_vec(&self) -> Vec<String> {
        self.0.iter().cloned().collect()
    }
}

impl FromIterator<String> for ApiScopes {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// Which scope an authenticated API-key request needs, if any.
///
/// `None` means the path is exempt (auth/health/public) or unmapped.
/// Callers treat unmapped authenticated paths as deny when a key is present.
pub fn required_scope(method: &str, path: &str) -> ScopeRequirement {
    let path = path.trim_end_matches('/');
    let write = !matches!(
        method.to_ascii_uppercase().as_str(),
        "GET" | "HEAD" | "OPTIONS"
    );

    if path_exempt(path) {
        return ScopeRequirement::Exempt;
    }

    // Archive routes live under /bookmarks/…/archive — check before library.
    if is_archive_path(path) {
        return ScopeRequirement::Need(if write {
            "archive:write"
        } else {
            "archive:read"
        });
    }

    if path.starts_with("/api/mcp") {
        return ScopeRequirement::AnyOf(&["library:read", "archive:read"]);
    }

    let resource = resource_for_path(path);
    match resource {
        Some(res) => ScopeRequirement::Need(if write {
            match res {
                "library" => "library:write",
                "archive" => "archive:write",
                "import_export" => "import_export:write",
                "social" => "social:write",
                "account" => "account:write",
                _ => return ScopeRequirement::DenyUnmapped,
            }
        } else {
            match res {
                "library" => "library:read",
                "archive" => "archive:read",
                "import_export" => "import_export:read",
                "social" => "social:read",
                "account" => "account:read",
                _ => return ScopeRequirement::DenyUnmapped,
            }
        }),
        None => ScopeRequirement::DenyUnmapped,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeRequirement {
    Exempt,
    Need(&'static str),
    AnyOf(&'static [&'static str]),
    DenyUnmapped,
}

fn path_exempt(path: &str) -> bool {
    path.starts_with("/api/auth/")
        || path.starts_with("/auth/")
        || path.starts_with("/api/health")
        || path.starts_with("/api/public")
        || path.starts_with("/api/metrics")
        || path == "/api/openapi.json"
        || path.starts_with("/api/email/")
}

fn is_archive_path(path: &str) -> bool {
    // /api/bookmarks/{id}/archive… or /api/bookmarks/archive (bulk)
    if let Some(rest) = path.strip_prefix("/api/bookmarks/") {
        return rest == "archive" || rest.contains("/archive");
    }
    false
}

fn resource_for_path(path: &str) -> Option<&'static str> {
    const RULES: &[(&str, &str)] = &[
        ("/api/bookmarks", "library"),
        ("/api/categories", "library"),
        ("/api/tags", "library"),
        ("/api/search", "library"),
        ("/api/trash", "library"),
        ("/api/dashboards", "library"),
        ("/api/activity", "library"),
        ("/api/import", "import_export"),
        ("/api/export", "import_export"),
        ("/api/agent-skill", "import_export"),
        ("/api/social", "social"),
        ("/api/share-invites", "social"),
        ("/api/users", "account"),
        ("/api/notifications", "account"),
        ("/api/vault", "account"),
        ("/api/reports", "account"),
        ("/api/api-keys", "account"), // SessionUser only in practice
    ];
    for (prefix, res) in RULES {
        if path == *prefix || path.starts_with(&format!("{prefix}/")) {
            return Some(res);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_path_beats_library() {
        assert_eq!(
            required_scope("GET", "/api/bookmarks/x/archive/html"),
            ScopeRequirement::Need("archive:read")
        );
        assert_eq!(
            required_scope("POST", "/api/bookmarks/archive"),
            ScopeRequirement::Need("archive:write")
        );
    }

    #[test]
    fn library_read_write() {
        assert_eq!(
            required_scope("GET", "/api/bookmarks"),
            ScopeRequirement::Need("library:read")
        );
        assert_eq!(
            required_scope("POST", "/api/bookmarks"),
            ScopeRequirement::Need("library:write")
        );
    }

    #[test]
    fn mcp_any_of_reads() {
        assert_eq!(
            required_scope("POST", "/api/mcp"),
            ScopeRequirement::AnyOf(&["library:read", "archive:read"])
        );
    }

    #[test]
    fn rejects_unknown_scope() {
        assert!(ApiScopes::from_strings(&["nope:read".into()]).is_err());
    }

    #[test]
    fn all_catalogue_parses() {
        let v: Vec<String> = ALL_SCOPES.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(ApiScopes::from_strings(&v).unwrap(), ApiScopes::all());
    }

    #[test]
    fn templates_parse_and_match() {
        for t in API_KEY_TEMPLATES {
            let v: Vec<String> = t.scopes.iter().map(|s| (*s).to_string()).collect();
            assert!(ApiScopes::from_strings(&v).is_ok(), "{}", t.id);
            assert_eq!(matching_template(&v), Some(t.id));
        }
        let custom = vec!["library:read".into(), "archive:read".into()];
        assert!(ApiScopes::from_strings(&custom).is_ok());
        assert_eq!(matching_template(&custom), None);
        // Order must not matter.
        let shuffled = vec!["library:write".into(), "library:read".into()];
        assert_eq!(matching_template(&shuffled), Some("save_bookmarks"));
    }
}
