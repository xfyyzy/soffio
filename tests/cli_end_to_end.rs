#![deny(clippy::all, clippy::pedantic)]

use assert_cmd::Command;
use httpmock::MockServer;
use predicates::str::contains;
use std::io::Write;
use tempfile::NamedTempFile;
use uuid::Uuid;

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

#[test]
fn tags_get_by_id_hits_new_endpoint() {
    let server = MockServer::start();
    let tag_id = Uuid::new_v4();
    let mock = server.mock(|when, then| {
        when.method("GET")
            .path(format!("/api/v1/tags/{tag_id}"));
        then.status(200)
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"id":"{tag_id}","slug":"t","name":"Tag","description":null,"pinned":false,"created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}}"#
            ));
    });

    let key = key_file("cli-test-key");
    Command::new(assert_cmd::cargo::cargo_bin!("soffio-cli"))
        .env("SOFFIO_SITE_URL", server.base_url())
        .env("SOFFIO_API_KEY_FILE", key.path())
        .arg("tags")
        .arg("get")
        .arg("--id")
        .arg(tag_id.to_string())
        .assert()
        .success()
        .stdout(contains(tag_id.to_string()));

    mock.assert();
}

#[test]
fn tags_get_by_slug_hits_new_endpoint() {
    let server = MockServer::start();
    let slug = "tag-slug";
    let mock = server.mock(|when, then| {
        when.method("GET")
            .path(format!("/api/v1/tags/slug/{slug}"));
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"id":"00000000-0000-0000-0000-000000000001","slug":"tag-slug","name":"Tag","description":null,"pinned":false,"created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}"#);
    });

    let key = key_file("cli-test-key");
    Command::new(assert_cmd::cargo::cargo_bin!("soffio-cli"))
        .env("SOFFIO_SITE_URL", server.base_url())
        .env("SOFFIO_API_KEY_FILE", key.path())
        .arg("tags")
        .arg("get")
        .arg("--slug")
        .arg(slug)
        .assert()
        .success()
        .stdout(contains(slug));

    mock.assert();
}

#[test]
fn posts_get_by_id_hits_new_endpoint() {
    let server = MockServer::start();
    let post_id = Uuid::new_v4();
    let mock = server.mock(|when, then| {
        when.method("GET").path(format!("/api/v1/posts/{post_id}"));
        then.status(200)
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"id":"{post_id}","slug":"p","title":"Post","excerpt":"e","body_markdown":"b","summary_markdown":null,"status":"draft","pinned":false,"scheduled_at":null,"published_at":null,"archived_at":null,"created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}}"#
            ));
    });

    let key = key_file("cli-test-key");
    Command::new(assert_cmd::cargo::cargo_bin!("soffio-cli"))
        .env("SOFFIO_SITE_URL", server.base_url())
        .env("SOFFIO_API_KEY_FILE", key.path())
        .arg("posts")
        .arg("get")
        .arg("--id")
        .arg(post_id.to_string())
        .assert()
        .success()
        .stdout(contains(post_id.to_string()));

    mock.assert();
}

#[test]
fn navigation_get_by_id_hits_new_endpoint() {
    let server = MockServer::start();
    let nav_id = Uuid::new_v4();
    let mock = server.mock(|when, then| {
        when.method("GET")
            .path(format!("/api/v1/navigation/{nav_id}"));
        then.status(200)
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"id":"{nav_id}","label":"Nav","destination_type":"url","destination_page_id":null,"destination_url":"https://example.com","sort_order":1,"visible":true,"open_in_new_tab":false,"created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}}"#
            ));
    });

    let key = key_file("cli-test-key");
    Command::new(assert_cmd::cargo::cargo_bin!("soffio-cli"))
        .env("SOFFIO_SITE_URL", server.base_url())
        .env("SOFFIO_API_KEY_FILE", key.path())
        .arg("navigation")
        .arg("get")
        .arg("--id")
        .arg(nav_id.to_string())
        .assert()
        .success()
        .stdout(contains(nav_id.to_string()));

    mock.assert();
}

#[test]
fn uploads_get_by_id_hits_new_endpoint() {
    let server = MockServer::start();
    let upload_id = Uuid::new_v4();
    let mock = server.mock(|when, then| {
        when.method("GET")
            .path(format!("/api/v1/uploads/{upload_id}"));
        then.status(200)
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"id":"{upload_id}","filename":"file.txt","content_type":"text/plain","size_bytes":2,"checksum":"abcd","stored_path":"uploads/file.txt","metadata":{{}},"created_at":"2025-01-01T00:00:00Z"}}"#
            ));
    });

    let key = key_file("cli-test-key");
    Command::new(assert_cmd::cargo::cargo_bin!("soffio-cli"))
        .env("SOFFIO_SITE_URL", server.base_url())
        .env("SOFFIO_API_KEY_FILE", key.path())
        .arg("uploads")
        .arg("get")
        .arg("--id")
        .arg(upload_id.to_string())
        .assert()
        .success()
        .stdout(contains(upload_id.to_string()));

    mock.assert();
}

#[test]
fn pages_get_by_id_hits_new_endpoint() {
    let server = MockServer::start();
    let page_id = Uuid::new_v4();
    let mock = server.mock(|when, then| {
        when.method("GET").path(format!("/api/v1/pages/{page_id}"));
        then.status(200)
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"id":"{page_id}","slug":"page","title":"Page","body_markdown":"b","rendered_html":"<p>b</p>","status":"draft","scheduled_at":null,"published_at":null,"archived_at":null,"created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}}"#
            ));
    });

    let key = key_file("cli-test-key");
    Command::new(assert_cmd::cargo::cargo_bin!("soffio-cli"))
        .env("SOFFIO_SITE_URL", server.base_url())
        .env("SOFFIO_API_KEY_FILE", key.path())
        .arg("pages")
        .arg("get")
        .arg("--id")
        .arg(page_id.to_string())
        .assert()
        .success()
        .stdout(contains(page_id.to_string()));

    mock.assert();
}

#[test]
fn snapshots_get_hits_endpoint() {
    let server = MockServer::start();
    let snap_id = Uuid::new_v4();
    let mock = server.mock(|when, then| {
        when.method("GET")
            .path(format!("/api/v1/snapshots/{snap_id}"));
        then.status(200)
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"id":"{snap_id}","entity_type":"post","entity_id":"00000000-0000-0000-0000-000000000001","version":1,"description":null,"schema_version":1,"content":{{}},"created_by":"tester","created_at":"2025-01-01T00:00:00Z"}}"#
            ));
    });

    let key = key_file("cli-test-key");
    Command::new(assert_cmd::cargo::cargo_bin!("soffio-cli"))
        .env("SOFFIO_SITE_URL", server.base_url())
        .env("SOFFIO_API_KEY_FILE", key.path())
        .arg("snapshots")
        .arg("get")
        .arg(snap_id.to_string())
        .assert()
        .success()
        .stdout(contains(snap_id.to_string()));

    mock.assert();
}

#[test]
fn snapshots_list_hits_endpoint() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method("GET")
            .path("/api/v1/snapshots")
            .query_param("limit", "50")
            .query_param("entity_type", "post");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"items":[],"next_cursor":null}"#);
    });

    let key = key_file("cli-test-key");
    Command::new(assert_cmd::cargo::cargo_bin!("soffio-cli"))
        .env("SOFFIO_SITE_URL", server.base_url())
        .env("SOFFIO_API_KEY_FILE", key.path())
        .arg("snapshots")
        .arg("list")
        .arg("--entity-type")
        .arg("post")
        .arg("--limit")
        .arg("50")
        .assert()
        .success();

    mock.assert();
}

#[test]
fn snapshots_create_hits_endpoint() {
    let server = MockServer::start();
    let post_id = Uuid::new_v4();
    let mock = server.mock(|when, then| {
        when.method("POST")
            .path("/api/v1/snapshots")
            .json_body(serde_json::json!({
                "entity_type": "post",
                "entity_id": post_id.to_string(),
                "description": "desc"
            }));
        then.status(201)
            .header("content-type", "application/json")
            .body(r#"{"id":"00000000-0000-0000-0000-000000000002","entity_type":"post","entity_id":"00000000-0000-0000-0000-000000000001","version":1,"description":null,"schema_version":1,"content":{},"created_by":"tester","created_at":"2025-01-01T00:00:00Z"}"#);
    });

    let key = key_file("cli-test-key");
    Command::new(assert_cmd::cargo::cargo_bin!("soffio-cli"))
        .env("SOFFIO_SITE_URL", server.base_url())
        .env("SOFFIO_API_KEY_FILE", key.path())
        .arg("snapshots")
        .arg("create")
        .arg("--entity-type")
        .arg("post")
        .arg("--entity-id")
        .arg(post_id.to_string())
        .arg("--description")
        .arg("desc")
        .assert()
        .success();

    mock.assert();
}

#[test]
fn snapshots_rollback_hits_endpoint() {
    let server = MockServer::start();
    let snap_id = Uuid::new_v4();
    let mock = server.mock(|when, then| {
        when.method("POST")
            .path(format!("/api/v1/snapshots/{snap_id}/rollback"));
        then.status(200)
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"id":"{snap_id}","entity_type":"post","entity_id":"00000000-0000-0000-0000-000000000001","version":1,"description":null,"schema_version":1,"content":{{}},"created_by":"tester","created_at":"2025-01-01T00:00:00Z"}}"#
            ));
    });

    let key = key_file("cli-test-key");
    Command::new(assert_cmd::cargo::cargo_bin!("soffio-cli"))
        .env("SOFFIO_SITE_URL", server.base_url())
        .env("SOFFIO_API_KEY_FILE", key.path())
        .arg("snapshots")
        .arg("rollback")
        .arg(snap_id.to_string())
        .assert()
        .success();

    mock.assert();
}
