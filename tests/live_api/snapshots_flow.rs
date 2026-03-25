use super::*;

/// Live coverage for snapshot APIs: create → list → get → rollback.
#[tokio::test]
#[ignore]
async fn live_api_snapshots_cover_flow() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    // Ensure the seeded "all" key actually carries snapshot scopes; fail fast if not.
    let key_info = get_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/api-keys/me",
        &[StatusCode::OK],
    )
    .await?;
    let scopes: Vec<String> = key_info
        .get("scopes")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    assert!(
        scopes.contains(&"snapshot_read".to_string())
            && scopes.contains(&"snapshot_write".to_string()),
        "Seeded api-keys must include snapshot scopes for live snapshot test"
    );

    let suf = current_suffix();
    let original_title = format!("Live Snapshot Post {suf}");
    let original_excerpt = "snapshot excerpt";
    let original_body = format!("# Snapshot Body {suf}\n\nOriginal content.");

    // Create a post we can snapshot and mutate.
    let (post_id, post_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": original_title,
            "excerpt": original_excerpt,
            "body_markdown": original_body,
        }),
    )
    .await?;

    // Create snapshot (positive) and ensure it shows up in list/get with proper auth.
    let (snapshot_id, _) = post_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/snapshots",
        &[StatusCode::CREATED],
        json!({
            "entity_type": "post",
            "entity_id": post_id,
            "description": "live snapshot coverage",
        }),
    )
    .await?;
    assert!(
        !snapshot_id.is_empty(),
        "snapshot creation should return an id"
    );

    // Scope gate: a key without snapshot scopes should be forbidden.
    let _ = request(
        &client,
        &base,
        Method::GET,
        &format!("/api/v1/snapshots?entity_type=post&entity_id={post_id}"),
        &config.keys.read,
        &[StatusCode::FORBIDDEN],
        |r| r,
    )
    .await?;

    // Authorized list should include the created snapshot id.
    let list_json = get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/snapshots?entity_type=post&entity_id={post_id}"),
        &[StatusCode::OK],
    )
    .await?;
    let list_contains_snapshot = list_json
        .get("items")
        .and_then(Value::as_array)
        .map(|items| {
            items.iter().any(|item| {
                item.get("id")
                    .and_then(Value::as_str)
                    .map(|id| id == snapshot_id)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    assert!(
        list_contains_snapshot,
        "snapshot list should contain the newly created snapshot"
    );

    let snap_json = get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/snapshots/{snapshot_id}"),
        &[StatusCode::OK],
    )
    .await?;
    assert_eq!(
        snap_json.get("id").and_then(Value::as_str).unwrap_or(""),
        snapshot_id,
        "snapshot get should return the same id"
    );

    // Mutate the post so rollback has an effect.
    let mutated_slug = format!("{post_slug}-mut");
    let mutated_title = format!("{original_title} changed");
    let mutated_body = "# mutated body";
    patch_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::OK],
        json!({
            "slug": mutated_slug,
            "title": mutated_title,
            "excerpt": "changed excerpt",
            "body_markdown": mutated_body,
        }),
    )
    .await?;

    let mutated_json = get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::OK],
    )
    .await?;
    assert_eq!(
        mutated_json
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(""),
        mutated_title
    );

    // Rollback using the snapshot.
    let _ = request(
        &client,
        &base,
        Method::POST,
        &format!("/api/v1/snapshots/{snapshot_id}/rollback"),
        &config.keys.all,
        &[StatusCode::OK],
        |r| r,
    )
    .await?;

    // Verify the post is restored to the original snapshot content (including slug).
    let restored_json = get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::OK],
    )
    .await?;
    assert_eq!(
        restored_json
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(""),
        original_title
    );
    assert_eq!(
        restored_json
            .get("excerpt")
            .and_then(Value::as_str)
            .unwrap_or(""),
        original_excerpt
    );
    assert_eq!(
        restored_json
            .get("body_markdown")
            .and_then(Value::as_str)
            .unwrap_or(""),
        original_body
    );
    assert_eq!(
        restored_json
            .get("slug")
            .and_then(Value::as_str)
            .unwrap_or(""),
        post_slug
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
