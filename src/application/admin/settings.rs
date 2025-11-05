use std::sync::Arc;

use chrono_tz::Tz;
use serde::Serialize;
use thiserror::Error;
use time::OffsetDateTime;

use crate::application::admin::audit::AdminAuditService;
use crate::application::repos::{RepoError, SettingsRepo};
use crate::domain::entities::SiteSettingsRecord;

#[derive(Debug, Error)]
pub enum AdminSettingsError {
    #[error("{0}")]
    ConstraintViolation(&'static str),
    #[error(transparent)]
    Repo(#[from] RepoError),
}

#[derive(Debug, Clone)]
pub struct UpdateSettingsCommand {
    pub homepage_size: i32,
    pub admin_page_size: i32,
    pub show_tag_aggregations: bool,
    pub show_month_aggregations: bool,
    pub tag_filter_limit: i32,
    pub month_filter_limit: i32,
    pub global_toc_enabled: bool,
    pub brand_title: String,
    pub brand_href: String,
    pub footer_copy: String,
    pub public_site_url: String,
    pub favicon_svg: String,
    pub timezone: Tz,
    pub meta_title: String,
    pub meta_description: String,
    pub og_title: String,
    pub og_description: String,
}

#[derive(Clone)]
pub struct AdminSettingsService {
    repo: Arc<dyn SettingsRepo>,
    audit: AdminAuditService,
}

impl AdminSettingsService {
    pub fn new(repo: Arc<dyn SettingsRepo>, audit: AdminAuditService) -> Self {
        Self { repo, audit }
    }

    pub async fn load(&self) -> Result<SiteSettingsRecord, AdminSettingsError> {
        self.repo
            .load_site_settings()
            .await
            .map_err(AdminSettingsError::from)
    }

    pub async fn update(
        &self,
        actor: &str,
        command: UpdateSettingsCommand,
    ) -> Result<SiteSettingsRecord, AdminSettingsError> {
        ensure_non_empty(&command.brand_title, "brand_title")?;
        ensure_non_empty(&command.brand_href, "brand_href")?;
        ensure_non_empty(&command.public_site_url, "public_site_url")?;
        ensure_non_empty(&command.meta_title, "meta_title")?;
        ensure_non_empty(&command.meta_description, "meta_description")?;
        ensure_non_empty(&command.og_title, "og_title")?;
        ensure_non_empty(&command.og_description, "og_description")?;
        ensure_non_empty(&command.favicon_svg, "favicon_svg")?;

        let mut record = self.repo.load_site_settings().await?;
        record.homepage_size = command.homepage_size;
        record.admin_page_size = command.admin_page_size;
        record.show_tag_aggregations = command.show_tag_aggregations;
        record.show_month_aggregations = command.show_month_aggregations;
        record.tag_filter_limit = command.tag_filter_limit;
        record.month_filter_limit = command.month_filter_limit;
        record.global_toc_enabled = command.global_toc_enabled;
        record.brand_title = command.brand_title;
        record.brand_href = command.brand_href;
        record.footer_copy = command.footer_copy;
        record.public_site_url = command.public_site_url;
        record.favicon_svg = command.favicon_svg;
        record.timezone = command.timezone;
        record.meta_title = command.meta_title;
        record.meta_description = command.meta_description;
        record.og_title = command.og_title;
        record.og_description = command.og_description;
        record.updated_at = OffsetDateTime::now_utc();

        self.repo.upsert_site_settings(record.clone()).await?;
        let latest = self.repo.load_site_settings().await?;

        let snapshot = SettingsSnapshot::from(&latest);
        self.audit
            .record(actor, "settings.update", "settings", None, Some(&snapshot))
            .await?;

        Ok(latest)
    }
}

#[derive(Debug, Serialize)]
struct SettingsSnapshot<'a> {
    homepage_size: i32,
    admin_page_size: i32,
    show_tag_aggregations: bool,
    show_month_aggregations: bool,
    tag_filter_limit: i32,
    month_filter_limit: i32,
    global_toc_enabled: bool,
    brand_title: &'a str,
    brand_href: &'a str,
    public_site_url: &'a str,
    timezone: &'a str,
}

impl<'a> From<&'a SiteSettingsRecord> for SettingsSnapshot<'a> {
    fn from(record: &'a SiteSettingsRecord) -> Self {
        Self {
            homepage_size: record.homepage_size,
            admin_page_size: record.admin_page_size,
            show_tag_aggregations: record.show_tag_aggregations,
            show_month_aggregations: record.show_month_aggregations,
            tag_filter_limit: record.tag_filter_limit,
            month_filter_limit: record.month_filter_limit,
            global_toc_enabled: record.global_toc_enabled,
            brand_title: record.brand_title.as_str(),
            brand_href: record.brand_href.as_str(),
            public_site_url: record.public_site_url.as_str(),
            timezone: record.timezone.name(),
        }
    }
}

fn ensure_non_empty(value: &str, field: &'static str) -> Result<(), AdminSettingsError> {
    if value.trim().is_empty() {
        return Err(AdminSettingsError::ConstraintViolation(field));
    }
    Ok(())
}
