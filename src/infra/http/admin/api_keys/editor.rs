use crate::{domain::api_keys::ApiScope, presentation::admin::views as admin_views};

pub fn build_new_key_view(issued_token: Option<String>) -> admin_views::AdminApiKeyNewView {
    admin_views::AdminApiKeyNewView {
        heading: "Create API key".to_string(),
        form_action: "/api-keys/new".to_string(),
        name: None,
        description: None,
        expires_in_options: expires_in_options(None),
        scope_picker: build_scope_picker(&[]),
        new_token: issued_token,
    }
}

pub fn scope_options() -> Vec<admin_views::AdminApiScopeOption> {
    ApiScope::all()
        .iter()
        .map(|scope| admin_views::AdminApiScopeOption {
            value: scope.as_str().to_string(),
            label: scope.display_name().to_string(),
        })
        .collect()
}

pub fn expires_in_options(selected: Option<&str>) -> Vec<admin_views::AdminApiKeyExpiresInOption> {
    vec![
        ("", "Never expires"),
        ("30d", "30 days"),
        ("90d", "90 days"),
        ("180d", "180 days"),
        ("1y", "1 year"),
    ]
    .into_iter()
    .map(|(value, label)| admin_views::AdminApiKeyExpiresInOption {
        value: value.to_string(),
        label: label.to_string(),
        selected: selected == Some(value) || (selected.is_none() && value.is_empty()),
    })
    .collect()
}

pub fn build_scope_picker(selected_scopes: &[String]) -> admin_views::AdminApiKeyScopePickerView {
    let all_scopes = scope_options();
    let selected: Vec<admin_views::AdminApiScopeOption> = all_scopes
        .iter()
        .filter(|s| selected_scopes.contains(&s.value))
        .cloned()
        .collect();
    let available: Vec<admin_views::AdminApiScopeOption> = all_scopes
        .iter()
        .filter(|s| !selected_scopes.contains(&s.value))
        .cloned()
        .collect();
    admin_views::AdminApiKeyScopePickerView {
        toggle_action: "/api-keys/new/scopes/toggle".to_string(),
        selected,
        available,
        selected_values: selected_scopes.to_vec(),
    }
}

pub fn parse_expires_in(value: Option<&str>) -> Option<time::Duration> {
    use time::Duration;
    match value {
        None | Some("") => None, // Never expires
        Some("30d") => Some(Duration::days(30)),
        Some("90d") => Some(Duration::days(90)),
        Some("180d") => Some(Duration::days(180)),
        Some("1y") => Some(Duration::days(365)),
        _ => None, // Unknown value, treat as never expires
    }
}

pub fn parse_scope_state(state: &Option<String>) -> Vec<String> {
    state
        .as_deref()
        .map(|raw| {
            raw.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}
