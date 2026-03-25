use super::*;

pub(super) async fn exercise(ctx: &LiveApiContext<'_>) -> TestResult<()> {
    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/site/settings",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.revoked,
        "/api/v1/site/settings",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    patch_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/site/settings",
        &[StatusCode::OK],
        json!({"brand_title": "Soffio"}),
    )
    .await?;

    patch_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/site/settings",
        &[StatusCode::OK],
        json!({"global_toc_enabled": true, "favicon_svg": "<svg/>"}),
    )
    .await?;

    patch_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        "/api/v1/site/settings",
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({"brand_title": "Soffio"}),
    )
    .await?;

    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/jobs",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.revoked,
        "/api/v1/jobs",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/audit",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.expired,
        "/api/v1/audit",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    Ok(())
}
