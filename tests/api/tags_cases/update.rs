use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_tag(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a tag via service to get the ID
    let tag = state
        .tags
        .create_tag(
            "test",
            soffio::application::admin::tags::CreateTagCommand {
                name: "original-tag".into(),
                description: None,
                pinned: false,
            },
        )
        .await
        .expect("create tag via service");

    // Update the tag via handler
    let update_payload = TagUpdateRequest {
        name: "updated-tag".into(),
        description: Some("Updated description".into()),
        pinned: true,
    };

    let _updated = handlers::update_tag(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(tag.id),
        Json(update_payload),
    )
    .await
    .expect("update tag via handler");
}
