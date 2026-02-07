use askama::Template;

use super::AdminLayout;

#[derive(Clone)]
pub struct AdminSettingsSummaryView {
    pub heading: String,
    pub simple_fields: Vec<AdminSettingsSummaryField>,
    pub multiline_fields: Vec<AdminSettingsSummaryField>,
    pub updated_at: String,
    pub edit_href: String,
}

#[derive(Clone)]
pub struct AdminSettingsSummaryField {
    pub label: String,
    pub value: String,
    pub value_kind: AdminSettingsSummaryValueKind,
}

#[derive(Clone)]
pub enum AdminSettingsSummaryValueKind {
    Text,
    Multiline,
    Badge {
        status: &'static str,
        label: &'static str,
    },
}

#[derive(Template)]
#[template(path = "admin/settings.html")]
pub struct AdminSettingsTemplate {
    pub view: AdminLayout<AdminSettingsSummaryView>,
}

#[derive(Template)]
#[template(path = "admin/settings_panel.html")]
pub struct AdminSettingsPanelTemplate {
    pub content: AdminSettingsSummaryView,
}

#[derive(Clone)]
pub struct AdminSettingsEditView {
    pub heading: String,
    pub simple_fields: Vec<AdminSettingsEditSimpleField>,
    pub multiline_fields: Vec<AdminSettingsEditMultilineField>,
    pub updated_at: String,
    pub form_action: String,
    pub submit_label: String,
    pub enable_live_submit: bool,
}

#[derive(Clone)]
pub struct AdminSettingsEditSimpleField {
    pub label: String,
    pub input: AdminSettingsEditInputKind,
}

#[derive(Clone)]
pub enum AdminSettingsEditInputKind {
    Number {
        name: String,
        value: String,
        min: Option<String>,
    },
    Text {
        name: String,
        value: String,
        placeholder: Option<String>,
    },
    Checkbox {
        name: String,
        checked: bool,
        toggle_id: String,
    },
}

#[derive(Clone)]
pub struct AdminSettingsEditMultilineField {
    pub label: String,
    pub name: String,
    pub value: String,
    pub rows: u32,
}

#[derive(Template)]
#[template(path = "admin/settings_edit.html")]
pub struct AdminSettingsEditTemplate {
    pub view: AdminLayout<AdminSettingsEditView>,
}

#[derive(Template)]
#[template(path = "admin/settings_editor_panel.html")]
pub struct AdminSettingsEditPanelTemplate {
    pub content: AdminSettingsEditView,
}
