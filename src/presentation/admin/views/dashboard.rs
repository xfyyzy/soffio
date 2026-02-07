use askama::Template;

use super::AdminLayout;

#[derive(Clone)]
pub struct AdminMetricView {
    pub label: String,
    pub value: u64,
    pub hint: Option<String>,
}

#[derive(Clone)]
pub struct AdminDashboardPanelView {
    pub title: String,
    pub caption: String,
    pub metrics: Vec<AdminMetricView>,
    pub empty_message: String,
}

impl AdminDashboardPanelView {
    pub fn has_metrics(&self) -> bool {
        !self.metrics.is_empty()
    }
}

#[derive(Clone)]
pub struct AdminDashboardView {
    pub title: String,
    pub panels: Vec<AdminDashboardPanelView>,
    pub empty_message: String,
}

impl AdminDashboardView {
    pub fn has_panels(&self) -> bool {
        !self.panels.is_empty()
    }
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
pub struct AdminDashboardTemplate {
    pub view: AdminLayout<AdminDashboardView>,
}
