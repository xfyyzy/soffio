use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_page_status(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a page via service to get the ID
    let page = state
        .pages
        .create_page(
            "test",
            soffio::application::admin::pages::CreatePageCommand {
                slug: None,
                title: "status-page".into(),
                body_markdown: "# content".into(),
                status: soffio::domain::types::PageStatus::Draft,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create page via service");

    // Update status via handler
    let status_payload = PageStatusRequest {
        status: soffio::domain::types::PageStatus::Published,
        scheduled_at: None,
        published_at: Some(OffsetDateTime::now_utc()),
        archived_at: None,
    };

    let _updated = handlers::update_page_status(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(page.id),
        Json(status_payload),
    )
    .await
    .expect("update page status via handler");
}
