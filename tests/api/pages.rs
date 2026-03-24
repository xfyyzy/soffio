use super::*;

// ============ Pages ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_create_and_list_pages(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let page_payload = PageCreateRequest {
        slug: None,
        title: "test-page".into(),
        body_markdown: "# Page content".into(),
        status: soffio::domain::types::PageStatus::Draft,
        scheduled_at: None,
        published_at: None,
        archived_at: None,
    };

    let (status, created_page) = response_json(
        handlers::create_page(
            State(state.clone()),
            Extension(principal.clone()),
            Json(page_payload),
        )
        .await
        .expect("create page via handler"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let created_page_id = string_field(&created_page, "id").to_string();
    let created_page_slug = string_field(&created_page, "slug").to_string();

    let (status, found_by_id) = response_json(
        handlers::get_page_by_id(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_page_id.parse().unwrap()),
        )
        .await
        .expect("get page by id"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(string_field(&found_by_id, "id"), created_page_id.as_str());

    let (status, found_by_slug) = response_json(
        handlers::get_page(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_page_slug.clone()),
        )
        .await
        .expect("get page by slug"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        string_field(&found_by_slug, "slug"),
        created_page_slug.as_str()
    );

    let _list = handlers::list_pages(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::PageListQuery {
            status: None,
            search: None,
            month: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("list pages via handler");
}

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
