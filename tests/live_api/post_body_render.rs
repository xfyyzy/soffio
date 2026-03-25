use super::*;

/// Tests that patching post body via API immediately renders the new content.
///
/// This test validates the race condition fix: when a post body is updated via
/// the `/api/v1/posts/{id}/body` endpoint, the render job should use the content
/// from the job payload (captured at enqueue time), not re-read from the database.
///
/// Without the fix, a race condition between HTTP pool writes and job pool reads
/// could cause the render job to process stale content.
///
/// Test flow:
/// 1. Create and publish a post with unique initial content
/// 2. Update the post body via the dedicated body endpoint
/// 3. Wait for render job completion
/// 4. Verify the public page shows the NEW content (not stale cached content)
#[tokio::test]
#[ignore]
async fn live_api_post_body_renders_immediately() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let unique_marker_v1 = format!("MARKER_V1_{suf}");
    let unique_marker_v2 = format!("MARKER_V2_{suf}");
    let original_body =
        format!("# Initial Content\n\nThis post contains {unique_marker_v1} for identification.");
    let updated_body =
        format!("# Updated Content\n\nThis post now contains {unique_marker_v2} after patch.");

    // Step 1: Create a post with initial content
    let (post_id, post_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": format!("Race Condition Test {suf}"),
            "excerpt": "Testing that body patches trigger immediate rendering",
            "body_markdown": original_body,
        }),
    )
    .await?;

    // Step 2: Publish the post
    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    // Wait for initial render job
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Step 3: Verify initial content is rendered
    let public_path = format!("/posts/{post_slug}");
    let first_fetch = get_public_page(&client, &base, &public_path).await?;
    assert!(
        first_fetch.contains(&unique_marker_v1),
        "Initial content should contain {unique_marker_v1}. Got: {}...",
        &first_fetch[..first_fetch.len().min(500)]
    );

    // Step 4: Patch the body via dedicated endpoint (simulating soffio-cli patch-body)
    // This is the exact flow that had the race condition bug.
    let (_status, _body) = request(
        &client,
        &base,
        Method::POST,
        &format!("/api/v1/posts/{post_id}/body"),
        &config.keys.write,
        &[StatusCode::OK],
        |r| r.json(&json!({"body_markdown": updated_body})),
    )
    .await?;

    // Wait for render job to complete (should use payload data, not stale DB read)
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Step 5: Fetch the public page and verify NEW content is present
    let second_fetch = get_public_page(&client, &base, &public_path).await?;

    // The critical assertion: the new unique marker MUST be present.
    // If this fails, the render job read stale data (race condition not fixed).
    assert!(
        second_fetch.contains(&unique_marker_v2),
        "RACE CONDITION BUG: After patching body, the page should show {unique_marker_v2}, \
         but it still shows old content. This indicates the render job is reading stale \
         data from the database instead of using the payload content. \
         Got: {}...",
        &second_fetch[..second_fetch.len().min(500)]
    );

    // Also verify the OLD marker is gone
    assert!(
        !second_fetch.contains(&unique_marker_v1),
        "Old content marker {unique_marker_v1} should no longer appear after update"
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
