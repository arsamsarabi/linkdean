//! Managed Free/Pro personal meters. See `docs/design/plans-and-pricing.md`.

use crate::ResolvedLimits;

/// Personal plan on the managed service. Self-host leaves `users.plan` unset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AccountPlan {
    Free,
    Pro,
}

impl AccountPlan {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "free" => Some(Self::Free),
            "pro" => Some(Self::Pro),
            _ => None,
        }
    }

    /// Plan ceilings from the design table. `None` = unlimited for that meter.
    pub fn limits(self) -> ResolvedLimits {
        match self {
            Self::Free => ResolvedLimits {
                max_bookmarks: Some(5_000),
                max_stored_image_bytes: Some(1_073_741_824), // 1 GiB
                max_enrich_actions_per_day: Some(500),
                max_import_bytes: None,
                max_upload_image_bytes: None,
                max_archive_bytes: Some(524_288_000), // 500 MiB
                max_vaults: Some(1),
            },
            Self::Pro => ResolvedLimits {
                max_bookmarks: None,
                max_stored_image_bytes: Some(10_737_418_240), // 10 GiB
                max_enrich_actions_per_day: Some(5_000),
                max_import_bytes: None,
                max_upload_image_bytes: None,
                max_archive_bytes: Some(26_843_545_600), // 25 GiB
                max_vaults: None,
            },
        }
    }
}

/// Tighter of two optional ceilings. Either side unlimited → the other; both unlimited → unlimited.
pub fn tighter(a: Option<i64>, b: Option<i64>) -> Option<i64> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

/// Effective limits = merge of platform row and optional plan (design Phase B).
pub fn merge_limits(platform: ResolvedLimits, plan: Option<AccountPlan>) -> ResolvedLimits {
    let Some(plan) = plan else {
        return platform;
    };
    let p = plan.limits();
    ResolvedLimits {
        max_bookmarks: tighter(platform.max_bookmarks, p.max_bookmarks),
        max_stored_image_bytes: tighter(platform.max_stored_image_bytes, p.max_stored_image_bytes),
        max_enrich_actions_per_day: tighter(
            platform.max_enrich_actions_per_day,
            p.max_enrich_actions_per_day,
        ),
        max_import_bytes: tighter(platform.max_import_bytes, p.max_import_bytes),
        max_upload_image_bytes: tighter(platform.max_upload_image_bytes, p.max_upload_image_bytes),
        max_archive_bytes: tighter(platform.max_archive_bytes, p.max_archive_bytes),
        max_vaults: tighter(platform.max_vaults, p.max_vaults),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_and_pro_match_design_table() {
        let free = AccountPlan::Free.limits();
        assert_eq!(free.max_bookmarks, Some(5_000));
        assert_eq!(free.max_stored_image_bytes, Some(1_073_741_824));
        assert_eq!(free.max_enrich_actions_per_day, Some(500));
        assert_eq!(free.max_archive_bytes, Some(524_288_000));
        assert_eq!(free.max_vaults, Some(1));
        assert!(free.max_import_bytes.is_none());
        assert!(free.max_upload_image_bytes.is_none());

        let pro = AccountPlan::Pro.limits();
        assert!(pro.max_bookmarks.is_none());
        assert_eq!(pro.max_stored_image_bytes, Some(10_737_418_240));
        assert_eq!(pro.max_enrich_actions_per_day, Some(5_000));
        assert_eq!(pro.max_archive_bytes, Some(26_843_545_600));
        assert!(pro.max_vaults.is_none());
    }

    #[test]
    fn tighter_picks_min_or_present() {
        assert_eq!(tighter(Some(10), Some(3)), Some(3));
        assert_eq!(tighter(Some(10), None), Some(10));
        assert_eq!(tighter(None, Some(7)), Some(7));
        assert_eq!(tighter(None, None), None);
    }

    #[test]
    fn merge_without_plan_is_platform() {
        let platform = ResolvedLimits {
            max_bookmarks: Some(100),
            max_vaults: None,
            ..ResolvedLimits::default()
        };
        assert_eq!(merge_limits(platform, None).max_bookmarks, Some(100));
        assert!(merge_limits(platform, None).max_vaults.is_none());
    }

    #[test]
    fn merge_free_with_tighter_platform() {
        let platform = ResolvedLimits {
            max_bookmarks: Some(100),
            max_archive_bytes: Some(1_000_000_000),
            ..ResolvedLimits::default()
        };
        let m = merge_limits(platform, Some(AccountPlan::Free));
        assert_eq!(m.max_bookmarks, Some(100));
        assert_eq!(m.max_archive_bytes, Some(524_288_000));
        assert_eq!(m.max_vaults, Some(1));
    }

    #[test]
    fn merge_pro_fills_unlimited_platform_meters() {
        let platform = ResolvedLimits::default();
        let m = merge_limits(platform, Some(AccountPlan::Pro));
        assert!(m.max_bookmarks.is_none());
        assert_eq!(m.max_stored_image_bytes, Some(10_737_418_240));
        assert!(m.max_vaults.is_none());
    }
}
