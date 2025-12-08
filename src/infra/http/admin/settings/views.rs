//! View building functions for settings.

use crate::domain::entities::SiteSettingsRecord;
use crate::presentation::admin::views as admin_views;

pub(super) const SETTINGS_FORM_ACTION: &str = "/settings/edit";

pub(super) struct EditFieldValues {
    pub(super) homepage_size: String,
    pub(super) admin_page_size: String,
    pub(super) tag_filter_limit: String,
    pub(super) month_filter_limit: String,
    pub(super) timezone: String,
    pub(super) show_tag_aggregations: bool,
    pub(super) show_month_aggregations: bool,
    pub(super) global_toc_enabled: bool,
    pub(super) brand_title: String,
    pub(super) brand_href: String,
    pub(super) footer_copy: String,
    pub(super) public_site_url: String,
    pub(super) favicon_svg: String,
    pub(super) meta_title: String,
    pub(super) meta_description: String,
    pub(super) og_title: String,
    pub(super) og_description: String,
    pub(super) updated_at: String,
}

pub(super) fn summary_view_from_record(
    record: &SiteSettingsRecord,
) -> admin_views::AdminSettingsSummaryView {
    let (simple_fields, multiline_fields) = summary_fields(record);

    admin_views::AdminSettingsSummaryView {
        heading: "Site Settings".to_string(),
        simple_fields,
        multiline_fields,
        updated_at: admin_views::format_timestamp(record.updated_at, record.timezone),
        edit_href: "/settings/edit".to_string(),
    }
}

pub(super) fn edit_view_from_record(
    record: &SiteSettingsRecord,
) -> admin_views::AdminSettingsEditView {
    let timezone = record.timezone;
    build_edit_view(EditFieldValues {
        homepage_size: record.homepage_size.to_string(),
        admin_page_size: record.admin_page_size.to_string(),
        tag_filter_limit: record.tag_filter_limit.to_string(),
        month_filter_limit: record.month_filter_limit.to_string(),
        timezone: timezone.name().to_string(),
        show_tag_aggregations: record.show_tag_aggregations,
        show_month_aggregations: record.show_month_aggregations,
        global_toc_enabled: record.global_toc_enabled,
        brand_title: record.brand_title.clone(),
        brand_href: record.brand_href.clone(),
        footer_copy: record.footer_copy.clone(),
        public_site_url: record.public_site_url.clone(),
        favicon_svg: record.favicon_svg.clone(),
        meta_title: record.meta_title.clone(),
        meta_description: record.meta_description.clone(),
        og_title: record.og_title.clone(),
        og_description: record.og_description.clone(),
        updated_at: admin_views::format_timestamp(record.updated_at, timezone),
    })
}

fn summary_fields(
    record: &SiteSettingsRecord,
) -> (
    Vec<admin_views::AdminSettingsSummaryField>,
    Vec<admin_views::AdminSettingsSummaryField>,
) {
    let mut simple = Vec::new();
    let mut multiline = Vec::new();

    simple.push(summary_text_field(
        "Homepage Size",
        record.homepage_size.to_string(),
    ));
    simple.push(summary_text_field(
        "Admin Page Size",
        record.admin_page_size.to_string(),
    ));
    simple.push(summary_text_field(
        "Tag Filter Limit",
        record.tag_filter_limit.to_string(),
    ));
    simple.push(summary_text_field(
        "Month Filter Limit",
        record.month_filter_limit.to_string(),
    ));
    simple.push(summary_text_field(
        "Timezone",
        record.timezone.name().to_string(),
    ));
    simple.push(summary_badge_field(
        "Show Tag Aggregations",
        record.show_tag_aggregations,
    ));
    simple.push(summary_badge_field(
        "Show Month Aggregations",
        record.show_month_aggregations,
    ));
    simple.push(summary_badge_field(
        "Global Table of Contents",
        record.global_toc_enabled,
    ));
    simple.push(summary_text_field(
        "Brand Title",
        record.brand_title.clone(),
    ));
    simple.push(summary_text_field("Brand Link", record.brand_href.clone()));
    simple.push(summary_text_field(
        "Public Site URL",
        record.public_site_url.clone(),
    ));
    simple.push(summary_text_field("Meta Title", record.meta_title.clone()));
    simple.push(summary_text_field("OG Title", record.og_title.clone()));

    multiline.push(summary_multiline_field(
        "Footer Copy",
        record.footer_copy.clone(),
    ));
    multiline.push(summary_multiline_field(
        "Favicon SVG",
        record.favicon_svg.clone(),
    ));
    multiline.push(summary_multiline_field(
        "Meta Description",
        record.meta_description.clone(),
    ));
    multiline.push(summary_multiline_field(
        "OG Description",
        record.og_description.clone(),
    ));

    (simple, multiline)
}

pub(super) fn build_edit_view(values: EditFieldValues) -> admin_views::AdminSettingsEditView {
    let EditFieldValues {
        homepage_size,
        admin_page_size,
        tag_filter_limit,
        month_filter_limit,
        timezone,
        show_tag_aggregations,
        show_month_aggregations,
        global_toc_enabled,
        brand_title,
        brand_href,
        footer_copy,
        public_site_url,
        favicon_svg,
        meta_title,
        meta_description,
        og_title,
        og_description,
        updated_at,
    } = values;

    let simple_fields = vec![
        admin_views::AdminSettingsEditSimpleField {
            label: "Homepage Size".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Number {
                name: "homepage_size".to_string(),
                value: homepage_size,
                min: Some("1".to_string()),
            },
        },
        admin_views::AdminSettingsEditSimpleField {
            label: "Admin Page Size".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Number {
                name: "admin_page_size".to_string(),
                value: admin_page_size,
                min: Some("1".to_string()),
            },
        },
        admin_views::AdminSettingsEditSimpleField {
            label: "Tag Filter Limit".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Number {
                name: "tag_filter_limit".to_string(),
                value: tag_filter_limit,
                min: Some("1".to_string()),
            },
        },
        admin_views::AdminSettingsEditSimpleField {
            label: "Month Filter Limit".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Number {
                name: "month_filter_limit".to_string(),
                value: month_filter_limit,
                min: Some("1".to_string()),
            },
        },
        admin_views::AdminSettingsEditSimpleField {
            label: "Timezone".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Text {
                name: "timezone".to_string(),
                value: timezone,
                placeholder: Some("Asia/Shanghai".to_string()),
            },
        },
        admin_views::AdminSettingsEditSimpleField {
            label: "Show Tag Aggregations".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Checkbox {
                name: "show_tag_aggregations".to_string(),
                checked: show_tag_aggregations,
                toggle_id: settings_toggle_id("show-tag"),
            },
        },
        admin_views::AdminSettingsEditSimpleField {
            label: "Show Month Aggregations".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Checkbox {
                name: "show_month_aggregations".to_string(),
                checked: show_month_aggregations,
                toggle_id: settings_toggle_id("show-month"),
            },
        },
        admin_views::AdminSettingsEditSimpleField {
            label: "Global Table of Contents".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Checkbox {
                name: "global_toc_enabled".to_string(),
                checked: global_toc_enabled,
                toggle_id: settings_toggle_id("global-toc"),
            },
        },
        admin_views::AdminSettingsEditSimpleField {
            label: "Brand Title".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Text {
                name: "brand_title".to_string(),
                value: brand_title,
                placeholder: None,
            },
        },
        admin_views::AdminSettingsEditSimpleField {
            label: "Brand Link".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Text {
                name: "brand_href".to_string(),
                value: brand_href,
                placeholder: None,
            },
        },
        admin_views::AdminSettingsEditSimpleField {
            label: "Public Site URL".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Text {
                name: "public_site_url".to_string(),
                value: public_site_url,
                placeholder: Some("https://example.com".to_string()),
            },
        },
        admin_views::AdminSettingsEditSimpleField {
            label: "Meta Title".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Text {
                name: "meta_title".to_string(),
                value: meta_title,
                placeholder: None,
            },
        },
        admin_views::AdminSettingsEditSimpleField {
            label: "OG Title".to_string(),
            input: admin_views::AdminSettingsEditInputKind::Text {
                name: "og_title".to_string(),
                value: og_title,
                placeholder: None,
            },
        },
    ];

    let multiline_fields = vec![
        admin_views::AdminSettingsEditMultilineField {
            label: "Footer Copy".to_string(),
            name: "footer_copy".to_string(),
            value: footer_copy,
            rows: 3,
        },
        admin_views::AdminSettingsEditMultilineField {
            label: "Favicon SVG".to_string(),
            name: "favicon_svg".to_string(),
            value: favicon_svg,
            rows: 6,
        },
        admin_views::AdminSettingsEditMultilineField {
            label: "Meta Description".to_string(),
            name: "meta_description".to_string(),
            value: meta_description,
            rows: 3,
        },
        admin_views::AdminSettingsEditMultilineField {
            label: "OG Description".to_string(),
            name: "og_description".to_string(),
            value: og_description,
            rows: 3,
        },
    ];

    admin_views::AdminSettingsEditView {
        heading: "Edit Site Settings".to_string(),
        simple_fields,
        multiline_fields,
        updated_at,
        form_action: SETTINGS_FORM_ACTION.to_string(),
        submit_label: "Save Changes".to_string(),
        enable_live_submit: true,
    }
}

fn summary_text_field(label: &str, value: String) -> admin_views::AdminSettingsSummaryField {
    admin_views::AdminSettingsSummaryField {
        label: label.to_string(),
        value,
        value_kind: admin_views::AdminSettingsSummaryValueKind::Text,
    }
}

fn summary_multiline_field(label: &str, value: String) -> admin_views::AdminSettingsSummaryField {
    admin_views::AdminSettingsSummaryField {
        label: label.to_string(),
        value,
        value_kind: admin_views::AdminSettingsSummaryValueKind::Multiline,
    }
}

fn summary_badge_field(label: &str, enabled: bool) -> admin_views::AdminSettingsSummaryField {
    let (status, badge_label) = if enabled {
        ("enabled", "Enabled")
    } else {
        ("disabled", "Disabled")
    };

    admin_views::AdminSettingsSummaryField {
        label: label.to_string(),
        value: badge_label.to_string(),
        value_kind: admin_views::AdminSettingsSummaryValueKind::Badge {
            status,
            label: badge_label,
        },
    }
}

fn settings_toggle_id(suffix: &str) -> String {
    format!("settings-toggle-{}", suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::UTC;
    use time::OffsetDateTime;

    #[test]
    fn summary_fields_cover_all_non_audit_settings() {
        let record = sample_settings_record();
        let (simple, multiline) = super::summary_fields(&record);

        let simple_labels: Vec<&str> = simple.iter().map(|field| field.label.as_str()).collect();
        let multiline_labels: Vec<&str> =
            multiline.iter().map(|field| field.label.as_str()).collect();

        for expected in [
            "Homepage Size",
            "Admin Page Size",
            "Tag Filter Limit",
            "Month Filter Limit",
            "Timezone",
            "Show Tag Aggregations",
            "Show Month Aggregations",
            "Global Table of Contents",
            "Brand Title",
            "Brand Link",
            "Public Site URL",
            "Meta Title",
            "OG Title",
        ] {
            assert!(
                simple_labels.contains(&expected),
                "missing simple summary field `{expected}`"
            );
        }

        for expected in [
            "Footer Copy",
            "Favicon SVG",
            "Meta Description",
            "OG Description",
        ] {
            assert!(
                multiline_labels.contains(&expected),
                "missing multiline summary field `{expected}`"
            );
        }
    }

    #[test]
    fn edit_view_exposes_all_configurable_fields() {
        let record = sample_settings_record();
        let view = super::edit_view_from_record(&record);

        let input_names: Vec<&str> = view
            .simple_fields
            .iter()
            .map(|field| match &field.input {
                admin_views::AdminSettingsEditInputKind::Number { name, .. } => name.as_str(),
                admin_views::AdminSettingsEditInputKind::Text { name, .. } => name.as_str(),
                admin_views::AdminSettingsEditInputKind::Checkbox { name, .. } => name.as_str(),
            })
            .collect();

        for expected in ["homepage_size", "admin_page_size", "public_site_url"] {
            assert!(
                input_names.contains(&expected),
                "missing edit input `{expected}`"
            );
        }
    }

    fn sample_settings_record() -> SiteSettingsRecord {
        SiteSettingsRecord {
            homepage_size: 10,
            admin_page_size: 20,
            show_tag_aggregations: true,
            show_month_aggregations: false,
            tag_filter_limit: 5,
            month_filter_limit: 6,
            global_toc_enabled: true,
            brand_title: "Soffio".to_string(),
            brand_href: "https://admin.example.com".to_string(),
            footer_copy: "Copyright Soffio".to_string(),
            public_site_url: "https://example.com".to_string(),
            favicon_svg: "<svg></svg>".to_string(),
            timezone: UTC,
            meta_title: "Meta".to_string(),
            meta_description: "Meta description".to_string(),
            og_title: "OG".to_string(),
            og_description: "OG description".to_string(),
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
}
