use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn api_can_partial_update_tag(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let tag = state
        .tags
        .create_tag(
            "test",
            soffio::application::admin::tags::CreateTagCommand {
                name: "tag".into(),
                description: Some("desc".into()),
                pinned: false,
            },
        )
        .await
        .expect("create tag");

    handlers::update_tag_pin(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(tag.id),
        Json(TagPinRequest { pinned: true }),
    )
    .await
    .expect("pin tag");

    handlers::update_tag_name(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(tag.id),
        Json(TagNameRequest {
            name: "renamed".into(),
        }),
    )
    .await
    .expect("rename tag");

    handlers::update_tag_description(
        State(state.clone()),
        Extension(principal),
        axum::extract::Path(tag.id),
        Json(TagDescriptionRequest {
            description: Some("new description".into()),
        }),
    )
    .await
    .expect("update tag description");

    let latest = state.tags.find_by_id(tag.id).await.unwrap().unwrap();
    assert!(latest.pinned);
    assert_eq!(latest.name, "renamed");
    assert_eq!(latest.description.as_deref(), Some("new description"));
}
