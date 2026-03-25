use super::*;

#[test]
fn render_admin_uploads_template() {
    let chrome = AdminChrome {
        brand: AdminBrandView {
            title: "Soffio Admin".into(),
        },
        navigation: AdminNavigationView { items: Vec::new() },
        meta: AdminMetaView {
            title: "Uploads".into(),
            description: "Manage uploads".into(),
        },
    };

    let rows = vec![AdminUploadRowView {
        id: Uuid::nil().to_string(),
        filename: "diagram.png".into(),
        content_type: "image/png".into(),
        size_bytes: 1_024,
        size_label: "1 KiB".into(),
        created_at: "2025-10-25T00:00:00Z".into(),
        download_href: "/uploads/00000000-0000-0000-0000-000000000000".into(),
        delete_action: "/uploads/00000000-0000-0000-0000-000000000000/delete".into(),
        preview_href: Some("https://example.com/uploads/diagram.png?height=480&width=640".into()),
        public_href: "https://example.com/uploads/diagram.png?height=480&width=640".into(),
    }];

    let content = AdminUploadListView {
        heading: "Uploads".into(),
        uploads: rows,
        filter_search: Some("diagram".into()),
        filter_tag: Some("image/png".into()),
        filter_month: Some("2025-10".into()),
        filter_query: "search=diagram&content_type=image/png&month=2025-10".into(),
        active_status_key: None,
        tag_options: vec![AdminPostTagOption {
            slug: "image/png".into(),
            name: "image/png".into(),
            count: 1,
        }],
        month_options: vec![AdminPostMonthOption {
            key: "2025-10".into(),
            label: "October 2025".into(),
            count: 1,
        }],
        next_cursor: None,
        cursor_param: None,
        trail: None,
        previous_page_state: None,
        next_page_state: None,
        panel_action: "/uploads/panel".into(),
        new_upload_href: "/uploads/new".into(),
        tag_filter_label: "Content Type".into(),
        tag_filter_all_label: "All types".into(),
        tag_filter_field: "content_type".into(),
        tag_filter_enabled: true,
        month_filter_enabled: true,
        copy_toast_action: "/toasts".into(),
        custom_hidden_fields: Vec::new(),
    };

    let template = AdminUploadsTemplate {
        view: AdminLayout::new(chrome, content),
    };

    let rendered = template.render().unwrap();
    assert!(rendered.contains("Upload File"));
    assert!(rendered.contains("diagram.png"));
}
