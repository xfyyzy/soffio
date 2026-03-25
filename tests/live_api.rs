//! Live end-to-end API coverage against a running soffio instance.
//!
//! - Reads demo API keys from `tests/api_keys.seed.toml` (committed, non-sensitive).
//! - Sends real HTTP requests to the public endpoint (`base_url` in the config).
//! - Marked `#[ignore]` so it only runs manually after seeding data and starting the server.

#[path = "live_api/api_end_to_end.rs"]
mod api_end_to_end;
#[path = "live_api/cli_snapshots.rs"]
mod cli_snapshots;
#[path = "live_api/post_body_render.rs"]
mod post_body_render;
#[path = "live_api/snapshots_flow.rs"]
mod snapshots_flow;

use chrono::Utc;
use reqwest::{Client, Method, StatusCode, multipart};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::Duration,
};
use tokio::task::spawn_blocking;

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

fn soffio_cli_bin() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();

    BIN.get_or_init(|| {
        if let Some(path) = std::env::var_os("CARGO_BIN_EXE_soffio-cli") {
            return PathBuf::from(path);
        }

        let target_triple = current_test_target_triple();
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut build = Command::new("cargo");
        build.current_dir(&workspace_root).args([
            "build",
            "-p",
            "soffio-cli",
            "--bin",
            "soffio-cli",
        ]);
        if let Some(triple) = target_triple.as_deref() {
            build.args(["--target", triple]);
        }
        let status = build
            .status()
            .expect("build soffio-cli binary for integration tests");
        assert!(status.success(), "failed to build soffio-cli test binary");

        let mut bin = workspace_root.join("target");
        if let Some(triple) = target_triple {
            bin = bin.join(triple);
        }
        bin = bin.join("debug").join("soffio-cli");
        if cfg!(windows) {
            bin.set_extension("exe");
        }
        bin
    })
}

fn current_test_target_triple() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let components = exe
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    let target_idx = components
        .iter()
        .position(|component| component == "target")?;
    let candidate = components.get(target_idx + 1)?;
    if candidate == "debug" || candidate == "release" {
        return None;
    }

    Some(candidate.clone())
}

async fn cli_output(args: &[&str], base: &str, key: &str) -> TestResult<(i32, String, String)> {
    let bin = soffio_cli_bin().to_path_buf();
    let args = args.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let base = base.to_string();
    let key = key.to_string();
    let output = spawn_blocking(move || {
        Command::new(bin)
            .env("SOFFIO_SITE_URL", base)
            .env("SOFFIO_API_KEY", key)
            .args(args)
            .output()
    })
    .await
    .map_err(|e| format!("failed to join soffio-cli task: {e}"))?
    .map_err(|e| format!("failed to run soffio-cli: {e}"))?;

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((code, stdout, stderr))
}

async fn cli_json(args: &[&str], base: &str, key: &str) -> TestResult<Value> {
    let (code, stdout, stderr) = cli_output(args, base, key).await?;
    if code != 0 {
        return Err(format!(
            "soffio-cli {:?} failed (code {code}): stderr={stderr}, stdout={stdout}",
            args
        )
        .into());
    }
    serde_json::from_str(&stdout)
        .map_err(|e| format!("failed to parse stdout as JSON: {e}; stdout={stdout}").into())
}

async fn cli_plain(args: &[&str], base: &str, key: &str) -> TestResult<String> {
    let (code, stdout, stderr) = cli_output(args, base, key).await?;
    if code != 0 {
        return Err(format!(
            "soffio-cli {:?} failed (code {code}): stderr={stderr}, stdout={stdout}",
            args
        )
        .into());
    }
    Ok(stdout)
}

async fn cli_expect_fail(args: &[&str], base: &str, key: &str) -> TestResult<()> {
    let (code, _stdout, _stderr) = cli_output(args, base, key).await?;
    if code == 0 {
        return Err(format!("expected soffio-cli {:?} to fail", args).into());
    }
    Ok(())
}
