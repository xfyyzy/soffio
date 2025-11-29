use askama::Template;
use insta::assert_snapshot;
use soffio::presentation::admin::views::*;
use uuid::Uuid;

fn sample_flash(kind: &'static str, text: &str) -> Option<AdminFlashMessage> {
    Some(AdminFlashMessage {
        kind,
        text: text.to_string(),
    })
}

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
            slug: "hello-world".into(),
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
    };

    let template = AdminPostsPanelTemplate { content };
    let rendered = template.render().unwrap();
    assert_snapshot!("admin_posts_panel", rendered);
}

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
    };

    let template = AdminPagesPanelTemplate { content };
    let rendered = template.render().unwrap();
    assert_snapshot!("admin_pages_panel", rendered);
}

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
    };

    let template = AdminTagsPanelTemplate { content };
    let rendered = template.render().unwrap();
    assert_snapshot!("admin_tags_panel", rendered);
}

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
    };

    let template = AdminNavigationPanelTemplate { content };
    let rendered = template.render().unwrap();
    assert_snapshot!("admin_navigation_panel", rendered);
}

#[test]
fn render_admin_settings_template() {
    let chrome = AdminChrome {
        brand: AdminBrandView {
            title: "Soffio Admin".into(),
        },
        navigation: AdminNavigationView { items: Vec::new() },
        meta: AdminMetaView {
            title: "Settings".into(),
            description: "Site configuration".into(),
        },
    };

    let content = AdminSettingsSummaryView {
        heading: "Site Settings".into(),
        simple_fields: vec![
            AdminSettingsSummaryField {
                label: "Homepage Size".into(),
                value: "10".into(),
                value_kind: AdminSettingsSummaryValueKind::Text,
            },
            AdminSettingsSummaryField {
                label: "Show Tag Aggregations".into(),
                value: "Enabled".into(),
                value_kind: AdminSettingsSummaryValueKind::Badge {
                    status: "enabled",
                    label: "Enabled",
                },
            },
            AdminSettingsSummaryField {
                label: "Brand Title".into(),
                value: "Soffio".into(),
                value_kind: AdminSettingsSummaryValueKind::Text,
            },
        ],
        multiline_fields: vec![AdminSettingsSummaryField {
            label: "Footer Copy".into(),
            value: "© Soffio".into(),
            value_kind: AdminSettingsSummaryValueKind::Multiline,
        }],
        updated_at: "2025-10-25T00:00:00Z".into(),
        edit_href: "/settings/edit".into(),
    };

    let template = AdminSettingsTemplate {
        view: AdminLayout::new(chrome, content),
    };

    let rendered = template.render().unwrap();
    assert!(rendered.contains("Edit Settings"));
}

#[test]
fn render_admin_settings_edit_template() {
    let chrome = AdminChrome {
        brand: AdminBrandView {
            title: "Soffio Admin".into(),
        },
        navigation: AdminNavigationView { items: Vec::new() },
        meta: AdminMetaView {
            title: "Edit Settings".into(),
            description: "Update configuration".into(),
        },
    };

    let content = AdminSettingsEditView {
        heading: "Edit Site Settings".into(),
        simple_fields: vec![
            AdminSettingsEditSimpleField {
                label: "Homepage Size".into(),
                input: AdminSettingsEditInputKind::Number {
                    name: "homepage_size".into(),
                    value: "10".into(),
                    min: Some("1".into()),
                },
            },
            AdminSettingsEditSimpleField {
                label: "Show Tag Aggregations".into(),
                input: AdminSettingsEditInputKind::Checkbox {
                    name: "show_tag_aggregations".into(),
                    checked: true,
                    toggle_id: "settings-toggle-show-tag".into(),
                },
            },
            AdminSettingsEditSimpleField {
                label: "Timezone".into(),
                input: AdminSettingsEditInputKind::Text {
                    name: "timezone".into(),
                    value: "Asia/Shanghai".into(),
                    placeholder: Some("Asia/Shanghai".into()),
                },
            },
        ],
        multiline_fields: vec![AdminSettingsEditMultilineField {
            label: "Footer Copy".into(),
            name: "footer_copy".into(),
            value: "© Soffio".into(),
            rows: 3,
        }],
        updated_at: "2025-10-25T00:00:00Z".into(),
        form_action: "/settings/edit".into(),
        submit_label: "Save Changes".into(),
        enable_live_submit: true,
    };

    let template = AdminSettingsEditTemplate {
        view: AdminLayout::new(chrome, content),
    };

    let rendered = template.render().unwrap();
    assert!(rendered.contains("Last updated"));
}

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
    };

    let template = AdminUploadsTemplate {
        view: AdminLayout::new(chrome, content),
    };

    let rendered = template.render().unwrap();
    assert!(rendered.contains("Upload File"));
    assert!(rendered.contains("diagram.png"));
}

#[test]
fn render_admin_job_detail_template() {
    let chrome = AdminChrome {
        brand: AdminBrandView {
            title: "Soffio Admin".into(),
        },
        navigation: AdminNavigationView { items: Vec::new() },
        meta: AdminMetaView {
            title: "Job detail".into(),
            description: "Inspect job".into(),
        },
    };

    let detail = AdminJobDetailView {
        id: "job-42".into(),
        job_type: "render_post".into(),
        status: "Failed".into(),
        attempts: 3,
        max_attempts: 5,
        run_at: "2025-10-25T08:00:00Z".into(),
        lock_at: Some("2025-10-25T08:01:00Z".into()),
        lock_by: Some("worker-1".into()),
        done_at: None,
        last_error: Some("panic: oops".into()),
        priority: 10,
        payload_pretty: "{\n  \"post_id\": \"123\"\n}".into(),
        retry_action: "/jobs/job-42/retry".into(),
        cancel_action: "/jobs/job-42/cancel".into(),
        back_href: "/jobs?state=failed&job_type=render_post&search=job-42&cursor=cursor-token"
            .into(),
        filter_state: Some("failed".into()),
        filter_job_type: Some("render_post".into()),
        filter_search: Some("job-42".into()),
        filter_cursor: Some("cursor-token".into()),
        can_retry: true,
        can_cancel: true,
        flash: sample_flash("info", "Updated"),
        filter_query: "state=failed&job_type=render_post&search=job-42".into(),
    };

    let template = AdminJobDetailTemplate {
        view: AdminLayout::new(chrome, detail),
    };

    let rendered = template.render().unwrap();
    assert!(rendered.contains("Job job-42"));
    assert!(rendered.contains("Retry"));
}

#[test]
fn snapshot_admin_jobs_template() {
    let chrome = AdminChrome {
        brand: AdminBrandView {
            title: "Soffio Admin".into(),
        },
        navigation: AdminNavigationView { items: Vec::new() },
        meta: AdminMetaView {
            title: "Jobs".into(),
            description: "Job queue".into(),
        },
    };

    let content = AdminJobListView {
        heading: "Jobs".into(),
        jobs: vec![AdminJobRowView {
            id: "job-1".into(),
            job_type: "render_post".into(),
            status: "Failed".into(),
            scheduled_at: Some("2025-10-25T09:00:00Z".into()),
            created_at: "2025-10-25T09:05:00Z".into(),
            error_text: Some("panic: boom".into()),
            detail_href:
                "/jobs/job-1?state=failed&job_type=render_post&search=job-1&cursor=prev-cursor"
                    .into(),
            retry_action: "/jobs/job-1/retry".into(),
            cancel_action: "/jobs/job-1/cancel".into(),
            can_retry: true,
            can_cancel: false,
        }],
        state_options: vec![
            AdminJobFilterOption {
                value: "".into(),
                label: "All states".into(),
                selected: false,
            },
            AdminJobFilterOption {
                value: "failed".into(),
                label: "Failed".into(),
                selected: true,
            },
        ],
        type_options: vec![
            AdminJobFilterOption {
                value: "".into(),
                label: "All types".into(),
                selected: false,
            },
            AdminJobFilterOption {
                value: "render_post".into(),
                label: "Render Post".into(),
                selected: true,
            },
        ],
        filter_search: Some("job-1".into()),
        filter_state: Some("failed".into()),
        filter_job_type: Some("render_post".into()),
        filter_query: "state=failed&job_type=render_post&search=job-1".into(),
        current_cursor: Some("prev-cursor".into()),
        next_cursor: Some("next-cursor".into()),
        flash: sample_flash("info", "Filtered"),
    };

    let template = AdminJobsTemplate {
        view: AdminLayout::new(chrome, content),
    };

    let rendered = template.render().unwrap();
    assert_snapshot!("admin_jobs", rendered);
}

#[test]
fn snapshot_admin_audit_template() {
    let chrome = AdminChrome {
        brand: AdminBrandView {
            title: "Soffio Admin".into(),
        },
        navigation: AdminNavigationView { items: Vec::new() },
        meta: AdminMetaView {
            title: "Audit Log".into(),
            description: "Recent changes".into(),
        },
    };

    let content = AdminAuditListView {
        heading: "Audit Log".into(),
        entries: vec![AdminAuditRowView {
            id: Uuid::nil().to_string(),
            actor: "admin".into(),
            action: "post.update".into(),
            entity: "post:123".into(),
            created_at: "2025-10-24T12:00:00Z".into(),
        }],
        filter_actor: Some("admin".into()),
        filter_action: Some("post".into()),
        filter_entity_type: Some("post".into()),
        filter_search: Some("123".into()),
        filter_query: "actor=admin&action=post&entity_type=post&search=123".into(),
        next_cursor: Some("audit-next".into()),
    };

    let template = AdminAuditTemplate {
        view: AdminLayout::new(chrome, content),
    };

    let rendered = template.render().unwrap();
    assert_snapshot!("admin_audit", rendered);
}

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
            },
            AdminApiScopeOption {
                value: "content_write".into(),
                label: "Content write".into(),
            },
        ],
        new_token: None,
    };

    let template = AdminApiKeysPanelTemplate { content };
    let rendered = template.render().unwrap();
    assert_snapshot!("admin_api_keys_panel", rendered);
}

#[test]
fn snapshot_admin_api_key_new_panel() {
    let content = AdminApiKeyNewView {
        heading: "Create API key".into(),
        form_action: "/api-keys/new".into(),
        name: None,
        description: None,
        expires_in_options: vec![
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
        ],
        scope_picker: AdminApiKeyScopePickerView {
            toggle_action: "/api-keys/new/scopes/toggle".into(),
            selected: Vec::new(),
            available: vec![
                AdminApiScopeOption {
                    value: "content_read".into(),
                    label: "Content read".into(),
                },
                AdminApiScopeOption {
                    value: "content_write".into(),
                    label: "Content write".into(),
                },
                AdminApiScopeOption {
                    value: "tag_write".into(),
                    label: "Tag write".into(),
                },
            ],
            selected_values: Vec::new(),
        },
        new_token: None,
    };

    let template = AdminApiKeyNewPanelTemplate { content };
    let rendered = template.render().unwrap();
    assert_snapshot!("admin_api_key_new_panel", rendered);
}
