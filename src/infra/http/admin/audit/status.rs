//! Status helpers for audit admin.

use crate::presentation::admin::views::AdminAuditStatusFilterView;

/// Build status filters for audit list (only "All" since audit has no status).
pub(super) fn status_filters(total_count: u64) -> Vec<AdminAuditStatusFilterView> {
    vec![AdminAuditStatusFilterView {
        status_key: None,
        label: "All".to_string(),
        count: total_count as usize,
        is_active: true,
    }]
}
