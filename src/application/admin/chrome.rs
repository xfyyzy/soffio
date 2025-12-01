use std::sync::Arc;

use crate::application::error::HttpError;
use crate::application::repos::{RepoError, SettingsRepo};
use crate::presentation::admin::views::{
    AdminBrandView, AdminChrome, AdminMetaView, AdminNavigationItemView, AdminNavigationView,
};

const SOURCE: &str = "application::admin::chrome::AdminChromeService";
const NAV_ITEMS: &[(&str, &str)] = &[
    ("/", "Dashboard"),
    ("/posts", "Posts"),
    ("/pages", "Pages"),
    ("/tags", "Tags"),
    ("/navigation", "Navigation"),
    ("/uploads", "Uploads"),
    ("/api-keys", "API keys"),
    ("/settings", "Site settings"),
    // TODO: Temporarily hidden from menu - pages and routes still functional
    // ("/jobs", "Jobs"),
    // ("/audit", "Audit log"),
];

#[derive(Clone)]
pub struct AdminChromeService {
    settings: Arc<dyn SettingsRepo>,
}

impl AdminChromeService {
    pub fn new(settings: Arc<dyn SettingsRepo>) -> Self {
        Self { settings }
    }

    pub async fn load(&self, active_path: &str) -> Result<AdminChrome, HttpError> {
        let settings = self
            .settings
            .load_site_settings()
            .await
            .map_err(repo_failure)?;

        let brand = AdminBrandView {
            title: format!("{} Admin", settings.brand_title),
        };

        let public_site_url = normalize_public_site_url(&settings.public_site_url);

        let mut items: Vec<AdminNavigationItemView> = NAV_ITEMS
            .iter()
            .map(|(href, label)| AdminNavigationItemView {
                label: (*label).to_string(),
                href: (*href).to_string(),
                is_active: *href == active_path,
                open_in_new_tab: false,
            })
            .collect();

        items.push(AdminNavigationItemView {
            label: "View site".to_string(),
            href: public_site_url,
            is_active: false,
            open_in_new_tab: true,
        });

        let navigation = AdminNavigationView { items };

        let active_label = navigation
            .items
            .iter()
            .find(|item| item.is_active)
            .map(|item| item.label.as_str())
            .unwrap_or("Dashboard");

        let meta = AdminMetaView {
            title: format!("{} · {}", brand.title, active_label),
            description: "Administrative control centre".to_string(),
        };

        Ok(AdminChrome {
            brand,
            navigation,
            meta,
        })
    }
}

fn repo_failure(err: RepoError) -> HttpError {
    HttpError::new(
        SOURCE,
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to load admin chrome",
        err.to_string(),
    )
}

fn normalize_public_site_url(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
    }
}
