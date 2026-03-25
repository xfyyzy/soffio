use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_page_content(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a page via service to get the ID
    let page = state
        .pages
        .create_page(
            "test",
            soffio::application::admin::pages::CreatePageCommand {
                slug: None,
                title: "original-page".into(),
                body_markdown: "# original".into(),
                status: soffio::domain::types::PageStatus::Draft,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create page via service");

    // Update the page via handler
    let update_payload = PageUpdateRequest {
        slug: page.slug.clone(),
        title: "updated-page".into(),
        body_markdown: "# updated".into(),
    };

    let _updated = handlers::update_page(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(page.id),
        Json(update_payload),
    )
    .await
    .expect("update page via handler");
}
