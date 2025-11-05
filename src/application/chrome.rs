use std::sync::Arc;

use axum::http::StatusCode;

use crate::application::error::HttpError;
use crate::application::pagination::{NavigationCursor, PageRequest};
use crate::application::repos::{NavigationQueryFilter, NavigationRepo, RepoError, SettingsRepo};
use crate::domain::entities::NavigationItemRecord;
use crate::domain::types::NavigationDestinationType;
use crate::presentation::views::{
    BrandView, FooterView, LayoutChrome, NavigationLinkView, NavigationView, PageMetaView,
};

const SOURCE: &str = "application::chrome::ChromeService";

#[derive(Clone)]
pub struct ChromeService {
    navigation: Arc<dyn NavigationRepo>,
    settings: Arc<dyn SettingsRepo>,
}

impl ChromeService {
    pub fn new(navigation: Arc<dyn NavigationRepo>, settings: Arc<dyn SettingsRepo>) -> Self {
        Self {
            navigation,
            settings,
        }
    }

    pub async fn load(&self) -> Result<LayoutChrome, HttpError> {
        let settings = self
            .settings
            .load_site_settings()
            .await
            .map_err(|err| repo_failure("load_site_settings", err))?;

        let mut cursor = None;
        let filter = NavigationQueryFilter::default();
        let mut navigation_items = Vec::new();

        loop {
            let page = self
                .navigation
                .list_navigation(None, &filter, PageRequest::new(100, cursor))
                .await
                .map_err(|err| repo_failure("list_navigation", err))?;
            navigation_items.extend(page.items);

            cursor = match page.next_cursor {
                Some(token) => {
                    let decoded = NavigationCursor::decode(&token)
                        .map_err(|err| repo_failure("decode_navigation_cursor", err.into()))?;
                    Some(decoded)
                }
                None => break,
            };
        }

        let mut entries = Vec::new();
        for item in navigation_items.into_iter().filter(|item| item.visible) {
            entries.push(map_navigation_item(&item)?);
        }

        let chrome = LayoutChrome {
            brand: BrandView {
                title: settings.brand_title.clone(),
                href: settings.brand_href.clone(),
            },
            navigation: NavigationView { entries },
            footer: FooterView {
                copy: settings.footer_copy.clone(),
            },
            meta: PageMetaView {
                title: settings.meta_title.clone(),
                description: settings.meta_description.clone(),
                og_title: settings.og_title.clone(),
                og_description: settings.og_description.clone(),
            },
        };

        Ok(chrome)
    }
}

fn repo_failure(operation: &'static str, err: RepoError) -> HttpError {
    HttpError::new(
        SOURCE,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to load site chrome",
        format!("{operation} failed: {err}"),
    )
}

fn map_navigation_item(item: &NavigationItemRecord) -> Result<NavigationLinkView, HttpError> {
    match item.destination_type {
        NavigationDestinationType::Internal => {
            let slug = item.destination_page_slug.as_deref().ok_or_else(|| {
                HttpError::new(
                    SOURCE,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Navigation item misconfigured",
                    format!("Internal link `{}` missing destination slug", item.label),
                )
            })?;

            let mut href = String::from("/");
            if !slug.is_empty() {
                href.push_str(slug);
            }

            Ok(NavigationLinkView {
                label: item.label.clone(),
                href,
                target: None,
                rel: None,
            })
        }
        NavigationDestinationType::External => {
            let url = item.destination_url.as_deref().ok_or_else(|| {
                HttpError::new(
                    SOURCE,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Navigation item misconfigured",
                    format!("External link `{}` missing destination url", item.label),
                )
            })?;

            let mut link = NavigationLinkView {
                label: item.label.clone(),
                href: url.to_string(),
                target: None,
                rel: None,
            };

            link.target = Some("_blank".to_string());
            link.rel = Some("noopener noreferrer".to_string());

            Ok(link)
        }
    }
}
