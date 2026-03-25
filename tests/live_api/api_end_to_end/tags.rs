use super::*;

pub(super) async fn exercise(ctx: &LiveApiContext<'_>) -> TestResult<TagFixture> {
    let (tag_id, tag_slug) = post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/tags",
        &[StatusCode::CREATED],
        json!({
            "name": format!("test-tag-{}", ctx.suffix),
            "description": "api test tag"
        }),
    )
    .await?;

    get_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/tags/{tag_id}"),
        &[StatusCode::OK],
    )
    .await?;

    get_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/tags/slug/{tag_slug}"),
        &[StatusCode::OK],
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        "/api/v1/tags",
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({"name": "fail-tag"}),
    )
    .await?;

    patch_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/tags/{tag_id}"),
        &[StatusCode::OK],
        json!({"name": format!("test-tag-{}-upd", ctx.suffix)}),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/tags/{tag_id}/pin"),
        &[StatusCode::OK],
        json!({"pinned": true}),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/tags/{tag_id}/description"),
        &[StatusCode::OK],
        json!({"description": "live desc"}),
    )
    .await?;

    delete(
        ctx.client,
        ctx.base,
        &ctx.config.keys.revoked,
        &format!("/api/v1/tags/{tag_id}"),
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    Ok(TagFixture { id: tag_id })
}
