#![deny(clippy::all, clippy::pedantic)]

use assert_cmd::Command;
use httpmock::MockServer;
use predicates::str::contains;
use std::io::Write;
use tempfile::NamedTempFile;

fn key_file(contents: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("tmp file");
    file.write_all(contents.as_bytes()).expect("write key");
    file
}

#[test]
fn api_keys_me_works_end_to_end() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method("GET").path("/api/v1/api-keys/me");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"name":"k","prefix":"abc","scopes":[],"status":"active","expires_at":null,"revoked_at":null,"last_used_at":null}"#);
    });

    let key = key_file("cli-test-key");
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("soffio-cli"));
    let assert = cmd
        .env("SOFFIO_SITE_URL", server.base_url())
        .env("SOFFIO_API_KEY_FILE", key.path())
        .arg("api-keys")
        .arg("me")
        .assert()
        .success();

    let output = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(output.contains("\"prefix\": \"abc\""));
    mock.assert();
}

#[test]
fn missing_site_fails_fast() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("soffio-cli"));
    cmd.arg("api-keys")
        .arg("me")
        .env_remove("SOFFIO_SITE_URL")
        .env_remove("SOFFIO_API_KEY")
        .env_remove("SOFFIO_API_KEY_FILE")
        .assert()
        .failure()
        .stderr(contains("MissingSite"));
}
