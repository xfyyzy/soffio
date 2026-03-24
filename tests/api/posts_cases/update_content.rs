use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_post_content(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a post via service to get the ID
    let post = state
        .posts
        .create_post(
            "test",
            soffio::application::admin::posts::CreatePostCommand {
                title: "original-title".into(),
                excerpt: "original".into(),
                body_markdown: "# original".into(),
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

    // Update the post via handler
    let update_payload = PostUpdateRequest {
        slug: post.slug.clone(),
        title: "updated-title".into(),
        excerpt: "updated".into(),
        body_markdown: "# updated".into(),
        summary_markdown: None,
        pinned: true,
    };

    let _updated = handlers::update_post(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(update_payload),
    )
    .await
    .expect("update post via handler");
}
