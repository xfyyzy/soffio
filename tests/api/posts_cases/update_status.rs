use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_post_status(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a post via service to get the ID
    let post = state
        .posts
        .create_post(
            "test",
            soffio::application::admin::posts::CreatePostCommand {
                title: "status-test".into(),
                excerpt: "excerpt".into(),
                body_markdown: "# body".into(),
                summary_markdown: None,
                status: soffio::domain::types::PostStatus::Draft,
                pinned: false,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create post via service");

    // Update status to published via handler
    let status_payload = PostStatusRequest {
        status: soffio::domain::types::PostStatus::Published,
        scheduled_at: None,
        published_at: Some(OffsetDateTime::now_utc()),
        archived_at: None,
    };

    let _updated = handlers::update_post_status(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(status_payload),
    )
    .await
    .expect("update post status via handler");
}
