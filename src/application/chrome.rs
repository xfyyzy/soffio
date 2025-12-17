use std::sync::Arc;

use axum::http::StatusCode;

use crate::application::error::HttpError;
use crate::application::pagination::{NavigationCursor, PageRequest};
use crate::application::repos::{NavigationQueryFilter, NavigationRepo, RepoError, SettingsRepo};
use crate::cache::L0Store;
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
    cache: Option<Arc<L0Store>>,
}

impl ChromeService {
    pub fn new(
        navigation: Arc<dyn NavigationRepo>,
        settings: Arc<dyn SettingsRepo>,
        cache: Option<Arc<L0Store>>,
    ) -> Self {
        Self {
            navigation,
            settings,
            cache,
        }
    }

    pub async fn load(&self) -> Result<LayoutChrome, HttpError> {
        // Record dependencies for L1 cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);
        crate::cache::deps::record(crate::cache::EntityKey::Navigation);

        let settings = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_site_settings() {
                cached
            } else {
                let settings = self
                    .settings
                    .load_site_settings()
                    .await
                    .map_err(|err| repo_failure("load_site_settings", err))?;
                cache.set_site_settings(settings.clone());
                settings
            }
        } else {
            self.settings
                .load_site_settings()
                .await
                .map_err(|err| repo_failure("load_site_settings", err))?
        };

        let navigation_items = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_navigation() {
                cached
            } else {
                let items = load_navigation_items(self.navigation.as_ref()).await?;
                cache.set_navigation(items.clone());
                items
            }
        } else {
            load_navigation_items(self.navigation.as_ref()).await?
        };

        let mut entries = Vec::new();
        for item in navigation_items.into_iter().filter(|item| item.visible) {
            entries.push(map_navigation_item(&item)?);
        }

        let public_site_url = normalize_public_site_url(&settings.public_site_url);

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
                canonical: public_site_url.clone(),
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

async fn load_navigation_items(
    repo: &dyn NavigationRepo,
) -> Result<Vec<NavigationItemRecord>, HttpError> {
    let mut cursor = None;
    let filter = NavigationQueryFilter::default();
    let mut navigation_items = Vec::new();

    loop {
        let page = repo
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

    Ok(navigation_items)
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

fn normalize_public_site_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    format!("{trimmed}/")
}
