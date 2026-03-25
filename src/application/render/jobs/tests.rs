use super::*;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

use super::helpers::persist_sections_and_summary;
use crate::application::repos::{CreatePostParams, PostsWriteRepo};
use crate::domain::types::PostStatus;
use crate::infra::db::{PersistedPostSectionOwned, PostgresRepositories};

/// Verifies that RenderPostJobPayload correctly serializes and deserializes
/// body_markdown and summary_markdown fields. This is critical for the race
/// condition fix: the payload must carry complete content so the worker
/// doesn't need to re-read from the database.
#[test]
fn render_post_payload_carries_complete_content() {
    let payload = RenderPostJobPayload {
        slug: "test-post".into(),
        body_markdown: "# Heading\n\nParagraph with **bold** text.".into(),
        summary_markdown: Some("Summary content here.".into()),
    };

    let json = serde_json::to_string(&payload).unwrap();
    let deserialized: RenderPostJobPayload = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.slug, "test-post");
    assert_eq!(
        deserialized.body_markdown,
        "# Heading\n\nParagraph with **bold** text."
    );
    assert_eq!(
        deserialized.summary_markdown,
        Some("Summary content here.".into())
    );
}

/// Verifies that RenderPostJobPayload handles None summary_markdown correctly.
#[test]
fn render_post_payload_handles_none_summary() {
    let payload = RenderPostJobPayload {
        slug: "no-summary".into(),
        body_markdown: "Body only".into(),
        summary_markdown: None,
    };

    let json = serde_json::to_string(&payload).unwrap();
    let deserialized: RenderPostJobPayload = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.summary_markdown, None);
}

/// Verifies that large markdown content is preserved through serialization.
#[test]
fn render_post_payload_preserves_large_content() {
    let large_body = "# Title\n\n".to_string() + &"Lorem ipsum dolor sit amet. ".repeat(1000);
    let payload = RenderPostJobPayload {
        slug: "large-post".into(),
        body_markdown: large_body.clone(),
        summary_markdown: Some("Short summary".into()),
    };

    let json = serde_json::to_string(&payload).unwrap();
    let deserialized: RenderPostJobPayload = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.body_markdown, large_body);
}

#[sqlx::test(migrations = "./migrations")]
async fn persist_sections_locks_posts_before_sections(pool: PgPool) {
    let repos = PostgresRepositories::new(pool.clone());

    let post = repos
        .create_post(CreatePostParams {
            slug: "lock-order-test".to_string(),
            title: "Lock Order Test".to_string(),
            excerpt: "excerpt".to_string(),
            body_markdown: "body".to_string(),
            status: PostStatus::Draft,
            pinned: false,
            scheduled_at: None,
            published_at: None,
            archived_at: None,
            summary_markdown: None,
            summary_html: None,
        })
        .await
        .expect("create post");

    let initial_section_id = Uuid::new_v4();
    sqlx::query!(
        r#"
            INSERT INTO post_sections (
                id, post_id, parent_id, position, level,
                heading_html, heading_text, body_html,
                contains_code, contains_math, contains_mermaid, anchor_slug
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        initial_section_id,
        post.id,
        Option::<Uuid>::None,
        0i32,
        1i16,
        "<h2>Heading</h2>",
        "Heading",
        "<p>Body</p>",
        false,
        false,
        false,
        "heading"
    )
    .execute(&pool)
    .await
    .expect("insert section");

    let mut lock_tx = pool.begin().await.expect("begin lock tx");
    sqlx::query!(
        r#"
            SELECT id
            FROM posts
            WHERE id = $1
            FOR UPDATE
            "#,
        post.id
    )
    .fetch_one(&mut *lock_tx)
    .await
    .expect("lock post");

    let new_section_id = Uuid::new_v4();
    let sections = vec![PersistedPostSectionOwned {
        id: new_section_id,
        parent_id: None,
        position: 0,
        level: 1,
        heading_html: "<h2>Updated</h2>".to_string(),
        heading_text: "Updated".to_string(),
        body_html: "<p>Updated</p>".to_string(),
        contains_code: false,
        contains_math: false,
        contains_mermaid: false,
        anchor_slug: "updated".to_string(),
    }];

    let mut handle = tokio::spawn({
        let repos = repos.clone();
        let post_id = post.id;
        async move { persist_sections_and_summary(&repos, post_id, &sections, None).await }
    });

    tokio::task::yield_now().await;

    let blocked = timeout(Duration::from_millis(200), &mut handle).await;
    assert!(blocked.is_err(), "persist should wait on post lock");

    lock_tx.commit().await.expect("release lock");

    let result = timeout(Duration::from_secs(2), &mut handle)
        .await
        .expect("persist completes after lock release")
        .expect("task join ok");
    assert!(result.is_ok(), "persist failed: {result:?}");

    let row = sqlx::query!(
        r#"
            SELECT id
            FROM post_sections
            WHERE post_id = $1
            "#,
        post.id
    )
    .fetch_one(&pool)
    .await
    .expect("fetch section");

    assert_eq!(row.id, new_section_id);
}
