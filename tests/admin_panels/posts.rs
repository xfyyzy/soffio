use super::*;

#[test]
fn snapshot_admin_posts_panel() {
    let content = AdminPostListView {
        heading: "Posts".into(),
        filters: vec![AdminPostStatusFilterView {
            status_key: None,
            label: "All".into(),
            count: 2,
            is_active: true,
        }],
        posts: vec![AdminPostRowView {
            id: "123".into(),
            title: "Hello World".into(),
            status_key: "draft".into(),
            status_label: "Draft".into(),
            display_time: Some("2025-10-24T12:00:00Z".into()),
            display_time_kind: AdminPostTimeKind::Updated,
            actions: vec![
                AdminPostRowActionView {
                    value: "pin",
                    label: "Pin",
                    is_danger: false,
                },
                AdminPostRowActionView {
                    value: "publish",
                    label: "Publish",
                    is_danger: false,
                },
                AdminPostRowActionView {
                    value: "archive",
                    label: "Archive",
                    is_danger: false,
                },
            ],
            preview_href: "http://localhost:3000/posts/_preview/123".into(),
            edit_href: "/posts/hello-world/edit".into(),
            is_pinned: false,
            snapshots_href: Some("/posts/123/snapshots".into()),
        }],
        tag_options: vec![AdminPostTagOption {
            slug: "rust".into(),
            name: "Rust".into(),
            count: 3,
        }],
        month_options: vec![AdminPostMonthOption {
            key: "2025-10".into(),
            label: "October 2025".into(),
            count: 1,
        }],
        filter_search: Some("hello".into()),
        filter_tag: Some("rust".into()),
        filter_month: Some("2025-10".into()),
        filter_query: "search=hello&tag=rust&month=2025-10".into(),
        next_cursor: Some("next".into()),
        cursor_param: Some("current".into()),
        trail: Some("~".into()),
        previous_page_state: Some(AdminPostPaginationState {
            cursor: None,
            trail: None,
        }),
        next_page_state: Some(AdminPostPaginationState {
            cursor: Some("next".into()),
            trail: Some("~.current".into()),
        }),
        time_column_label: "Published/Updated".into(),
        new_post_href: "/posts/new".into(),
        public_site_url: "http://localhost:3000/".into(),
        active_status_key: Some("draft".into()),
        panel_action: "/posts/panel".into(),
        tag_filter_label: "Tag".into(),
        tag_filter_all_label: "All tags".into(),
        tag_filter_field: "tag".into(),
        tag_filter_enabled: true,
        month_filter_enabled: true,
        row_action_prefix: "/posts".into(),
        custom_hidden_fields: Vec::new(),
    };

    let template = AdminPostsPanelTemplate { content };
    let rendered = template.render().unwrap();
    assert_admin_snapshot!("admin_posts_panel", rendered);
}
