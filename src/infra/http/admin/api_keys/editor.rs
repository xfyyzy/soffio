use crate::{domain::api_keys::ApiScope, presentation::admin::views as admin_views};

pub fn build_new_key_view() -> admin_views::AdminApiKeyEditorView {
    admin_views::AdminApiKeyEditorView {
        heading: "Create API key".to_string(),
        form_action: "/api-keys/create".to_string(),
        name: String::new(),
        description: None,
        scope_picker: build_scope_picker(&[]),
        expires_in_options: Some(expires_in_options(None)),
        submit_label: "Create key".to_string(),
        show_back_link: false,
    }
}

pub fn scope_options() -> Vec<admin_views::AdminApiScopeOption> {
    ApiScope::all()
        .iter()
        .map(|scope| admin_views::AdminApiScopeOption {
            value: scope.as_str().to_string(),
            label: scope.display_name().to_string(),
            is_selected: false,
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
    let selected_set: std::collections::HashSet<&str> =
        selected_scopes.iter().map(String::as_str).collect();

    let mut selected = Vec::new();
    let mut available = Vec::new();
    let mut selected_values = Vec::new();

    for scope in all_scopes {
        let is_selected = selected_set.contains(scope.value.as_str());
        let option = admin_views::AdminApiScopeOption {
            is_selected,
            ..scope
        };

        if is_selected {
            selected_values.push(option.value.clone());
            selected.push(option.clone());
        }

        available.push(option);
    }

    admin_views::AdminApiKeyScopePickerView {
        toggle_action: "/api-keys/new/scopes/toggle".to_string(),
        selected,
        available,
        selected_values,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::api_keys::ApiScope;

    #[test]
    fn build_scope_picker_keeps_selected_available() {
        let selected_scopes = vec![ApiScope::PostRead.as_str().to_string()];

        let picker = build_scope_picker(&selected_scopes);

        // Available list remains stable and marks selected items.
        assert_eq!(picker.available.len(), scope_options().len());
        assert!(
            picker
                .available
                .iter()
                .any(|option| option.value == ApiScope::PostRead.as_str() && option.is_selected)
        );
        assert_eq!(picker.selected_values, selected_scopes);
    }
}
