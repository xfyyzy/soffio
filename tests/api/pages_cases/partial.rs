use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn api_can_partial_update_page(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let page = state
        .pages
        .create_page(
            "test",
            soffio::application::admin::pages::CreatePageCommand {
                slug: None,
                title: "page".into(),
                body_markdown: "hello".into(),
                status: soffio::domain::types::PageStatus::Draft,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create page");

    handlers::update_page_title(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(page.id),
        Json(PageTitleRequest {
            title: "new page".into(),
        }),
    )
    .await
    .expect("update page title");

    let mut latest = state.pages.find_by_id(page.id).await.unwrap().unwrap();
    assert_eq!(latest.title, "new page");

    handlers::update_page_body(
        State(state.clone()),
        Extension(principal),
        axum::extract::Path(page.id),
        Json(PageBodyRequest {
            body_markdown: "updated body".into(),
        }),
    )
    .await
    .expect("update page body");

    latest = state.pages.find_by_id(page.id).await.unwrap().unwrap();
    assert_eq!(latest.body_markdown, "updated body");
}
