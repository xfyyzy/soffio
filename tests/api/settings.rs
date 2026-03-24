use super::*;

// ============ Settings ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_get_and_patch_settings(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Get settings
    let _settings = handlers::get_settings(State(state.clone()), Extension(principal.clone()))
        .await
        .expect("get settings via handler");

    // Patch settings
    let patch_payload = SettingsPatchRequest {
        brand_title: Some("Updated Title".into()),
        brand_href: None,
        footer_copy: None,
        homepage_size: Some(15),
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
        global_toc_enabled: None,
        favicon_svg: None,
    };

    let _patched = handlers::patch_settings(
        State(state.clone()),
        Extension(principal.clone()),
        Json(patch_payload),
    )
    .await
    .expect("patch settings via handler");
}

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
