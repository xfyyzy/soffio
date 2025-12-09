//! Status helpers for audit admin.

use crate::application::repos::{AuditActionCount, AuditActorCount, AuditEntityTypeCount};
use crate::presentation::admin::views::{
    AdminAuditActionOption, AdminAuditActorOption, AdminAuditStatusFilterView,
};

/// Build entity type status filters (tabs).
pub(super) fn entity_type_filters(
    entity_type_counts: &[AuditEntityTypeCount],
    total_count: u64,
    active_entity_type: Option<&str>,
) -> Vec<AdminAuditStatusFilterView> {
    let mut filters = Vec::with_capacity(entity_type_counts.len() + 1);

    // "All" tab
    filters.push(AdminAuditStatusFilterView {
        status_key: None,
        label: "All".to_string(),
        count: total_count as usize,
        is_active: active_entity_type.is_none(),
    });

    // Entity type tabs
    for et in entity_type_counts {
        filters.push(AdminAuditStatusFilterView {
            status_key: Some(et.entity_type.clone()),
            label: capitalize_label(&et.entity_type),
            count: et.count as usize,
            is_active: active_entity_type == Some(&et.entity_type),
        });
    }

    filters
}

/// Build actor dropdown options.
pub(super) fn actor_options(
    actor_counts: &[AuditActorCount],
    _selected_actor: Option<&str>,
) -> Vec<AdminAuditActorOption> {
    actor_counts
        .iter()
        .map(|a| AdminAuditActorOption {
            value: a.actor.clone(),
            label: format!("{} ({})", &a.actor, a.count),
            count: a.count as usize,
        })
        .collect()
}

/// Build action dropdown options.
pub(super) fn action_options(
    action_counts: &[AuditActionCount],
    _selected_action: Option<&str>,
) -> Vec<AdminAuditActionOption> {
    action_counts
        .iter()
        .map(|a| AdminAuditActionOption {
            value: a.action.clone(),
            label: format!("{} ({})", &a.action, a.count),
            count: a.count as usize,
        })
        .collect()
}

fn capitalize_label(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
