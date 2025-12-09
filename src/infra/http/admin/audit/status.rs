//! Status helpers for audit admin.

use crate::application::repos::{AuditActionCount, AuditActorCount, AuditEntityTypeCount};
use crate::presentation::admin::views::{
    AdminAuditActionOption, AdminAuditActorOption, AdminAuditStatusFilterView,
};
use std::collections::HashMap;

/// All known entity types in the system.
const ALL_ENTITY_TYPES: &[&str] = &[
    "post",
    "page",
    "tag",
    "navigation",
    "upload",
    "api_key",
    "settings",
    "job",
];

/// Build entity type status filters (tabs).
/// Shows all entity types even if count is 0.
pub(super) fn entity_type_filters(
    entity_type_counts: &[AuditEntityTypeCount],
    total_count: u64,
    active_entity_type: Option<&str>,
) -> Vec<AdminAuditStatusFilterView> {
    // Build count map from database results
    let count_map: HashMap<&str, u64> = entity_type_counts
        .iter()
        .map(|et| (et.entity_type.as_str(), et.count))
        .collect();

    let mut filters = Vec::with_capacity(ALL_ENTITY_TYPES.len() + 1);

    // "All" tab
    filters.push(AdminAuditStatusFilterView {
        status_key: None,
        label: "All".to_string(),
        count: total_count as usize,
        is_active: active_entity_type.is_none(),
    });

    // Entity type tabs - show all types, use 0 if not in database
    for &entity_type in ALL_ENTITY_TYPES {
        let count = count_map.get(entity_type).copied().unwrap_or(0);
        filters.push(AdminAuditStatusFilterView {
            status_key: Some(entity_type.to_string()),
            label: capitalize_label(entity_type),
            count: count as usize,
            is_active: active_entity_type == Some(entity_type),
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
    // Handle snake_case like "api_key" -> "Api Key"
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
