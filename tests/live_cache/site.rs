use super::*;

/// Tests that updating site settings immediately reflects in responses.
#[tokio::test]
#[ignore]
#[serial]
async fn live_cache_consistency_settings_update() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    // 1. Get current settings
    let current_settings = get_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/site/settings",
        &[StatusCode::OK],
    )
    .await?;

    let original_footer = current_settings
        .get("footer_copy")
        .and_then(Value::as_str)
        .unwrap_or("© Soffio")
        .to_string();

    let suf = current_suffix();
    let test_footer = format!("CACHE_TEST_FOOTER_{suf}");

    // 2. Update footer_copy
    patch_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/site/settings",
        &[StatusCode::OK],
        json!({"footer_copy": &test_footer}),
    )
    .await?;

    // 3. CRITICAL: Verify homepage immediately shows new footer
    let homepage = get_public_page(&client, &base, "/").await?;
    assert!(
        homepage.contains(&test_footer),
        "CACHE INCONSISTENCY: After settings update, homepage should show new footer '{test_footer}'. \
         This indicates settings cache was not invalidated. \
         Got: {}...",
        &homepage[..homepage.len().min(2000)]
    );

    // 4. Restore original footer
    patch_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/site/settings",
        &[StatusCode::OK],
        json!({"footer_copy": &original_footer}),
    )
    .await?;

    Ok(())
}

/// Tests that updating navigation immediately reflects on the homepage.
#[tokio::test]
#[ignore]
#[serial]
async fn live_cache_consistency_navigation_update() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let test_label = format!("CacheNav_{suf}");

    // 1. Create a navigation item
    let (nav_id, _) = post_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/navigation",
        &[StatusCode::CREATED],
        json!({
            "label": &test_label,
            "destination_type": "external",
            "destination_url": "https://example.com",
            "visible": true,
            "sort_order": 999,
        }),
    )
    .await?;

    // 2. CRITICAL: Homepage should immediately show the new navigation item
    let homepage = get_public_page(&client, &base, "/").await?;
    assert!(
        homepage.contains(&test_label),
        "CACHE INCONSISTENCY: After adding nav item, homepage should show '{test_label}'. \
         This indicates navigation cache was not invalidated. \
         Got: {}...",
        &homepage[..homepage.len().min(2000)]
    );

    // 3. Cleanup - delete the navigation item
    delete(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;

    // 4. Verify navigation item is removed from homepage
    let homepage_after = get_public_page(&client, &base, "/").await?;
    assert!(
        !homepage_after.contains(&test_label),
        "After deleting nav item, homepage should not show '{test_label}'"
    );

    Ok(())
}
