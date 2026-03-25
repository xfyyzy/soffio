use super::*;

/// Tests that creating/deleting a page immediately reflects in the sitemap.
#[tokio::test]
#[ignore]
#[serial]
async fn live_cache_consistency_sitemap() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let page_slug = format!("sitemap-test-{suf}");

    // 1. Create and publish a page
    let (page_id, _) = post_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/pages",
        &[StatusCode::CREATED],
        json!({
            "title": format!("Sitemap Test Page {suf}"),
            "body_markdown": "# Sitemap Test\n\nThis page tests sitemap cache.",
        }),
    )
    .await?;

    // Update slug to predictable value
    patch_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::OK],
        json!({"slug": &page_slug, "title": format!("Sitemap Test Page {suf}"), "body_markdown": "# Sitemap Test\n\nContent."}),
    )
    .await?;

    // Publish the page
    post_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/pages/{page_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(800)).await;

    // 2. CRITICAL: Sitemap should contain the new page
    let sitemap = get_public_page(&client, &base, "/sitemap.xml").await?;
    assert!(
        sitemap.contains(&page_slug),
        "CACHE INCONSISTENCY: After publishing page, sitemap should contain '/{page_slug}'. \
         This indicates sitemap cache was not invalidated. \
         Got: {}...",
        &sitemap[..sitemap.len().min(2000)]
    );

    // 3. Delete the page
    delete(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;

    // 4. CRITICAL: Sitemap should NOT contain the deleted page
    let sitemap_after = get_public_page(&client, &base, "/sitemap.xml").await?;
    assert!(
        !sitemap_after.contains(&page_slug),
        "CACHE INCONSISTENCY: After deleting page, sitemap should NOT contain '/{page_slug}'. \
         This indicates sitemap cache was not invalidated."
    );

    Ok(())
}

/// Tests cache precision: updating one post should NOT invalidate another post's cache.
///
/// This verifies the "minimal cost" requirement: only affected caches are invalidated.
#[tokio::test]
#[ignore]
#[serial]
async fn live_cache_precision_unrelated_post_unaffected() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let post_a_marker = format!("POST_A_MARKER_{suf}");
    let post_b_marker = format!("POST_B_MARKER_{suf}");

    // 1. Create and publish two posts
    let (post_a_id, post_a_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": format!("Precision Test Post A {suf}"),
            "excerpt": "Post A for precision testing",
            "body_markdown": format!("# Post A\n\n{post_a_marker}"),
        }),
    )
    .await?;

    let (post_b_id, post_b_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": format!("Precision Test Post B {suf}"),
            "excerpt": "Post B for precision testing",
            "body_markdown": format!("# Post B\n\n{post_b_marker}"),
        }),
    )
    .await?;

    // Publish both
    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_a_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;
    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_b_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(800)).await;

    // 2. Verify both posts are accessible
    let post_a_page = get_public_page(&client, &base, &format!("/posts/{post_a_slug}")).await?;
    let post_b_page = get_public_page(&client, &base, &format!("/posts/{post_b_slug}")).await?;
    assert!(
        post_a_page.contains(&post_a_marker),
        "Post A should be accessible"
    );
    assert!(
        post_b_page.contains(&post_b_marker),
        "Post B should be accessible"
    );

    // 3. Update only Post A
    let post_a_updated_marker = format!("POST_A_UPDATED_{suf}");
    request(
        &client,
        &base,
        Method::POST,
        &format!("/api/v1/posts/{post_a_id}/body"),
        &config.keys.write,
        &[StatusCode::OK],
        |r| {
            r.json(
                &json!({"body_markdown": format!("# Post A Updated\n\n{post_a_updated_marker}")}),
            )
        },
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(800)).await;

    // 4. CRITICAL: Post A should show updated content
    let post_a_after = get_public_page(&client, &base, &format!("/posts/{post_a_slug}")).await?;
    assert!(
        post_a_after.contains(&post_a_updated_marker),
        "Post A should show updated content"
    );

    // 5. CRITICAL: Post B should still be accessible with original content
    // (This verifies that Post B's cache was NOT invalidated)
    let post_b_after = get_public_page(&client, &base, &format!("/posts/{post_b_slug}")).await?;
    assert!(
        post_b_after.contains(&post_b_marker),
        "PRECISION ERROR: After updating Post A, Post B should still show its original content. \
         If this fails with a different error (like 404), it might indicate over-invalidation."
    );

    // Cleanup
    delete(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_a_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_b_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;

    Ok(())
}

/// Tests that updating a post title immediately reflects in the Atom feed.
#[tokio::test]
#[ignore]
#[serial]
async fn live_cache_consistency_atom_feed() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let original_title = format!("Atom Test Original {suf}");
    let updated_title = format!("Atom Test Updated {suf}");

    // 1. Create and publish a post
    let (post_id, _) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": &original_title,
            "excerpt": "Testing atom feed cache consistency",
            "body_markdown": "# Atom Test\n\nContent for Atom feed.",
        }),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(800)).await;

    // 2. Verify Atom feed contains original title
    let atom = get_public_page(&client, &base, "/atom.xml").await?;
    assert!(
        atom.contains(&original_title),
        "Atom feed should contain post title '{original_title}'"
    );

    // 3. Update the post title
    request(
        &client,
        &base,
        Method::POST,
        &format!("/api/v1/posts/{post_id}/title"),
        &config.keys.write,
        &[StatusCode::OK],
        |r| r.json(&json!({"title": &updated_title})),
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(800)).await;

    // 4. CRITICAL: Atom feed should immediately show updated title
    let atom_after = get_public_page(&client, &base, "/atom.xml").await?;
    assert!(
        atom_after.contains(&updated_title),
        "CACHE INCONSISTENCY: After title update, Atom feed should show '{updated_title}'. \
         This indicates atom feed cache was not invalidated. \
         Got: {}...",
        &atom_after[..atom_after.len().min(2000)]
    );

    // Cleanup
    delete(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;

    Ok(())
}

/// Tests that updating a page immediately reflects on its detail page.
#[tokio::test]
#[ignore]
#[serial]
async fn live_cache_consistency_page_update() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let original_content = format!("PAGE_ORIGINAL_CONTENT_{suf}");
    let updated_content = format!("PAGE_UPDATED_CONTENT_{suf}");
    let page_slug = format!("cache-test-page-{suf}");

    // 1. Create and publish a page
    let (page_id, _) = post_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/pages",
        &[StatusCode::CREATED],
        json!({
            "title": format!("Cache Test Page {suf}"),
            "body_markdown": format!("# Test Page\n\n{original_content}"),
        }),
    )
    .await?;

    // Update slug to predictable value
    patch_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::OK],
        json!({
            "slug": &page_slug,
            "title": format!("Cache Test Page {suf}"),
            "body_markdown": format!("# Test Page\n\n{original_content}")
        }),
    )
    .await?;

    // Publish
    post_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/pages/{page_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(800)).await;

    // 2. Verify page shows original content
    let page = get_public_page(&client, &base, &format!("/{page_slug}")).await?;
    assert!(
        page.contains(&original_content),
        "Page should show original content '{original_content}'"
    );

    // 3. Update page content
    patch_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::OK],
        json!({
            "slug": &page_slug,
            "title": format!("Cache Test Page {suf}"),
            "body_markdown": format!("# Test Page Updated\n\n{updated_content}")
        }),
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(800)).await;

    // 4. CRITICAL: Page should immediately show updated content
    let page_after = get_public_page(&client, &base, &format!("/{page_slug}")).await?;
    assert!(
        page_after.contains(&updated_content),
        "CACHE INCONSISTENCY: After update, page should show '{updated_content}'. \
         This indicates page cache was not invalidated. \
         Got: {}...",
        &page_after[..page_after.len().min(2000)]
    );

    // Cleanup
    delete(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;

    Ok(())
}
