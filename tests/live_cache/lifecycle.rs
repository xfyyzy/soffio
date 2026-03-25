use super::*;

/// Tests that deleting a published post returns 404 on detail.
#[tokio::test]
#[ignore]
#[serial]
async fn live_cache_consistency_post_delete() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let unique_title = format!("Delete Test Post {suf}");
    let unique_content = format!("DELETE_TEST_CONTENT_{suf}");

    // 1. Create and publish a post
    let (post_id, post_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": &unique_title,
            "excerpt": "Testing cache consistency on delete",
            "body_markdown": format!("# To Be Deleted\n\n{unique_content}"),
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

    // 2. Verify post detail page is accessible and shows content
    let public_path = format!("/posts/{post_slug}");
    let (status, body) = get_public_page_with_status(&client, &base, &public_path).await?;
    assert_eq!(
        status, 200,
        "Post detail page should return 200 after publishing"
    );
    assert!(
        body.contains(&unique_content),
        "Post detail page should show the post content"
    );

    // 3. Delete the post
    delete(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;

    // 4. CRITICAL: Detail page should immediately return 404
    let (status_after, _) = get_public_page_with_status(&client, &base, &public_path).await?;
    assert_eq!(
        status_after, 404,
        "CACHE INCONSISTENCY: After deleting, detail page should return 404, got {}. \
         This indicates the post cache was not invalidated.",
        status_after
    );

    Ok(())
}

/// Tests that creating a post with a new tag immediately makes the tag page accessible.
///
/// Note: We verify the tag *page* is accessible rather than checking content,
/// because the page layout may vary and post titles might be rendered differently.
#[tokio::test]
#[ignore]
#[serial]
async fn live_cache_consistency_aggregations() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let unique_tag = format!("cachetag{suf}");

    // 1. First, create a tag
    let (tag_id, tag_slug) = post_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/tags",
        &[StatusCode::CREATED],
        json!({"name": &unique_tag}),
    )
    .await?;

    // Use the tag name as slug since API returns empty slug
    let tag_slug = if tag_slug.is_empty() {
        unique_tag.clone()
    } else {
        tag_slug
    };

    // The tag page should not exist yet (no posts with this tag)
    // Accessing it will return 404 or empty content

    // 2. Create and publish a post with this tag
    let (post_id, _post_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": format!("Aggregation Test Post {suf}"),
            "excerpt": "Testing tag aggregation cache",
            "body_markdown": "# Aggregation Test\n\nContent here.",
            "tags": [unique_tag],
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

    // 3. CRITICAL: Tag page should be accessible and return 200
    // (verifying the aggregation cache was updated to include this tag)
    let (status, body) =
        get_public_page_with_status(&client, &base, &format!("/tags/{tag_slug}")).await?;
    assert_eq!(
        status,
        200,
        "CACHE INCONSISTENCY: After publishing post with tag, tag page /tags/{tag_slug} should return 200. \
         Got status {} with body: {}...",
        status,
        &body[..body.len().min(500)]
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
    delete(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/tags/{tag_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;

    Ok(())
}

/// Tests that updating a post title immediately reflects in the RSS feed.
#[tokio::test]
#[ignore]
#[serial]
async fn live_cache_consistency_feed() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let original_title = format!("Feed Test Original {suf}");
    let updated_title = format!("Feed Test Updated {suf}");

    // 1. Create and publish a post
    let (post_id, _) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": &original_title,
            "excerpt": "Testing feed cache consistency",
            "body_markdown": "# Feed Test\n\nContent for RSS.",
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

    // 2. Verify RSS contains original title
    let rss = get_public_page(&client, &base, "/rss.xml").await?;
    assert!(
        rss.contains(&original_title),
        "RSS feed should contain post title '{original_title}'"
    );

    // 3. Update the post title using the title-specific endpoint
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

    // 4. CRITICAL: RSS should immediately show updated title
    let rss_after = get_public_page(&client, &base, "/rss.xml").await?;
    assert!(
        rss_after.contains(&updated_title),
        "CACHE INCONSISTENCY: After title update, RSS feed should show '{updated_title}'. \
         This indicates feed cache was not invalidated. \
         Got: {}...",
        &rss_after[..rss_after.len().min(2000)]
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
