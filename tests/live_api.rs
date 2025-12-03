//! Live end-to-end API coverage against a running soffio instance.
//!
//! - Reads demo API keys from `tests/api_keys.seed.toml` (committed, non-sensitive).
//! - Sends real HTTP requests to the public endpoint (`base_url` in the config).
//! - Marked `#[ignore]` so it only runs manually after seeding data and starting the server.

use chrono::Utc;
use reqwest::{Client, Method, StatusCode, multipart};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashSet, fs, path::Path};

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize)]
struct SeedConfig {
    base_url: String,
    keys: Keys,
}

#[derive(Deserialize)]
struct Keys {
    all: String,
    write: String,
    read: String,
    revoked: String,
    expired: String,
}

#[tokio::test]
#[ignore]
async fn live_api_end_to_end() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    // TAGS
    let suf = current_suffix();
    let (tag_id, _) = post_json(
        &client,
        &base,
        &config.keys.all,
        "/api/v1/tags",
        &[StatusCode::CREATED],
        json!({"name": format!("test-tag-{suf}"), "description": "api test tag"}),
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
        &format!("/api/v1/posts/{post_id}/title-slug"),
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

    let (nav_id, _) = post_json(
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
        &format!("/api/v1/navigation/{nav_id}"),
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
        &format!("/api/v1/navigation/{nav_id}/visibility"),
        &[StatusCode::OK],
        json!({"visible": true}),
    )
    .await?;

    post_json(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id}/sort-order"),
        &[StatusCode::OK],
        json!({"sort_order": 7}),
    )
    .await?;

    patch_json(
        &client,
        &base,
        &config.keys.expired,
        &format!("/api/v1/navigation/{nav_id}"),
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
    delete(
        &client,
        &base,
        &config.keys.all,
        &format!("/api/v1/navigation/{nav_id}"),
        &[StatusCode::NO_CONTENT],
    )
    .await?;
    delete(
        &client,
        &base,
        &config.keys.read,
        &format!("/api/v1/navigation/{nav_id}"),
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

fn load_config() -> TestResult<SeedConfig> {
    let path = Path::new("tests/api_keys.seed.toml");
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "Unable to read {} (did you commit the demo keys and run from repo root?): {}",
            path.display(),
            e
        )
    })?;
    let cfg: SeedConfig = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;
    Ok(cfg)
}

fn current_suffix() -> String {
    format!("{}", Utc::now().timestamp())
}

async fn request(
    client: &Client,
    base: &str,
    method: Method,
    path: &str,
    key: &str,
    expected: &[StatusCode],
    builder: impl FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
) -> TestResult<(StatusCode, String)> {
    let url = format!("{}{}", base, path);
    let method_str = method.to_string();
    let req = client.request(method, &url).bearer_auth(key);
    let req = builder(req);

    let resp = req.send().await.map_err(|e| map_net_err(e, &url))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !expected.contains(&status) {
        let exp: HashSet<_> = expected.iter().collect();
        return Err(format!(
            "{} {} expected {:?}, got {} body: {}",
            method_str, url, exp, status, body
        )
        .into());
    }

    Ok((status, body))
}

async fn get_plain(
    client: &Client,
    base: &str,
    key: &str,
    path: &str,
    expected: &[StatusCode],
) -> TestResult<()> {
    let _ = request(client, base, Method::GET, path, key, expected, |r| r).await?;
    Ok(())
}

async fn get_json(
    client: &Client,
    base: &str,
    key: &str,
    path: &str,
    expected: &[StatusCode],
) -> TestResult<Value> {
    let (_status, body) = request(client, base, Method::GET, path, key, expected, |r| r).await?;
    Ok(serde_json::from_str(&body).unwrap_or(Value::Null))
}

async fn post_json(
    client: &Client,
    base: &str,
    key: &str,
    path: &str,
    expected: &[StatusCode],
    payload: Value,
) -> TestResult<(String, String)> {
    let (_status, body) = request(client, base, Method::POST, path, key, expected, |r| {
        r.json(&payload)
    })
    .await?;

    let json: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let id = json
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let slug = json
        .get("slug")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    Ok((id, slug))
}

async fn patch_json(
    client: &Client,
    base: &str,
    key: &str,
    path: &str,
    expected: &[StatusCode],
    payload: Value,
) -> TestResult<()> {
    let _ = request(client, base, Method::PATCH, path, key, expected, |r| {
        r.json(&payload)
    })
    .await?;
    Ok(())
}

async fn post_multipart(
    client: &Client,
    base: &str,
    key: &str,
    path: &str,
    expected: &[StatusCode],
    bytes: Vec<u8>,
) -> TestResult<(String, String)> {
    let part = multipart::Part::bytes(bytes)
        .file_name("hello.txt")
        .mime_str("text/plain")
        .map_err(|e| format!("mime error: {e}"))?;
    let form = multipart::Form::new().part("file", part);

    let (_status, body) = request(client, base, Method::POST, path, key, expected, |r| {
        r.multipart(form)
    })
    .await?;

    let json: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let id = json
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let slug = json
        .get("slug")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    Ok((id, slug))
}

async fn delete(
    client: &Client,
    base: &str,
    key: &str,
    path: &str,
    expected: &[StatusCode],
) -> TestResult<()> {
    let _ = request(client, base, Method::DELETE, path, key, expected, |r| r).await?;
    Ok(())
}

fn map_net_err(err: reqwest::Error, url: &str) -> Box<dyn std::error::Error> {
    if err.is_connect() {
        format!(
            "Failed to connect to {url}. Start the soffio server on {url_base} before running this test.",
            url_base = url.split("/api").next().unwrap_or(url)
        )
        .into()
    } else {
        err.into()
    }
}
