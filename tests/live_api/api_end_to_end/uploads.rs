use super::*;

pub(super) async fn exercise(ctx: &LiveApiContext<'_>) -> TestResult<UploadFixture> {
    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/uploads",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        ctx.client,
        ctx.base,
        &ctx.config.keys.revoked,
        "/api/v1/uploads",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    let (upload_id, _) = post_multipart(
        ctx.client,
        ctx.base,
        &ctx.config.keys.all,
        "/api/v1/uploads",
        &[StatusCode::CREATED],
        b"hello world".to_vec(),
    )
    .await?;

    post_multipart(
        ctx.client,
        ctx.base,
        &ctx.config.keys.read,
        "/api/v1/uploads",
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        b"nope".to_vec(),
    )
    .await?;

    Ok(UploadFixture { id: upload_id })
}
