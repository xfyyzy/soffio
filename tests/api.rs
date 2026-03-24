use axum::body::Body;
use axum::extract::{Extension, Json, Path, Query, State};
use axum::http::{Request, StatusCode};
use axum::middleware;
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use std::sync::Arc;

use sqlx::PgPool;
use time::OffsetDateTime;

use soffio::application::api_keys::IssueApiKeyCommand;
use soffio::domain::api_keys::ApiScope;
use soffio::domain::entities::UploadRecord;
use soffio::infra::http::api::handlers;
use soffio::infra::http::api::models::*;
use soffio::infra::http::api::state::ApiState;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "support/mod.rs"]
mod support;

use support::api_harness::{build_state, response_json, string_field, uuid_field};

#[sqlx::test(migrations = "./migrations")]
async fn api_rate_limit_uses_route_template(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state
        .api_keys
        .authenticate(&token)
        .await
        .expect("authenticate token");

    let state = ApiState {
        rate_limiter: Arc::new(soffio::infra::http::api::rate_limit::ApiRateLimiter::new(
            std::time::Duration::from_secs(60),
            1,
        )),
        ..state
    };

    let app = Router::new()
        .route("/api/v1/posts/{id}", get(|| async { StatusCode::OK }))
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(
            state,
            soffio::infra::http::api::middleware::api_rate_limit,
        ));

    let first_path = format!("/api/v1/posts/{}", Uuid::new_v4());
    let mut first_request = Request::builder()
        .method("GET")
        .uri(first_path)
        .body(Body::empty())
        .expect("build first request");
    first_request.extensions_mut().insert(principal.clone());

    let first_response = app
        .clone()
        .oneshot(first_request)
        .await
        .expect("send first request");
    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(
        first_response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|value| value.to_str().ok()),
        Some("0")
    );

    let second_path = format!("/api/v1/posts/{}", Uuid::new_v4());
    let mut second_request = Request::builder()
        .method("GET")
        .uri(second_path)
        .body(Body::empty())
        .expect("build second request");
    second_request.extensions_mut().insert(principal);

    let second_response = app
        .oneshot(second_request)
        .await
        .expect("send second request");
    assert_eq!(second_response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[path = "api/posts.rs"]
mod posts;

#[path = "api/pages.rs"]
mod pages;

#[path = "api/tags.rs"]
mod tags;

#[path = "api/navigation.rs"]
mod navigation;

// ============ Uploads ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_list_uploads(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let upload = UploadRecord {
        id: Uuid::new_v4(),
        filename: "demo.txt".into(),
        content_type: "text/plain".into(),
        size_bytes: 4,
        checksum: "abcd".into(),
        stored_path: "uploads/demo.txt".into(),
        metadata: soffio::domain::uploads::UploadMetadata::default(),
        created_at: OffsetDateTime::now_utc(),
    };
    state
        .uploads
        .register_upload("tests", upload.clone())
        .await
        .expect("register upload");

    let (status, fetched) = response_json(
        handlers::get_upload(
            State(state.clone()),
            Extension(principal.clone()),
            Path(upload.id),
        )
        .await
        .expect("get upload by id"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(string_field(&fetched, "id"), upload.id.to_string());

    let _list = handlers::list_uploads(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::UploadListQuery {
            search: None,
            content_type: None,
            month: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("list uploads via handler");
}

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

// ============ Jobs ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_list_jobs(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let _list = handlers::list_jobs(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::JobsListQuery {
            state: None,
            job_type: None,
            search: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("list jobs via handler");
}

// ============ Audit ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_list_audit_logs(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Add AuditRead scope for this test
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "audit-test".to_string(),
            description: None,
            scopes: vec![ApiScope::AuditRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    let principal = state.api_keys.authenticate(&issued.token).await.unwrap();

    let _list = handlers::list_audit_logs(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::AuditListQuery {
            actor: None,
            action: None,
            entity_type: None,
            search: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("list audit logs via handler");
}

// ============ API Keys ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_get_api_key_info(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let Json(info) = handlers::get_api_key_info(State(state.clone()), Extension(principal))
        .await
        .expect("get api key info");

    assert_eq!(info.prefix.len(), 12);
    assert!(info.scopes.contains(&ApiScope::PostRead));
    assert_eq!(info.status, soffio::domain::api_keys::ApiKeyStatus::Active);
}

// ============ API Key Scope Granularity ============

#[sqlx::test(migrations = "./migrations")]
async fn api_scope_granularity_post_vs_page(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key with only PostRead scope
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "post-only".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    let principal = state.api_keys.authenticate(&issued.token).await.unwrap();

    // Should be able to list posts
    let _posts = handlers::list_posts(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::PostListQuery {
            status: None,
            search: None,
            tag: None,
            month: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("should be able to list posts with PostRead scope");

    // Should NOT be able to list pages (requires PageRead)
    assert!(
        principal.requires(ApiScope::PageRead).is_err(),
        "PostRead scope should not grant PageRead access"
    );
}

// ============ API Key Authentication Status ============

#[sqlx::test(migrations = "./migrations")]
async fn api_auth_rejects_revoked_key(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "revoke-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    // Revoke the key
    state
        .api_keys
        .revoke(issued.record.id)
        .await
        .expect("revoke should succeed");

    // Authentication should fail with Revoked error
    let result = state.api_keys.authenticate(&issued.token).await;
    assert!(
        result.is_err(),
        "authentication should fail for revoked key"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, soffio::application::api_keys::ApiAuthError::Revoked),
        "should get Revoked error, got: {:?}",
        err
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn api_auth_rejects_expired_key(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key that expires immediately (expires_in = 0 means expires_at = now)
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "expired-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: Some(time::Duration::ZERO),
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    // Small delay to ensure we're past the expires_at time
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Authentication should fail with Expired error
    let result = state.api_keys.authenticate(&issued.token).await;
    assert!(
        result.is_err(),
        "authentication should fail for expired key"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, soffio::application::api_keys::ApiAuthError::Expired),
        "should get Expired error, got: {:?}",
        err
    );
}

// ============ API Key Rotation ============

#[sqlx::test(migrations = "./migrations")]
async fn api_rotate_reactivates_revoked_key(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "rotate-revoke-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    // Revoke the key
    state
        .api_keys
        .revoke(issued.record.id)
        .await
        .expect("revoke should succeed");

    // Rotation should succeed and reactivate the key
    let rotated = state
        .api_keys
        .rotate(issued.record.id)
        .await
        .expect("rotation should succeed for revoked key");

    // The key should now be active
    assert_eq!(
        rotated.record.status,
        soffio::domain::api_keys::ApiKeyStatus::Active,
        "key should be reactivated after rotation"
    );

    // The new token should work for authentication
    let auth_result = state.api_keys.authenticate(&rotated.token).await;
    assert!(
        auth_result.is_ok(),
        "authentication should succeed with rotated token"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn api_rotate_recalculates_expiration_preserves_created_at(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key with 30-day expiration duration
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "rotate-preserve-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: Some(time::Duration::days(30)),
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    let original_created_at = issued.record.created_at;
    let original_expires_in = issued.record.expires_in;
    let original_expires_at = issued.record.expires_at;

    // Small delay to ensure recalculated expires_at is different
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Rotate the key
    let rotated = state
        .api_keys
        .rotate(issued.record.id)
        .await
        .expect("rotation should succeed");

    // created_at and expires_in should be preserved
    assert_eq!(
        rotated.record.created_at, original_created_at,
        "created_at should be preserved after rotation"
    );
    assert_eq!(
        rotated.record.expires_in, original_expires_in,
        "expires_in duration should be preserved after rotation"
    );

    // expires_at should be recalculated (should be later than original)
    assert!(
        rotated.record.expires_at > original_expires_at,
        "expires_at should be recalculated to a later time after rotation"
    );

    // The token should be different
    assert_ne!(
        issued.token, rotated.token,
        "token should change after rotation"
    );

    // Old token should no longer work
    let old_auth = state.api_keys.authenticate(&issued.token).await;
    assert!(old_auth.is_err(), "old token should not authenticate");

    // New token should work
    let new_auth = state.api_keys.authenticate(&rotated.token).await;
    assert!(new_auth.is_ok(), "new token should authenticate");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_snapshots_cover_create_get_list_and_rollback(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create post
    let (status, post_json) = response_json(
        handlers::create_post(
            State(state.clone()),
            Extension(principal.clone()),
            Json(PostCreateRequest {
                title: "snap-post".into(),
                excerpt: "excerpt".into(),
                body_markdown: "# body".into(),
                summary_markdown: None,
                status: soffio::domain::types::PostStatus::Draft,
                pinned: false,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            }),
        )
        .await
        .expect("create post"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let post_id = uuid_field(&post_json, "id");
    let post_slug = string_field(&post_json, "slug").to_string();

    // Tag to ensure tag list restored
    let (_, tag_json) = response_json(
        handlers::create_tag(
            State(state.clone()),
            Extension(principal.clone()),
            Json(TagCreateRequest {
                name: "snap-tag".into(),
                description: None,
                pinned: false,
            }),
        )
        .await
        .expect("create tag"),
    )
    .await;
    let tag_id = uuid_field(&tag_json, "id");
    let status = handlers::replace_post_tags(
        State(state.clone()),
        Extension(principal.clone()),
        Path(post_id),
        Json(PostTagsRequest {
            tag_ids: vec![tag_id],
        }),
    )
    .await
    .expect("attach tag")
    .into_response()
    .status();
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Snapshot (version 1)
    let (status, snap_json) = response_json(
        handlers::create_snapshot(
            State(state.clone()),
            Extension(principal.clone()),
            Json(SnapshotCreateRequest {
                entity_type: soffio::domain::types::SnapshotEntityType::Post,
                entity_id: post_id,
                description: Some("v1".into()),
            }),
        )
        .await
        .expect("create snapshot"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let snapshot_id = uuid_field(&snap_json, "id");

    // Mutate post & tags
    let (status, _) = response_json(
        handlers::update_post(
            State(state.clone()),
            Extension(principal.clone()),
            Path(post_id),
            Json(PostUpdateRequest {
                slug: post_slug.clone(),
                title: "changed".into(),
                excerpt: "changed excerpt".into(),
                body_markdown: "changed body".into(),
                summary_markdown: None,
                pinned: false,
            }),
        )
        .await
        .expect("update post"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let status = handlers::replace_post_tags(
        State(state.clone()),
        Extension(principal.clone()),
        Path(post_id),
        Json(PostTagsRequest { tag_ids: vec![] }),
    )
    .await
    .expect("clear tags")
    .into_response()
    .status();
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Rollback
    let (status, _) = response_json(
        handlers::rollback_snapshot(
            State(state.clone()),
            Extension(principal.clone()),
            Path(snapshot_id),
        )
        .await
        .expect("rollback snapshot"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Verify post restored
    let restored = state
        .posts
        .load_post(post_id)
        .await
        .expect("find post")
        .expect("post exists");
    assert_eq!(restored.title, "snap-post");
    assert_eq!(restored.excerpt, "excerpt");

    let restored_tags: Vec<Uuid> = sqlx::query_scalar::<_, Uuid>(
        "SELECT tag_id FROM post_tags WHERE post_id = $1 ORDER BY tag_id",
    )
    .bind(post_id)
    .fetch_all(state.db.pool())
    .await
    .expect("post tags");
    assert_eq!(restored_tags, vec![tag_id]);

    // List & get
    let (status, list_json) = response_json(
        handlers::list_snapshots(
            State(state.clone()),
            Query(SnapshotListQuery {
                entity_type: Some(soffio::domain::types::SnapshotEntityType::Post),
                entity_id: Some(post_id),
                search: None,
                cursor: None,
                limit: Some(10),
            }),
            Extension(principal.clone()),
        )
        .await
        .expect("list snapshots"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        list_json
            .get("items")
            .and_then(|v| v.as_array())
            .map(Vec::len)
            .unwrap_or(0),
        1
    );

    let (status, get_json) = response_json(
        handlers::get_snapshot(
            State(state.clone()),
            Path(snapshot_id),
            Extension(principal),
        )
        .await
        .expect("get snapshot"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(uuid_field(&get_json, "id"), snapshot_id);
}
