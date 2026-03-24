use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn api_can_delete_tag(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a tag via service to get the ID
    let tag = state
        .tags
        .create_tag(
            "test",
            soffio::application::admin::tags::CreateTagCommand {
                name: "deletable-tag".into(),
                description: None,
                pinned: false,
            },
        )
        .await
        .expect("create tag via service");

    // Delete the tag via handler
    let _deleted = handlers::delete_tag(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(tag.id),
    )
    .await
    .expect("delete tag via handler");
}
