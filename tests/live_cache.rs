//! Live cache consistency tests against a running soffio instance.
//!
//! - Tests cache invalidation and consistency after write operations.
//! - Marked `#[ignore]` so it only runs after seeding data and starting server.
//! - Reads demo API keys from `tests/api_keys.seed.toml`.
//!
//! **Note**: These tests share a live server instance and L1 cache.
//! They use `#[serial]` to automatically run sequentially.
//!
//! ```sh
//! cargo test --test live_cache -- --ignored
//! ```

#[path = "live_cache/feed_pages.rs"]
mod feed_pages;
#[path = "live_cache/lifecycle.rs"]
mod lifecycle;
#[path = "live_cache/posts.rs"]
mod posts;
#[path = "live_cache/site.rs"]
mod site;

use chrono::Utc;
use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use serial_test::serial;
use std::{collections::HashSet, fs, path::Path, time::Duration};

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize)]
struct SeedConfig {
    base_url: String,
    keys: Keys,
}

#[derive(Deserialize)]
struct Keys {
    #[allow(dead_code)]
    all: String,
    write: String,
    #[allow(dead_code)]
    read: String,
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

/// Fetches a public page without authentication.
async fn get_public_page(client: &Client, base: &str, path: &str) -> TestResult<String> {
    let url = format!("{}{}", base, path);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| map_net_err(e, &url))?;

    if !resp.status().is_success() {
        return Err(format!("GET {} failed with status {}", url, resp.status()).into());
    }

    Ok(resp.text().await.unwrap_or_default())
}

/// Fetches a public page without authentication, returning status code and body.
async fn get_public_page_with_status(
    client: &Client,
    base: &str,
    path: &str,
) -> TestResult<(u16, String)> {
    let url = format!("{}{}", base, path);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| map_net_err(e, &url))?;

    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    Ok((status, body))
}
