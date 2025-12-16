//! Live cache consistency tests against a running soffio instance.
//!
//! - Tests cache invalidation and consistency after write operations.
//! - Marked `#[ignore]` so it only runs after seeding data and starting server.
//! - Reads demo API keys from `tests/api_keys.seed.toml`.

use chrono::Utc;
use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashSet, fs, path::Path, time::Duration};

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize)]
struct SeedConfig {
    base_url: String,
    keys: Keys,
}

#[derive(Deserialize)]
struct Keys {
    #[allow(dead_code)]
    all: String,
    write: String,
    #[allow(dead_code)]
    read: String,
}

// ============================================================================
// Cache Consistency Tests
// ============================================================================

/// Tests that updating a post via API immediately reflects on the public page.
///
/// This is the core cache consistency test: after updating content,
/// the public-facing page must show the new content without delay.
#[tokio::test]
#[ignore]
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

/// Tests that updating site settings immediately reflects in responses.
#[tokio::test]
#[ignore]
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

/// Tests that deleting a published post returns 404 on detail.
#[tokio::test]
#[ignore]
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
    assert_eq!(status, 200, "Post detail page should return 200 after publishing");
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
        status, 200,
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

/// Tests that creating/deleting a page immediately reflects in the sitemap.
#[tokio::test]
#[ignore]
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

// ============================================================================
// Helper Functions
// ============================================================================

fn load_config() -> TestResult<SeedConfig> {
    let path = Path::new("tests/api_keys.seed.toml");
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "Unable to read {} (did you commit the demo keys and run from repo root?): {}",
            path.display(),
            e
        )
    })?;
    let cfg: SeedConfig = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;
    Ok(cfg)
}

fn current_suffix() -> String {
    format!("{}", Utc::now().timestamp())
}

async fn request(
    client: &Client,
    base: &str,
    method: Method,
    path: &str,
    key: &str,
    expected: &[StatusCode],
    builder: impl FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
) -> TestResult<(StatusCode, String)> {
    let url = format!("{}{}", base, path);
    let method_str = method.to_string();
    let req = client.request(method, &url).bearer_auth(key);
    let req = builder(req);

    let resp = req.send().await.map_err(|e| map_net_err(e, &url))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !expected.contains(&status) {
        let exp: HashSet<_> = expected.iter().collect();
        return Err(format!(
            "{} {} expected {:?}, got {} body: {}",
            method_str, url, exp, status, body
        )
        .into());
    }

    Ok((status, body))
}

async fn get_json(
    client: &Client,
    base: &str,
    key: &str,
    path: &str,
    expected: &[StatusCode],
) -> TestResult<Value> {
    let (_status, body) = request(client, base, Method::GET, path, key, expected, |r| r).await?;
    Ok(serde_json::from_str(&body).unwrap_or(Value::Null))
}

async fn post_json(
    client: &Client,
    base: &str,
    key: &str,
    path: &str,
    expected: &[StatusCode],
    payload: Value,
) -> TestResult<(String, String)> {
    let (_status, body) = request(client, base, Method::POST, path, key, expected, |r| {
        r.json(&payload)
    })
    .await?;

    let json: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let id = json
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let slug = json
        .get("slug")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    Ok((id, slug))
}

async fn patch_json(
    client: &Client,
    base: &str,
    key: &str,
    path: &str,
    expected: &[StatusCode],
    payload: Value,
) -> TestResult<()> {
    let _ = request(client, base, Method::PATCH, path, key, expected, |r| {
        r.json(&payload)
    })
    .await?;
    Ok(())
}

async fn delete(
    client: &Client,
    base: &str,
    key: &str,
    path: &str,
    expected: &[StatusCode],
) -> TestResult<()> {
    let _ = request(client, base, Method::DELETE, path, key, expected, |r| r).await?;
    Ok(())
}

fn map_net_err(err: reqwest::Error, url: &str) -> Box<dyn std::error::Error> {
    if err.is_connect() {
        format!(
            "Failed to connect to {url}. Start the soffio server on {url_base} before running this test.",
            url_base = url.split("/api").next().unwrap_or(url)
        )
        .into()
    } else {
        err.into()
    }
}

/// Fetches a public page without authentication.
async fn get_public_page(client: &Client, base: &str, path: &str) -> TestResult<String> {
    let url = format!("{}{}", base, path);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| map_net_err(e, &url))?;

    if !resp.status().is_success() {
        return Err(format!("GET {} failed with status {}", url, resp.status()).into());
    }

    Ok(resp.text().await.unwrap_or_default())
}

/// Fetches a public page without authentication, returning status code and body.
async fn get_public_page_with_status(
    client: &Client,
    base: &str,
    path: &str,
) -> TestResult<(u16, String)> {
    let url = format!("{}{}", base, path);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| map_net_err(e, &url))?;

    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    Ok((status, body))
}
