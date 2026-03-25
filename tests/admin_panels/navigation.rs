use super::*;

#[test]
fn snapshot_admin_navigation_panel() {
    let content = AdminNavigationListView {
        heading: "Navigation".into(),
        filters: vec![
            AdminNavigationStatusFilterView {
                status_key: None,
                label: "All".into(),
                count: 2,
                is_active: true,
            },
            AdminNavigationStatusFilterView {
                status_key: Some("visible".into()),
                label: "Visible".into(),
                count: 1,
                is_active: false,
            },
            AdminNavigationStatusFilterView {
                status_key: Some("hidden".into()),
                label: "Hidden".into(),
                count: 1,
                is_active: false,
            },
        ],
        items: vec![AdminNavigationRowView {
            id: "nav1".into(),
            label: "About".into(),
            preview_href: "https://example.com/about".into(),
            destination_type_label: "Internal".into(),
            destination_type_status: "internal".into(),
            destination_display: "/about".into(),
            sort_order: 1,
            visible: true,
            toggle_action: "/navigation/nav1/visibility".into(),
            toggle_label: "Hide",
            edit_href: "/navigation/nav1/edit".into(),
            delete_action: "/navigation/nav1/delete".into(),
        }],
        filter_search: Some("about".into()),
        filter_tag: None,
        filter_month: None,
        filter_query: "search=about".into(),
        next_cursor: Some("next".into()),
        cursor_param: Some("cursor_token".into()),
        trail: Some("trail_token".into()),
        previous_page_state: Some(AdminPostPaginationState {
            cursor: Some("prev".into()),
            trail: Some("prev_trail".into()),
        }),
        next_page_state: None,
        tag_options: Vec::new(),
        month_options: Vec::new(),
        tag_filter_enabled: false,
        month_filter_enabled: false,
        panel_action: "/navigation/panel".into(),
        active_status_key: Some("visible".into()),
        new_navigation_href: "/navigation/new".into(),
        tag_filter_label: "Tag".into(),
        tag_filter_all_label: "All tags".into(),
        tag_filter_field: "tag".into(),
        custom_hidden_fields: Vec::new(),
    };

    let template = AdminNavigationPanelTemplate { content };
    let rendered = template.render().unwrap();
    assert_admin_snapshot!("admin_navigation_panel", rendered);
}
