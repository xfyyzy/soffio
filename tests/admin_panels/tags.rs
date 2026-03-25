use super::*;

#[test]
fn snapshot_admin_tags_panel() {
    let content = AdminTagListView {
        heading: "Tags".into(),
        tags: vec![AdminTagRowView {
            id: "111".into(),
            name: "rust".into(),
            slug: "rust".into(),
            description: Some("Systems".into()),
            usage_count: 10,
            pinned: true,
            display_time: Some("2025-10-24T12:00:00Z".into()),
            display_time_kind: AdminPostTimeKind::Updated,
            public_href: "http://localhost:3000/tags/rust".into(),
            edit_href: "/tags/111/edit".into(),
            pin_action: "/tags/111/pin".into(),
            unpin_action: "/tags/111/unpin".into(),
            delete_action: "/tags/111/delete".into(),
        }],
        filter_search: Some("rust".into()),
        filter_month: Some("2025-10".into()),
        filter_tag: None,
        filter_query: "search=rust&month=2025-10".into(),
        tag_options: Vec::new(),
        filters: vec![
            AdminPageStatusFilterView {
                status_key: None,
                label: "All".into(),
                count: 5,
                is_active: false,
            },
            AdminPageStatusFilterView {
                status_key: Some("pinned".into()),
                label: "Pinned".into(),
                count: 2,
                is_active: true,
            },
            AdminPageStatusFilterView {
                status_key: Some("unpinned".into()),
                label: "Unpinned".into(),
                count: 3,
                is_active: false,
            },
        ],
        month_options: vec![AdminPostMonthOption {
            key: "2025-10".into(),
            label: "October 2025".into(),
            count: 1,
        }],
        next_cursor: Some("next".into()),
        cursor_param: Some("cursor".into()),
        trail: Some("~".into()),
        previous_page_state: Some(AdminPostPaginationState {
            cursor: Some("prev".into()),
            trail: Some("".into()),
        }),
        next_page_state: Some(AdminPostPaginationState {
            cursor: Some("next".into()),
            trail: Some("~.cursor".into()),
        }),
        active_status_key: Some("pinned".into()),
        panel_action: "/tags/panel".into(),
        new_tag_href: "/tags/new".into(),
        time_column_label: "Updated/Created".into(),
        month_filter_enabled: true,
        tag_filter_enabled: false,
        tag_filter_label: "Tag".into(),
        tag_filter_all_label: "All tags".into(),
        tag_filter_field: "tag".into(),
        custom_hidden_fields: Vec::new(),
    };

    let template = AdminTagsPanelTemplate { content };
    let rendered = template.render().unwrap();
    assert_admin_snapshot!("admin_tags_panel", rendered);
}
