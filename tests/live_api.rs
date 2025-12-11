//! Live end-to-end API coverage against a running soffio instance.
//!
//! - Reads demo API keys from `tests/api_keys.seed.toml` (committed, non-sensitive).
//! - Sends real HTTP requests to the public endpoint (`base_url` in the config).
//! - Marked `#[ignore]` so it only runs manually after seeding data and starting the server.

use chrono::Utc;
use reqwest::{Client, Method, StatusCode, multipart};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashSet, fs, path::Path, process::Command, time::Duration};
use tokio::task::spawn_blocking;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Brief delay to allow cache invalidation to propagate.
/// The middleware invalidates synchronously, but we add a small margin for safety.
const CACHE_PROPAGATION_DELAY: Duration = Duration::from_millis(100);

#[derive(Deserialize)]
struct SeedConfig {
    base_url: String,
    keys: Keys,
}

#[derive(Deserialize)]
struct Keys {
    all: String,
    write: String,
    read: String,
    revoked: String,
    expired: String,
}

#[tokio::test]
#[ignore]
async fn live_api_end_to_end() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    // TAGS
    let suf = current_suffix();
    let (tag_id, tag_slug) = post_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/tags",
        &[StatusCode::CREATED],
        json!({"name": format!("test-tag-{suf}"), "description": "api test tag"}),
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/tags/{tag_id}"),
        &[StatusCode::OK],
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/tags/slug/{tag_slug}"),
        &[StatusCode::OK],
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.read,
        "/api/v1/tags",
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({"name": "fail-tag"}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/tags/{tag_id}"),
        &[StatusCode::OK],
        json!({"name": format!("test-tag-{suf}-upd")}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/tags/{tag_id}/pin"),
        &[StatusCode::OK],
        json!({"pinned": true}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/tags/{tag_id}/description"),
        &[StatusCode::OK],
        json!({"description": "live desc"}),
    )
    .await?;

    delete(
        &client,
        &base,
        &config.keys.revoked,
        &format!("/api/v1/tags/{tag_id}"),
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    // POSTS
    get_plain(
        &client,
        &base,
        &config.keys.read,
        "/api/v1/posts",
        &[StatusCode::OK],
    )
    .await?;

    let key_info = get_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/api-keys/me",
        &[StatusCode::OK],
    )
    .await?;
    let scopes = key_info
        .get("scopes")
        .and_then(Value::as_array)
        .unwrap_or(&vec![])
        .len();
    assert!(scopes > 0, "expected at least one scope");
    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        "/api/v1/posts",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    let (post_id, post_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": format!("Test Post {suf}"),
            "excerpt": "test excerpt",
            "body_markdown": "# hello\ncontent",
        }),
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::OK],
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/posts/slug/{post_slug}"),
        &[StatusCode::OK],
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.read,
        "/api/v1/posts",
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({"title": "fail"}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::OK],
        json!({
            "slug": post_slug,
            "title": format!("Test Post {suf} updated"),
            "excerpt": "updated excerpt",
            "body_markdown": "## updated",
        }),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/pin"),
        &[StatusCode::OK],
        json!({"pinned": true}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/body"),
        &[StatusCode::OK],
        json!({"body_markdown": "## body live"}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/title"),
        &[StatusCode::OK],
        json!({"title": format!("Post {suf} partial")}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/posts/{post_id}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({
            "slug": post_slug,
            "title": "nope",
            "excerpt": "no",
            "body_markdown": "no",
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

    post_json(
        &client,
        &base,
        &config.keys.expired,
        &format!("/api/v1/posts/{post_id}/status"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
        ],
        json!({"status": "published"}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/tags"),
        &[StatusCode::NO_CONTENT],
        json!({"tag_ids": [tag_id]}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.expired,
        &format!("/api/v1/posts/{post_id}/tags"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({"tag_ids": [tag_id]}),
    )
    .await?;

    get_plain(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/posts/slug/{post_slug}"),
        &[StatusCode::OK, StatusCode::NOT_FOUND],
    )
    .await?;

    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        &format!("/api/v1/posts/slug/{post_slug}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    // PAGES
    get_plain(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/pages",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        "/api/v1/pages",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    let (page_id, page_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/pages",
        &[StatusCode::CREATED],
        json!({
            "slug": format!("page-{suf}"),
            "title": format!("Test Page {suf}"),
            "body_markdown": "Hello page",
        }),
    )
    .await?;

    // NAVIGATION
    let (nav_id_1, _) = post_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/navigation",
        &[StatusCode::CREATED],
        json!({
            "label": format!("Nav {suf}"),
            "destination_type": "external",
            "destination_url": "https://example.com",
            "destination_page_id": null,
            "sort_order": 1,
            "visible": true,
            "open_in_new_tab": false
        }),
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id_1}"),
        &[StatusCode::OK],
    )
    .await?;

    // UPLOADS (register via API upload endpoint)
    let upload_bytes = format!("hi-{}", current_suffix()).into_bytes();
    let (upload_id, _) = post_multipart(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/uploads",
        &[StatusCode::CREATED],
        upload_bytes,
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/uploads/{upload_id}"),
        &[StatusCode::OK],
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::OK],
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/pages/slug/{page_slug}"),
        &[StatusCode::OK],
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.read,
        "/api/v1/pages",
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({"slug": "bad", "title": "bad", "body_markdown": "bad"}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::OK],
        json!({
            "slug": page_slug,
            "title": format!("Test Page {suf} updated"),
            "body_markdown": "Updated body",
        }),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/{page_id}/body"),
        &[StatusCode::OK],
        json!({"body_markdown": "Updated body partial"}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/pages/{page_id}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({
            "slug": page_slug,
            "title": "x",
            "body_markdown": "x",
        }),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/{page_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.expired,
        &format!("/api/v1/pages/{page_id}/status"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
        ],
        json!({"status": "published"}),
    )
    .await?;

    get_plain(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/slug/{page_slug}"),
        &[StatusCode::OK, StatusCode::NOT_FOUND],
    )
    .await?;

    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        &format!("/api/v1/pages/slug/{page_slug}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    // NAVIGATION
    get_plain(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/navigation",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        "/api/v1/navigation",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    let (nav_id_2, _) = post_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/navigation",
        &[StatusCode::CREATED],
        json!({
            "label": format!("Nav {suf}"),
            "destination_type": "external",
            "destination_url": "https://example.com",
            "sort_order": 99,
            "visible": true,
        }),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.read,
        "/api/v1/navigation",
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({
            "label": "fail",
            "destination_type": "external",
            "destination_url": "https://example.com",
            "sort_order": 1,
        }),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id_2}"),
        &[StatusCode::OK],
        json!({
            "label": format!("Nav {suf} updated"),
            "destination_type": "external",
            "destination_url": "https://example.com/updated",
            "sort_order": 100,
            "visible": false,
        }),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id_2}/visibility"),
        &[StatusCode::OK],
        json!({"visible": true}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id_2}/sort-order"),
        &[StatusCode::OK],
        json!({"sort_order": 7}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.expired,
        &format!("/api/v1/navigation/{nav_id_2}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({
            "label": "x",
            "destination_type": "external",
            "destination_url": "https://x",
            "sort_order": 1,
        }),
    )
    .await?;

    // UPLOADS
    get_plain(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/uploads",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        "/api/v1/uploads",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    let (upload_id, _) = post_multipart(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/uploads",
        &[StatusCode::CREATED],
        b"hello world".to_vec(),
    )
    .await?;

    post_multipart(
        &client,
        &base,
        &config.keys.read,
        "/api/v1/uploads",
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        b"nope".to_vec(),
    )
    .await?;

    // SITE SETTINGS
    get_plain(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/site/settings",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        "/api/v1/site/settings",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/site/settings",
        &[StatusCode::OK],
        json!({"brand_title": "Soffio"}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/site/settings",
        &[StatusCode::OK],
        json!({"global_toc_enabled": true, "favicon_svg": "<svg/>"}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.read,
        "/api/v1/site/settings",
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({"brand_title": "Soffio"}),
    )
    .await?;

    // JOBS & AUDIT
    get_plain(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/jobs",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        "/api/v1/jobs",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    get_plain(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/audit",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        &client,
        &base,
        &config.keys.expired,
        "/api/v1/audit",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    // CLEANUP (positive delete + negative delete per resource)
    // Delete first navigation
    delete(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id_1}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    // Delete second navigation
    delete(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id_2}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/navigation/{nav_id_2}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    delete(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/uploads/{upload_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        &client,
        &base,
        &config.keys.expired,
        &format!("/api/v1/uploads/{upload_id}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    delete(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/pages/{page_id}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

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
        &config.keys.read,
        &format!("/api/v1/posts/{post_id}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
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
    delete(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/tags/{tag_id}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    Ok(())
}

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

async fn get_plain(
    client: &Client,
    base: &str,
    key: &str,
    path: &str,
    expected: &[StatusCode],
) -> TestResult<()> {
    let _ = request(client, base, Method::GET, path, key, expected, |r| r).await?;
    Ok(())
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

async fn post_multipart(
    client: &Client,
    base: &str,
    key: &str,
    path: &str,
    expected: &[StatusCode],
    bytes: Vec<u8>,
) -> TestResult<(String, String)> {
    let part = multipart::Part::bytes(bytes)
        .file_name("hello.txt")
        .mime_str("text/plain")
        .map_err(|e| format!("mime error: {e}"))?;
    let form = multipart::Form::new().part("file", part);

    let (_status, body) = request(client, base, Method::POST, path, key, expected, |r| {
        r.multipart(form)
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

/// Counts warm_cache jobs (any state) via the public API.
async fn warm_cache_job_count(client: &Client, base: &str, key: &str) -> TestResult<usize> {
    let json = get_json(
        client,
        base,
        key,
        "/api/v1/jobs?job_type=warm_cache&limit=200",
        &[StatusCode::OK],
    )
    .await?;

    Ok(json
        .get("items")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0))
}

async fn cli_output(args: &[&str], base: &str, key: &str) -> TestResult<(i32, String, String)> {
    let bin = assert_cmd::cargo::cargo_bin!("soffio-cli");
    let args = args.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let base = base.to_string();
    let key = key.to_string();
    let output = spawn_blocking(move || {
        Command::new(bin)
            .env("SOFFIO_SITE_URL", base)
            .env("SOFFIO_API_KEY", key)
            .args(args)
            .output()
    })
    .await
    .map_err(|e| format!("failed to join soffio-cli task: {e}"))?
    .map_err(|e| format!("failed to run soffio-cli: {e}"))?;

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((code, stdout, stderr))
}

async fn cli_json(args: &[&str], base: &str, key: &str) -> TestResult<Value> {
    let (code, stdout, stderr) = cli_output(args, base, key).await?;
    if code != 0 {
        return Err(format!(
            "soffio-cli {:?} failed (code {code}): stderr={stderr}, stdout={stdout}",
            args
        )
        .into());
    }
    serde_json::from_str(&stdout)
        .map_err(|e| format!("failed to parse stdout as JSON: {e}; stdout={stdout}").into())
}

async fn cli_plain(args: &[&str], base: &str, key: &str) -> TestResult<String> {
    let (code, stdout, stderr) = cli_output(args, base, key).await?;
    if code != 0 {
        return Err(format!(
            "soffio-cli {:?} failed (code {code}): stderr={stderr}, stdout={stdout}",
            args
        )
        .into());
    }
    Ok(stdout)
}

async fn cli_expect_fail(args: &[&str], base: &str, key: &str) -> TestResult<()> {
    let (code, _stdout, _stderr) = cli_output(args, base, key).await?;
    if code == 0 {
        return Err(format!("expected soffio-cli {:?} to fail", args).into());
    }
    Ok(())
}

/// Cache consistency test: verifies that API modifications invalidate the public cache.
///
/// This test would have caught the original bug where soffio-cli operations
/// (via the API) did not trigger cache invalidation, causing stale content
/// to be served on the public site.
///
/// Test flow:
/// 1. Create a post via API with unique content
/// 2. Publish the post via API
/// 3. Fetch the public post page to warm the cache
/// 4. Verify the original content is present
/// 5. Update the post content via API (PATCH)
/// 6. Immediately fetch the public page again
/// 7. Verify the updated content is present (cache was invalidated)
#[tokio::test]
#[ignore]
async fn live_api_cache_invalidation_on_update() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let original_title = format!("Cache Test Original {suf}");
    let updated_title = format!("Cache Test Updated {suf}");
    let original_body = format!("# Original Body {suf}\n\nThis is the original content.");
    let updated_body = format!("# Updated Body {suf}\n\nThis content was updated via API.");

    // Step 1: Create a post
    let (post_id, post_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": original_title,
            "excerpt": "Cache invalidation test post",
            "body_markdown": original_body,
        }),
    )
    .await?;

    assert!(!post_id.is_empty(), "post_id should not be empty");
    assert!(!post_slug.is_empty(), "post_slug should not be empty");

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

    // Small delay to ensure publish job completes
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Step 3: Fetch the public page to warm the cache
    let public_path = format!("/posts/{post_slug}");
    let first_fetch = get_public_page(&client, &base, &public_path).await?;

    // Step 4: Verify original content is present
    assert!(
        first_fetch.contains("Original Body") || first_fetch.contains(&original_title),
        "First fetch should contain original content. Got: {}...{}",
        &first_fetch[..first_fetch.len().min(200)],
        if first_fetch.len() > 400 {
            &first_fetch[first_fetch.len() - 200..]
        } else {
            ""
        }
    );

    // Step 5: Update the post via API
    patch_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::OK],
        json!({
            "slug": post_slug,
            "title": updated_title,
            "excerpt": "Updated excerpt",
            "body_markdown": updated_body,
        }),
    )
    .await?;

    // Brief delay to allow cache invalidation to propagate
    tokio::time::sleep(CACHE_PROPAGATION_DELAY).await;

    // Step 6: Immediately fetch the public page again
    let second_fetch = get_public_page(&client, &base, &public_path).await?;

    // Step 7: Verify the UPDATED content is present (cache was invalidated!)
    // If this assertion fails, it means the cache was NOT invalidated by the API update.
    // This is exactly the bug we fixed: API routes were missing the invalidate_admin_writes middleware.
    assert!(
        second_fetch.contains("Updated Body") || second_fetch.contains(&updated_title),
        "CACHE BUG DETECTED: Second fetch should contain UPDATED content, \
         but it still shows stale cached content. \
         This indicates the API middleware is not invalidating the cache. \
         Got: {}...{}",
        &second_fetch[..second_fetch.len().min(200)],
        if second_fetch.len() > 400 {
            &second_fetch[second_fetch.len() - 200..]
        } else {
            ""
        }
    );

    // Cleanup: delete the test post
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

/// Cache consistency test for page updates via API.
#[tokio::test]
#[ignore]
async fn live_api_cache_invalidation_on_page_update() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let original_body = format!("# Original Page {suf}");
    let updated_body = format!("# Updated Page {suf}");

    // Create a page
    let (page_id, page_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/pages",
        &[StatusCode::CREATED],
        json!({
            "title": format!("Cache Test Page {suf}"),
            "body_markdown": original_body,
        }),
    )
    .await?;

    // Publish the page
    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/{page_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Fetch public page to warm cache
    let public_path = format!("/{page_slug}");
    let first_fetch = get_public_page(&client, &base, &public_path).await?;
    assert!(
        first_fetch.contains("Original Page"),
        "First fetch should contain original content"
    );

    // Update via API
    patch_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::OK],
        json!({
            "slug": page_slug,
            "title": format!("Cache Test Page Updated {suf}"),
            "body_markdown": updated_body,
        }),
    )
    .await?;

    tokio::time::sleep(CACHE_PROPAGATION_DELAY).await;

    // Verify cache was invalidated
    let second_fetch = get_public_page(&client, &base, &public_path).await?;
    assert!(
        second_fetch.contains("Updated Page"),
        "CACHE BUG: Page content should be updated after API modification"
    );

    delete(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;

    Ok(())
}

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

/// Verifies that editing a published post triggers cache invalidation/warm **after**
/// the render job finishes, so the public page does not stay cached with empty
/// summary or stale body content.
#[tokio::test]
#[ignore]
async fn live_api_post_edit_warms_cache_after_render() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    let suf = current_suffix();
    let summary_v1 = format!("SUMMARY_V1_{suf}");
    let summary_v2 = format!("SUMMARY_V2_{suf}");
    let body_v1 = format!(
        "# Heading V1 {suf}\n\n## Section One\n\nBody V1 marker {suf}-A\n\n### Sub\n\nMore V1 content."
    );
    let body_v2 = format!(
        "# Heading V2 {suf}\n\n## Section One Updated\n\nBody V2 marker {suf}-B\n\n### Sub Updated\n\nFresh content."
    );
    let title_v1 = format!("Cache Warm Post {suf}");
    let title_v2 = format!("Cache Warm Post Updated {suf}");

    // Create with summary.
    let (post_id, post_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": title_v1,
            "excerpt": "live cache warm test",
            "body_markdown": body_v1,
            "summary_markdown": summary_v1,
        }),
    )
    .await?;

    // Publish.
    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    // Allow initial render/publish to finish.
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Warm cache once.
    let public_path = format!("/posts/{post_slug}");
    let first_fetch = get_public_page(&client, &base, &public_path).await?;
    assert!(
        first_fetch.contains(&summary_v1) && first_fetch.contains(&post_slug),
        "initial render should include v1 summary and slug"
    );

    let warm_jobs_before = warm_cache_job_count(&client, &base, &config.keys.all).await?;

    // Wait past debounce window to ensure a new warm job can be enqueued.
    tokio::time::sleep(Duration::from_secs(6)).await;

    // Update body + summary; keep slug stable.
    patch_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::OK],
        json!({
            "slug": post_slug,
            "title": title_v2,
            "excerpt": "live cache warm test updated",
            "body_markdown": body_v2,
            "summary_markdown": summary_v2,
        }),
    )
    .await?;

    // Poll public page until new summary/body appear (render + post-render invalidation).
    let mut updated_html = String::new();
    for _ in 0..20 {
        let html = get_public_page(&client, &base, &public_path).await?;
        if html.contains(&summary_v2) && html.contains(&format!("{suf}-B")) {
            updated_html = html;
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    assert!(
        !updated_html.is_empty(),
        "public page did not reflect updated summary/body within timeout"
    );
    assert!(
        !updated_html.contains(&summary_v1) && !updated_html.contains(&format!("{suf}-A")),
        "old summary/body markers should be absent after render completes"
    );

    // Warm cache job should have been enqueued after render completion.
    let mut saw_new_warm_job = false;
    for _ in 0..20 {
        let current = warm_cache_job_count(&client, &base, &config.keys.all).await?;
        if current > warm_jobs_before {
            saw_new_warm_job = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(saw_new_warm_job, "expected a new warm_cache job after edit");

    // Cleanup.
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

/// End-to-end coverage of `soffio-cli snapshots` commands against a running server.
#[tokio::test]
#[ignore]
async fn live_cli_snapshots_cover_all_flows() -> TestResult<()> {
    let cfg = load_config()?;
    let base = cfg.base_url.trim_end_matches('/').to_string();
    let all_key = &cfg.keys.all;
    let read_key = &cfg.keys.read;
    let suf = current_suffix();

    // Create a published post with summary.
    let post_title_v1 = format!("CLI Snap Post {suf}");
    let post_title_v2 = format!("CLI Snap Post Updated {suf}");
    let post_body_v1 = format!("# Post V1 {suf}\n\nBody marker V1-{suf}");
    let post_body_v2 = format!("# Post V2 {suf}\n\nBody marker V2-{suf}");
    let post_summary_v1 = format!("Summary V1 {suf}");
    let post_summary_v2 = format!("Summary V2 {suf}");

    let post_json = cli_json(
        &[
            "posts",
            "create",
            "--title",
            &post_title_v1,
            "--excerpt",
            "cli snap test",
            "--body",
            &post_body_v1,
            "--summary",
            &post_summary_v1,
            "--status",
            "published",
        ],
        &base,
        all_key,
    )
    .await?;
    let post_id = post_json
        .get("id")
        .and_then(Value::as_str)
        .ok_or("missing post id")?
        .to_string();
    let post_slug = post_json
        .get("slug")
        .and_then(Value::as_str)
        .ok_or("missing post slug")?
        .to_string();

    // Create a published page.
    let page_title_v1 = format!("CLI Snap Page {suf}");
    let page_title_v2 = format!("CLI Snap Page Updated {suf}");
    let page_body_v1 = format!("# Page V1 {suf}\n\nPage body V1");
    let page_body_v2 = format!("# Page V2 {suf}\n\nPage body V2");

    let page_json = cli_json(
        &[
            "pages",
            "create",
            "--title",
            &page_title_v1,
            "--body",
            &page_body_v1,
            "--status",
            "published",
        ],
        &base,
        all_key,
    )
    .await?;
    let page_id = page_json
        .get("id")
        .and_then(Value::as_str)
        .ok_or("missing page id")?
        .to_string();
    let page_slug = page_json
        .get("slug")
        .and_then(Value::as_str)
        .ok_or("missing page slug")?
        .to_string();

    // Create snapshots for post and page.
    let post_snap = cli_json(
        &[
            "snapshots",
            "create",
            "--entity-type",
            "post",
            "--entity-id",
            &post_id,
            "--description",
            "cli snap post v1",
        ],
        &base,
        all_key,
    )
    .await?;
    let post_snap_id = post_snap
        .get("id")
        .and_then(Value::as_str)
        .ok_or("missing post snapshot id")?
        .to_string();

    let page_snap = cli_json(
        &[
            "snapshots",
            "create",
            "--entity-type",
            "page",
            "--entity-id",
            &page_id,
            "--description",
            "cli snap page v1",
        ],
        &base,
        all_key,
    )
    .await?;
    let page_snap_id = page_snap
        .get("id")
        .and_then(Value::as_str)
        .ok_or("missing page snapshot id")?
        .to_string();

    // List snapshots filtered by entity_id.
    let list_post = cli_json(
        &[
            "snapshots",
            "list",
            "--entity-type",
            "post",
            "--entity-id",
            &post_id,
        ],
        &base,
        all_key,
    )
    .await?;
    let items = list_post
        .get("items")
        .and_then(Value::as_array)
        .ok_or("missing items in list")?;
    assert!(
        items
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some(post_snap_id.as_str())),
        "post snapshot not present in list"
    );

    // Get snapshot detail.
    let snap_detail = cli_json(&["snapshots", "get", &post_snap_id], &base, all_key).await?;
    assert_eq!(
        snap_detail.get("entity_id").and_then(Value::as_str),
        Some(post_id.as_str())
    );
    assert_eq!(
        snap_detail.get("entity_type").and_then(Value::as_str),
        Some("post")
    );

    // Mutate post & page.
    cli_json(
        &[
            "posts",
            "update",
            "--id",
            &post_id,
            "--slug",
            &post_slug,
            "--title",
            &post_title_v2,
            "--excerpt",
            "cli snap test updated",
            "--body",
            &post_body_v2,
            "--summary",
            &post_summary_v2,
        ],
        &base,
        all_key,
    )
    .await?;

    cli_json(
        &[
            "pages",
            "update",
            "--id",
            &page_id,
            "--slug",
            &page_slug,
            "--title",
            &page_title_v2,
            "--body",
            &page_body_v2,
        ],
        &base,
        all_key,
    )
    .await?;

    // Rollback post snapshot and verify content restored.
    let rollback_msg = cli_plain(&["snapshots", "rollback", &post_snap_id], &base, all_key).await?;
    assert!(
        rollback_msg.contains("Rolled back snapshot"),
        "unexpected rollback message: {rollback_msg}"
    );

    let post_after = cli_json(&["posts", "get", "--id", &post_id], &base, all_key).await?;
    let body_after = post_after
        .get("body_markdown")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let summary_after = post_after
        .get("summary_markdown")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        body_after.contains(&post_body_v1),
        "post body not rolled back"
    );
    assert_eq!(summary_after, post_summary_v1);

    // Rollback page snapshot and verify.
    cli_plain(&["snapshots", "rollback", &page_snap_id], &base, all_key).await?;
    let page_after = cli_json(&["pages", "get", "--id", &page_id], &base, all_key).await?;
    let page_body_after = page_after
        .get("body_markdown")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        page_body_after.contains(&page_body_v1),
        "page body not rolled back"
    );

    // Permission check: create snapshot with read-only key should fail.
    cli_expect_fail(
        &[
            "snapshots",
            "create",
            "--entity-type",
            "post",
            "--entity-id",
            &post_id,
        ],
        &base,
        read_key,
    )
    .await?;

    // Cleanup test content.
    let _ = cli_plain(&["posts", "delete", &post_id], &base, all_key).await?;
    let _ = cli_plain(&["pages", "delete", &page_id], &base, all_key).await?;

    Ok(())
}
