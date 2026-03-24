use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn api_settings_patch_includes_toc_and_favicon(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let patch_payload = SettingsPatchRequest {
        brand_title: None,
        brand_href: None,
        footer_copy: None,
        homepage_size: None,
        admin_page_size: None,
        show_tag_aggregations: None,
        show_month_aggregations: None,
        tag_filter_limit: None,
        month_filter_limit: None,
        timezone: None,
        meta_title: None,
        meta_description: None,
        og_title: None,
        og_description: None,
        public_site_url: None,
        global_toc_enabled: Some(true),
        favicon_svg: Some("<svg></svg>".into()),
    };

    handlers::patch_settings(
        State(state.clone()),
        Extension(principal),
        Json(patch_payload),
    )
    .await
    .expect("patch settings toc/favicon");

    // Reload from repo to assert persisted values
    let latest = state.settings.load().await.unwrap();
    assert!(latest.global_toc_enabled);
    assert_eq!(latest.favicon_svg, "<svg></svg>");
}
