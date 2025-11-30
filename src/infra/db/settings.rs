use async_trait::async_trait;
use time::OffsetDateTime;

use crate::{
    application::repos::{RepoError, SettingsRepo},
    domain::entities::SiteSettingsRecord,
};

use super::{map_sqlx_error, DbTimeZone, PostgresRepositories};

#[derive(sqlx::FromRow)]
struct SiteSettingsRow {
    homepage_size: i32,
    admin_page_size: i32,
    show_tag_aggregations: bool,
    show_month_aggregations: bool,
    tag_filter_limit: i32,
    month_filter_limit: i32,
    global_toc_enabled: bool,
    brand_title: String,
    brand_href: String,
    footer_copy: String,
    public_site_url: String,
    favicon_svg: String,
    timezone: DbTimeZone,
    meta_title: String,
    meta_description: String,
    og_title: String,
    og_description: String,
    updated_at: OffsetDateTime,
}

impl From<SiteSettingsRow> for SiteSettingsRecord {
    fn from(row: SiteSettingsRow) -> Self {
        Self {
            homepage_size: row.homepage_size,
            admin_page_size: row.admin_page_size,
            show_tag_aggregations: row.show_tag_aggregations,
            show_month_aggregations: row.show_month_aggregations,
            tag_filter_limit: row.tag_filter_limit,
            month_filter_limit: row.month_filter_limit,
            global_toc_enabled: row.global_toc_enabled,
            brand_title: row.brand_title,
            brand_href: row.brand_href,
            footer_copy: row.footer_copy,
            public_site_url: row.public_site_url,
            favicon_svg: row.favicon_svg,
            timezone: row.timezone.into(),
            meta_title: row.meta_title,
            meta_description: row.meta_description,
            og_title: row.og_title,
            og_description: row.og_description,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl SettingsRepo for PostgresRepositories {
    async fn load_site_settings(&self) -> Result<SiteSettingsRecord, RepoError> {
        let row = sqlx::query_as!(
            SiteSettingsRow,
            r#"
            SELECT homepage_size,
                   admin_page_size,
                   show_tag_aggregations,
                   show_month_aggregations,
                   tag_filter_limit,
                   month_filter_limit,
                   global_toc_enabled,
                   brand_title,
                   brand_href,
                   footer_copy,
                   public_site_url,
                   favicon_svg,
                   timezone AS "timezone: DbTimeZone",
                   meta_title,
                   meta_description,
                   og_title,
                   og_description,
                   updated_at
            FROM site_settings
            WHERE id = 1
            "#
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        let row = row.ok_or_else(|| RepoError::from_persistence("site settings row missing"))?;

        Ok(SiteSettingsRecord::from(row))
    }

    async fn upsert_site_settings(&self, settings: SiteSettingsRecord) -> Result<(), RepoError> {
        sqlx::query!(
            r#"
            INSERT INTO site_settings (
                id,
                homepage_size,
                admin_page_size,
                show_tag_aggregations,
                show_month_aggregations,
                tag_filter_limit,
                month_filter_limit,
                global_toc_enabled,
                brand_title,
                brand_href,
                footer_copy,
                public_site_url,
                favicon_svg,
                timezone,
                meta_title,
                meta_description,
                og_title,
                og_description,
                updated_at
            ) VALUES (1, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            ON CONFLICT (id) DO UPDATE SET
                homepage_size = EXCLUDED.homepage_size,
                admin_page_size = EXCLUDED.admin_page_size,
                show_tag_aggregations = EXCLUDED.show_tag_aggregations,
                show_month_aggregations = EXCLUDED.show_month_aggregations,
                tag_filter_limit = EXCLUDED.tag_filter_limit,
                month_filter_limit = EXCLUDED.month_filter_limit,
                global_toc_enabled = EXCLUDED.global_toc_enabled,
                brand_title = EXCLUDED.brand_title,
                brand_href = EXCLUDED.brand_href,
                footer_copy = EXCLUDED.footer_copy,
                public_site_url = EXCLUDED.public_site_url,
                favicon_svg = EXCLUDED.favicon_svg,
                timezone = EXCLUDED.timezone,
                meta_title = EXCLUDED.meta_title,
                meta_description = EXCLUDED.meta_description,
                og_title = EXCLUDED.og_title,
                og_description = EXCLUDED.og_description,
                updated_at = EXCLUDED.updated_at
            "#,
            settings.homepage_size,
            settings.admin_page_size,
            settings.show_tag_aggregations,
            settings.show_month_aggregations,
            settings.tag_filter_limit,
            settings.month_filter_limit,
            settings.global_toc_enabled,
            settings.brand_title,
            settings.brand_href,
            settings.footer_copy,
            settings.public_site_url,
            settings.favicon_svg,
            settings.timezone.name(),
            settings.meta_title,
            settings.meta_description,
            settings.og_title,
            settings.og_description,
            settings.updated_at
        )
        .execute(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }
}
