use super::*;

pub(super) async fn exercise(
    ctx: &LiveApiContext<'_>,
    tag: &TagFixture,
    post: &PostFixture,
    page_bootstrap: &PageBootstrapFixture,
    navigation: &NavigationFixture,
    upload: &UploadFixture,
) -> TestResult<()> {
    delete(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/navigation/{}", page_bootstrap.nav_id_1),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/navigation/{}", navigation.nav_id_2),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        &format!("/api/v1/navigation/{}", navigation.nav_id_2),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    delete(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/uploads/{}", upload.id),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        ctx.client,
        ctx.base,
        &ctx.config.keys.expired,
        &format!("/api/v1/uploads/{}", upload.id),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    delete(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        &format!("/api/v1/pages/{}", page_bootstrap.page.id),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        &format!("/api/v1/pages/{}", page_bootstrap.page.id),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    delete(
        ctx.client,
        ctx.base,
        &ctx.config.keys.write,
        &format!("/api/v1/posts/{}", post.id),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        &format!("/api/v1/posts/{}", post.id),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    delete(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        &format!("/api/v1/tags/{}", tag.id),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        &format!("/api/v1/tags/{}", tag.id),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    Ok(())
}
