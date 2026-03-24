use super::*;

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
