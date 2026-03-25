use super::*;

#[test]
fn snapshot_admin_pages_panel() {
    let content = AdminPageListView {
        heading: "Pages".into(),
        filters: vec![AdminPageStatusFilterView {
            status_key: None,
            label: "All".into(),
            count: 1,
            is_active: true,
        }],
        pages: vec![AdminPageRowView {
            id: "321".into(),
            title: "About".into(),
            slug: "about".into(),
            status_key: "published".into(),
            status_label: "Published".into(),
            display_time: Some("2025-10-24T12:00:00Z".into()),
            display_time_kind: AdminPostTimeKind::Updated,
            actions: vec![AdminPostRowActionView {
                value: "draft",
                label: "Move to Draft",
                is_danger: false,
            }],
            preview_href: "http://localhost:3000/pages/_preview/321".into(),
            edit_href: "/pages/about/edit".into(),
            snapshots_href: Some("/pages/321/snapshots".into()),
        }],
        filter_search: Some("about".into()),
        filter_tag: None,
        filter_month: Some("2025-10".into()),
        filter_query: "search=about&month=2025-10".into(),
        next_cursor: None,
        cursor_param: None,
        trail: None,
        previous_page_state: None,
        next_page_state: None,
        tag_options: Vec::new(),
        month_options: vec![AdminPostMonthOption {
            key: "2025-10".into(),
            label: "October 2025".into(),
            count: 1,
        }],
        time_column_label: "Published/Updated".into(),
        new_page_href: "/pages/new".into(),
        public_site_url: "http://localhost:3000/".into(),
        active_status_key: None,
        panel_action: "/pages/panel".into(),
        tag_filter_label: "Tag".into(),
        tag_filter_all_label: "All tags".into(),
        tag_filter_field: "tag".into(),
        tag_filter_enabled: false,
        month_filter_enabled: true,
        row_action_prefix: "/pages".into(),
        custom_hidden_fields: Vec::new(),
    };

    let template = AdminPagesPanelTemplate { content };
    let rendered = template.render().unwrap();
    assert_admin_snapshot!("admin_pages_panel", rendered);
}
