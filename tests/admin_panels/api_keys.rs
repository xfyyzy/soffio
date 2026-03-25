use super::*;

#[test]
fn snapshot_admin_api_keys_panel() {
    let content = AdminApiKeyListView {
        heading: "API keys".into(),
        keys: vec![AdminApiKeyRowView {
            id: Uuid::nil().to_string(),
            name: "test-key".into(),
            prefix: "sk_abc123".into(),
            scopes: vec![
                AdminApiScopeDisplay {
                    slug: "post_read".into(),
                    label: "Post read".into(),
                },
                AdminApiScopeDisplay {
                    slug: "post_write".into(),
                    label: "Post write".into(),
                },
            ],
            created_at: "2025-10-24T12:00:00Z".into(),
            last_used_at: Some("2025-10-25T09:00:00Z".into()),
            expires_at: Some("2026-10-24T12:00:00Z".into()),
            status: "active".into(),
            status_label: "Active".into(),
            description: Some("Test API key".into()),
            edit_href: "/api-keys/00000000-0000-0000-0000-000000000000/edit".into(),
            revoke_action: "/api-keys/00000000-0000-0000-0000-000000000000/revoke".into(),
            rotate_action: "/api-keys/00000000-0000-0000-0000-000000000000/rotate".into(),
            delete_action: "/api-keys/00000000-0000-0000-0000-000000000000/delete".into(),
        }],
        create_action: "/api-keys/create".into(),
        new_key_href: "/api-keys/new".into(),
        panel_action: "/api-keys/panel".into(),
        filters: vec![
            AdminApiKeyStatusFilterView {
                status_key: None,
                label: "All".into(),
                count: 5,
                is_active: true,
            },
            AdminApiKeyStatusFilterView {
                status_key: Some("active".into()),
                label: "Active".into(),
                count: 4,
                is_active: false,
            },
            AdminApiKeyStatusFilterView {
                status_key: Some("revoked".into()),
                label: "Revoked".into(),
                count: 1,
                is_active: false,
            },
        ],
        active_status_key: None,
        filter_search: Some("test".into()),
        filter_scope: Some("content_read".into()),
        filter_tag: Some("content_read".into()),
        filter_month: None,
        tag_filter_enabled: true,
        month_filter_enabled: false,
        tag_filter_label: "Scope".into(),
        tag_filter_all_label: "All scopes".into(),
        tag_filter_field: "scope".into(),
        tag_options: vec![
            AdminPostTagOption {
                slug: "content_read".into(),
                name: "content_read".into(),
                count: 3,
            },
            AdminPostTagOption {
                slug: "content_write".into(),
                name: "content_write".into(),
                count: 2,
            },
        ],
        month_options: Vec::new(),
        cursor_param: Some("cursor-token".into()),
        trail: Some("~".into()),
        previous_page_state: Some(AdminApiKeyPaginationState {
            cursor: None,
            trail: None,
        }),
        next_page_state: Some(AdminApiKeyPaginationState {
            cursor: Some("next-cursor".into()),
            trail: Some("~.cursor-token".into()),
        }),
        available_scopes: vec![
            AdminApiScopeOption {
                value: "content_read".into(),
                label: "Content read".into(),
                is_selected: false,
            },
            AdminApiScopeOption {
                value: "content_write".into(),
                label: "Content write".into(),
                is_selected: false,
            },
        ],
        custom_hidden_fields: Vec::new(),
    };

    let template = AdminApiKeysPanelTemplate { content };
    let rendered = template.render().unwrap();
    assert_admin_snapshot!("admin_api_keys_panel", rendered);
}

#[test]
fn snapshot_admin_api_key_new_panel() {
    let content = AdminApiKeyEditorView {
        heading: "Create API key".into(),
        form_action: "/api-keys/create".into(),
        name: String::new(),
        description: None,
        scope_picker: AdminApiKeyScopePickerView {
            toggle_action: "/api-keys/new/scopes/toggle".into(),
            selected: Vec::new(),
            available: vec![
                AdminApiScopeOption {
                    value: "content_read".into(),
                    label: "Content read".into(),
                    is_selected: false,
                },
                AdminApiScopeOption {
                    value: "content_write".into(),
                    label: "Content write".into(),
                    is_selected: false,
                },
                AdminApiScopeOption {
                    value: "tag_write".into(),
                    label: "Tag write".into(),
                    is_selected: false,
                },
            ],
            selected_values: Vec::new(),
        },
        expires_in_options: Some(vec![
            AdminApiKeyExpiresInOption {
                value: "".into(),
                label: "Never expires".into(),
                selected: true,
            },
            AdminApiKeyExpiresInOption {
                value: "30d".into(),
                label: "30 days".into(),
                selected: false,
            },
            AdminApiKeyExpiresInOption {
                value: "90d".into(),
                label: "90 days".into(),
                selected: false,
            },
        ]),
        submit_label: "Create key".into(),
        show_back_link: false,
    };

    let template = AdminApiKeyEditorPanelTemplate { content };
    let rendered = template.render().unwrap();
    assert_admin_snapshot!("admin_api_key_new_panel", rendered);
}
