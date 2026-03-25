use super::*;

#[async_trait]
impl SettingsRepo for StaticContentRepo {
    async fn load_site_settings(&self) -> Result<SiteSettingsRecord, RepoError> {
        Ok(SiteSettingsRecord {
            homepage_size: 6,
            admin_page_size: 20,
            show_tag_aggregations: true,
            show_month_aggregations: true,
            tag_filter_limit: 16,
            month_filter_limit: 16,
            global_toc_enabled: true,
            brand_title: "Soffio".to_string(),
            brand_href: "/".to_string(),
            footer_copy: "Stillness guides the wind; the wind reshapes stillness.".to_string(),
            public_site_url: "http://localhost:3000/".to_string(),
            favicon_svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\"></svg>"
                .to_string(),
            timezone: chrono_tz::Asia::Shanghai,
            meta_title: "Soffio".to_string(),
            meta_description: "Whispers on motion, balance, and form.".to_string(),
            og_title: "Soffio".to_string(),
            og_description: "Traces of motion, balance, and form in continual drift.".to_string(),
            updated_at: OffsetDateTime::UNIX_EPOCH,
        })
    }

    async fn upsert_site_settings(&self, _settings: SiteSettingsRecord) -> Result<(), RepoError> {
        Ok(())
    }
}
