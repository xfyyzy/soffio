use super::*;

pub(super) async fn exercise(ctx: &LiveApiContext<'_>) -> TestResult<PageBootstrapFixture> {
    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        "/api/v1/pages",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.revoked,
        "/api/v1/pages",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    let (page_id, page_slug) = post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        "/api/v1/pages",
        &[StatusCode::CREATED],
        json!({
            "slug": format!("page-{}", ctx.suffix),
            "title": format!("Test Page {}", ctx.suffix),
            "body_markdown": "Hello page",
        }),
    )
    .await?;

    let (nav_id_1, _) = post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/navigation",
        &[StatusCode::CREATED],
        json!({
            "label": format!("Nav {}", ctx.suffix),
            "destination_type": "external",
            "destination_url": "https://example.com",
            "destination_page_id": null,
            "sort_order": 1,
            "visible": true,
            "open_in_new_tab": false
        }),
    )
    .await?;

    get_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/navigation/{nav_id_1}"),
        &[StatusCode::OK],
    )
    .await?;

    let upload_bytes = format!("hi-{}", current_suffix()).into_bytes();
    let (upload_id, _) = post_multipart(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/uploads",
        &[StatusCode::CREATED],
        upload_bytes,
    )
    .await?;

    get_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/uploads/{upload_id}"),
        &[StatusCode::OK],
    )
    .await?;

    get_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::OK],
    )
    .await?;

    get_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/pages/slug/{page_slug}"),
        &[StatusCode::OK],
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        "/api/v1/pages",
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({"slug": "bad", "title": "bad", "body_markdown": "bad"}),
    )
    .await?;

    patch_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::OK],
        json!({
            "slug": page_slug,
            "title": format!("Test Page {} updated", ctx.suffix),
            "body_markdown": "Updated body",
        }),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        &format!("/api/v1/pages/{page_id}/body"),
        &[StatusCode::OK],
        json!({"body_markdown": "Updated body partial"}),
    )
    .await?;

    patch_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        &format!("/api/v1/pages/{page_id}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({
            "slug": page_slug,
            "title": "x",
            "body_markdown": "x",
        }),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        &format!("/api/v1/pages/{page_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.expired,
        &format!("/api/v1/pages/{page_id}/status"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
        ],
        json!({"status": "published"}),
    )
    .await?;

    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        &format!("/api/v1/pages/slug/{page_slug}"),
        &[StatusCode::OK, StatusCode::NOT_FOUND],
    )
    .await?;

    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.revoked,
        &format!("/api/v1/pages/slug/{page_slug}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    Ok(PageBootstrapFixture {
        page: PageFixture { id: page_id },
        nav_id_1,
    })
}
