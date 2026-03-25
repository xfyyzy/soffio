use super::*;

// ============================================================================
// Cache Consistency Tests
// ============================================================================

/// Tests that updating a post via API immediately reflects on the public page.
///
/// This is the core cache consistency test: after updating content,
/// the public-facing page must show the new content without delay.
#[tokio::test]
#[ignore]
#[serial]
#[serial]
async fn live_cache_consistency_post_update() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let original_content = format!("CACHE_TEST_ORIGINAL_{suf}");
    let updated_content = format!("CACHE_TEST_UPDATED_{suf}");

    // 1. Create a post with unique content
    let (post_id, post_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": format!("Cache Test Post {suf}"),
            "excerpt": "Testing cache consistency",
            "body_markdown": format!("# Test\n\n{original_content}"),
        }),
    )
    .await?;

    // 2. Publish the post
    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    // Wait for render job completion
    tokio::time::sleep(Duration::from_millis(800)).await;

    // 3. Verify original content is visible on public page
    let public_path = format!("/posts/{post_slug}");
    let first_fetch = get_public_page(&client, &base, &public_path).await?;
    assert!(
        first_fetch.contains(&original_content),
        "Initial page should contain {original_content}. Got: {}...",
        &first_fetch[..first_fetch.len().min(500)]
    );

    // 4. Update the post body
    request(
        &client,
        &base,
        Method::POST,
        &format!("/api/v1/posts/{post_id}/body"),
        &config.keys.write,
        &[StatusCode::OK],
        |r| r.json(&json!({"body_markdown": format!("# Updated\n\n{updated_content}")})),
    )
    .await?;

    // Wait for render job completion
    tokio::time::sleep(Duration::from_millis(800)).await;

    // 5. CRITICAL: Verify updated content is immediately visible
    let second_fetch = get_public_page(&client, &base, &public_path).await?;
    assert!(
        second_fetch.contains(&updated_content),
        "CACHE INCONSISTENCY: After update, page should show {updated_content}, \
         but still shows old content. This indicates cache invalidation failed. \
         Got: {}...",
        &second_fetch[..second_fetch.len().min(500)]
    );

    // Verify old content is gone
    assert!(
        !second_fetch.contains(&original_content),
        "Old content marker {original_content} should no longer appear after update"
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

/// Tests that creating and publishing a new post immediately appears on the homepage.
#[tokio::test]
#[ignore]
#[serial]
async fn live_cache_consistency_post_create() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let unique_title = format!("New Post Cache Test {suf}");

    // 1. Create and publish a new post
    let (post_id, _post_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": &unique_title,
            "excerpt": "Testing cache consistency on create",
            "body_markdown": "# New Post\n\nContent here.",
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

    // Wait for render job completion
    tokio::time::sleep(Duration::from_millis(800)).await;

    // 2. CRITICAL: Homepage should immediately show the new post
    let homepage = get_public_page(&client, &base, "/").await?;
    assert!(
        homepage.contains(&unique_title),
        "CACHE INCONSISTENCY: After publishing, homepage should show new post '{unique_title}'. \
         This indicates homepage cache was not invalidated. \
         Got: {}...",
        &homepage[..homepage.len().min(1000)]
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
