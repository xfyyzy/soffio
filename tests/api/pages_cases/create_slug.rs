use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn api_page_create_honors_slug_field(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let (status, custom_page) = response_json(
        handlers::create_page(
            State(state.clone()),
            Extension(principal.clone()),
            Json(PageCreateRequest {
                slug: Some("custom-page-slug".into()),
                title: "ignored-title-for-slug".into(),
                body_markdown: "# Page content".into(),
                status: soffio::domain::types::PageStatus::Draft,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            }),
        )
        .await
        .expect("create page with custom slug"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(string_field(&custom_page, "slug"), "custom-page-slug");

    let (status, auto_page) = response_json(
        handlers::create_page(
            State(state.clone()),
            Extension(principal),
            Json(PageCreateRequest {
                slug: None,
                title: "Auto Slug Page".into(),
                body_markdown: "# Page content".into(),
                status: soffio::domain::types::PageStatus::Draft,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            }),
        )
        .await
        .expect("create page with derived slug"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(string_field(&auto_page, "slug"), "auto-slug-page");
}
