use super::*;

pub(super) async fn exercise(
    ctx: &LiveApiContext<'_>,
    tag: &TagFixture,
) -> TestResult<PostFixture> {
    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        "/api/v1/posts",
        &[StatusCode::OK],
    )
    .await?;

    let key_info = get_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/api-keys/me",
        &[StatusCode::OK],
    )
    .await?;
    let scopes = key_info
        .get("scopes")
        .and_then(Value::as_array)
        .unwrap_or(&vec![])
        .len();
    assert!(scopes > 0, "expected at least one scope");

    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.revoked,
        "/api/v1/posts",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    let (post_id, post_slug) = post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": format!("Test Post {}", ctx.suffix),
            "excerpt": "test excerpt",
            "body_markdown": "# hello\ncontent",
        }),
    )
    .await?;

    get_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::OK],
    )
    .await?;

    get_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        &format!("/api/v1/posts/slug/{post_slug}"),
        &[StatusCode::OK],
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        "/api/v1/posts",
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({"title": "fail"}),
    )
    .await?;

    patch_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::OK],
        json!({
            "slug": post_slug,
            "title": format!("Test Post {} updated", ctx.suffix),
            "excerpt": "updated excerpt",
            "body_markdown": "## updated",
        }),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        &format!("/api/v1/posts/{post_id}/pin"),
        &[StatusCode::OK],
        json!({"pinned": true}),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        &format!("/api/v1/posts/{post_id}/body"),
        &[StatusCode::OK],
        json!({"body_markdown": "## body live"}),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        &format!("/api/v1/posts/{post_id}/title"),
        &[StatusCode::OK],
        json!({"title": format!("Post {} partial", ctx.suffix)}),
    )
    .await?;

    patch_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        &format!("/api/v1/posts/{post_id}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({
            "slug": post_slug,
            "title": "nope",
            "excerpt": "no",
            "body_markdown": "no",
        }),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        &format!("/api/v1/posts/{post_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.expired,
        &format!("/api/v1/posts/{post_id}/status"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
        ],
        json!({"status": "published"}),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        &format!("/api/v1/posts/{post_id}/tags"),
        &[StatusCode::NO_CONTENT],
        json!({"tag_ids": [tag.id]}),
    )
    .await?;

    post_json(
        ctx.client,
        ctx.base,
        &ctx.config.keys.expired,
        &format!("/api/v1/posts/{post_id}/tags"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({"tag_ids": [tag.id]}),
    )
    .await?;

    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        &format!("/api/v1/posts/slug/{post_slug}"),
        &[StatusCode::OK, StatusCode::NOT_FOUND],
    )
    .await?;

    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.revoked,
        &format!("/api/v1/posts/slug/{post_slug}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    Ok(PostFixture { id: post_id })
}
