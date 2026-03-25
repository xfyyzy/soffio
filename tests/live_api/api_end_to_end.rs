use super::*;

#[tokio::test]
#[ignore]
async fn live_api_end_to_end() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    // TAGS
    let suf = current_suffix();
    let (tag_id, tag_slug) = post_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/tags",
        &[StatusCode::CREATED],
        json!({"name": format!("test-tag-{suf}"), "description": "api test tag"}),
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/tags/{tag_id}"),
        &[StatusCode::OK],
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/tags/slug/{tag_slug}"),
        &[StatusCode::OK],
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.read,
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
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/tags/{tag_id}"),
        &[StatusCode::OK],
        json!({"name": format!("test-tag-{suf}-upd")}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/tags/{tag_id}/pin"),
        &[StatusCode::OK],
        json!({"pinned": true}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/tags/{tag_id}/description"),
        &[StatusCode::OK],
        json!({"description": "live desc"}),
    )
    .await?;

    delete(
        &client,
        &base,
        &config.keys.revoked,
        &format!("/api/v1/tags/{tag_id}"),
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    // POSTS
    get_plain(
        &client,
        &base,
        &config.keys.read,
        "/api/v1/posts",
        &[StatusCode::OK],
    )
    .await?;

    let key_info = get_json(
        &client,
        &base,
        &config.keys.all,
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
        &client,
        &base,
        &config.keys.revoked,
        "/api/v1/posts",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    let (post_id, post_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/posts",
        &[StatusCode::CREATED],
        json!({
            "title": format!("Test Post {suf}"),
            "excerpt": "test excerpt",
            "body_markdown": "# hello\ncontent",
        }),
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::OK],
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/posts/slug/{post_slug}"),
        &[StatusCode::OK],
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.read,
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
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::OK],
        json!({
            "slug": post_slug,
            "title": format!("Test Post {suf} updated"),
            "excerpt": "updated excerpt",
            "body_markdown": "## updated",
        }),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/pin"),
        &[StatusCode::OK],
        json!({"pinned": true}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/body"),
        &[StatusCode::OK],
        json!({"body_markdown": "## body live"}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/title"),
        &[StatusCode::OK],
        json!({"title": format!("Post {suf} partial")}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.read,
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
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.expired,
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
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}/tags"),
        &[StatusCode::NO_CONTENT],
        json!({"tag_ids": [tag_id]}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.expired,
        &format!("/api/v1/posts/{post_id}/tags"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::UNPROCESSABLE_ENTITY,
        ],
        json!({"tag_ids": [tag_id]}),
    )
    .await?;

    get_plain(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/posts/slug/{post_slug}"),
        &[StatusCode::OK, StatusCode::NOT_FOUND],
    )
    .await?;

    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        &format!("/api/v1/posts/slug/{post_slug}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    // PAGES
    get_plain(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/pages",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        "/api/v1/pages",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    let (page_id, page_slug) = post_json(
        &client,
        &base,
        &config.keys.write,
        "/api/v1/pages",
        &[StatusCode::CREATED],
        json!({
            "slug": format!("page-{suf}"),
            "title": format!("Test Page {suf}"),
            "body_markdown": "Hello page",
        }),
    )
    .await?;

    // NAVIGATION
    let (nav_id_1, _) = post_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/navigation",
        &[StatusCode::CREATED],
        json!({
            "label": format!("Nav {suf}"),
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
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id_1}"),
        &[StatusCode::OK],
    )
    .await?;

    // UPLOADS (register via API upload endpoint)
    let upload_bytes = format!("hi-{}", current_suffix()).into_bytes();
    let (upload_id, _) = post_multipart(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/uploads",
        &[StatusCode::CREATED],
        upload_bytes,
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/uploads/{upload_id}"),
        &[StatusCode::OK],
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::OK],
    )
    .await?;

    get_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/pages/slug/{page_slug}"),
        &[StatusCode::OK],
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.read,
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
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::OK],
        json!({
            "slug": page_slug,
            "title": format!("Test Page {suf} updated"),
            "body_markdown": "Updated body",
        }),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/{page_id}/body"),
        &[StatusCode::OK],
        json!({"body_markdown": "Updated body partial"}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.read,
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
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/{page_id}/status"),
        &[StatusCode::OK],
        json!({"status": "published"}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.expired,
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
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/slug/{page_slug}"),
        &[StatusCode::OK, StatusCode::NOT_FOUND],
    )
    .await?;

    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        &format!("/api/v1/pages/slug/{page_slug}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    // NAVIGATION
    get_plain(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/navigation",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        "/api/v1/navigation",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    let (nav_id_2, _) = post_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/navigation",
        &[StatusCode::CREATED],
        json!({
            "label": format!("Nav {suf}"),
            "destination_type": "external",
            "destination_url": "https://example.com",
            "sort_order": 99,
            "visible": true,
        }),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.read,
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
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id_2}"),
        &[StatusCode::OK],
        json!({
            "label": format!("Nav {suf} updated"),
            "destination_type": "external",
            "destination_url": "https://example.com/updated",
            "sort_order": 100,
            "visible": false,
        }),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id_2}/visibility"),
        &[StatusCode::OK],
        json!({"visible": true}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id_2}/sort-order"),
        &[StatusCode::OK],
        json!({"sort_order": 7}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.expired,
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

    // UPLOADS
    get_plain(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/uploads",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        "/api/v1/uploads",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    let (upload_id, _) = post_multipart(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/uploads",
        &[StatusCode::CREATED],
        b"hello world".to_vec(),
    )
    .await?;

    post_multipart(
        &client,
        &base,
        &config.keys.read,
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

    // SITE SETTINGS
    get_plain(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/site/settings",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        "/api/v1/site/settings",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/site/settings",
        &[StatusCode::OK],
        json!({"brand_title": "Soffio"}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/site/settings",
        &[StatusCode::OK],
        json!({"global_toc_enabled": true, "favicon_svg": "<svg/>"}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.read,
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

    // JOBS & AUDIT
    get_plain(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/jobs",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        &client,
        &base,
        &config.keys.revoked,
        "/api/v1/jobs",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    get_plain(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/audit",
        &[StatusCode::OK],
    )
    .await?;
    get_plain(
        &client,
        &base,
        &config.keys.expired,
        "/api/v1/audit",
        &[StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN],
    )
    .await?;

    // CLEANUP (positive delete + negative delete per resource)
    // Delete first navigation
    delete(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id_1}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    // Delete second navigation
    delete(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id_2}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/navigation/{nav_id_2}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    delete(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/uploads/{upload_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        &client,
        &base,
        &config.keys.expired,
        &format!("/api/v1/uploads/{upload_id}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    delete(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/pages/{page_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/pages/{page_id}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    delete(
        &client,
        &base,
        &config.keys.write,
        &format!("/api/v1/posts/{post_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/posts/{post_id}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    delete(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/tags/{tag_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/tags/{tag_id}"),
        &[
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ],
    )
    .await?;

    Ok(())
}
