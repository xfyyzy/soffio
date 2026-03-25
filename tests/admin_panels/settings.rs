use super::*;

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
