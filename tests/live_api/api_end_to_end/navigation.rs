use super::*;

pub(super) async fn exercise(ctx: &LiveApiContext<'_>) -> TestResult<NavigationFixture> {
    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/navigation",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.revoked,
        "/api/v1/navigation",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    let (nav_id_2, _) = post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/navigation",
        &[StatusCode::CREATED],
        json!({
            "label": format!("Nav {}", ctx.suffix),
            "destination_type": "external",
            "destination_url": "https://example.com",
            "sort_order": 99,
            "visible": true,
        }),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        "/api/v1/navigation",
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({
            "label": "fail",
            "destination_type": "external",
            "destination_url": "https://example.com",
            "sort_order": 1,
        }),
    )
    .await?;

    patch_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/navigation/{nav_id_2}"),
        &[StatusCode::OK],
        json!({
            "label": format!("Nav {} updated", ctx.suffix),
            "destination_type": "external",
            "destination_url": "https://example.com/updated",
            "sort_order": 100,
            "visible": false,
        }),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/navigation/{nav_id_2}/visibility"),
        &[StatusCode::OK],
        json!({"visible": true}),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/navigation/{nav_id_2}/sort-order"),
        &[StatusCode::OK],
        json!({"sort_order": 7}),
    )
    .await?;

    patch_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.expired,
        &format!("/api/v1/navigation/{nav_id_2}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({
            "label": "x",
            "destination_type": "external",
            "destination_url": "https://x",
            "sort_order": 1,
        }),
    )
    .await?;

    Ok(NavigationFixture { nav_id_2 })
}
