#![deny(clippy::all, clippy::pedantic)]

use httpmock::MockServer;
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::args::{ApiKeysAction, ApiKeysCmd, AuditCmd, NavCmd, PostStatusArg, PostsCmd, SettingsCmd, SettingsPatchArgs};
use crate::client::{build_ctx_from_cli, CliError, Ctx};
use crate::handlers::{audit, navigation, posts, settings};

fn ctx(server: &MockServer) -> Ctx {
    Ctx::new(&server.base_url(), "key".into()).expect("ctx")
}

fn tmp_file(contents: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("tmp file");
    std::io::Write::write_all(&mut file, contents.as_bytes()).expect("write tmp");
    file
}

#[test]
fn build_ctx_prefers_key_file() -> Result<(), CliError> {
    let file = tmp_file("file-key");
    let cli = crate::args::Cli {
        site: Some("https://example.com".to_string()),
        key_file: Some(file.path().to_path_buf()),
        api_key_env: Some("env-key".to_string()),
        command: crate::args::Commands::ApiKeys(ApiKeysCmd {
            action: ApiKeysAction::Me,
        }),
    };

    let ctx = build_ctx_from_cli(&cli)?;
    let header = ctx.auth_header()?;
    assert_eq!(header.to_str().expect("header str"), "Bearer file-key");
    Ok(())
}

#[test]
fn build_ctx_errors_without_key() {
    let cli = crate::args::Cli {
        site: Some("https://example.com".to_string()),
        key_file: None,
        api_key_env: None,
        command: crate::args::Commands::ApiKeys(ApiKeysCmd {
            action: ApiKeysAction::Me,
        }),
    };

    let err = build_ctx_from_cli(&cli).expect_err("missing key should fail");
    assert!(matches!(err, CliError::MissingKey));
}

#[test]
fn read_value_prefers_file_over_inline() -> Result<(), CliError> {
    let file = tmp_file("from-file");
    let val = crate::io::read_value(Some("inline".into()), Some(file.path().to_path_buf()))?;
    assert_eq!(val, "from-file");
    Ok(())
}

#[test]
fn parse_time_rejects_invalid() {
    let err = crate::io::parse_time_opt(Some("not-a-date".into())).expect_err("invalid time");
    assert!(matches!(err, CliError::InvalidInput(_)));
}

#[tokio::test]
async fn api_keys_me_hits_endpoint() -> Result<(), CliError> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method("GET").path("/api/v1/api-keys/me");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"name":"k","prefix":"abc","scopes":["post_read"],"status":"active","expires_at":null,"revoked_at":null,"last_used_at":null}"#);
    });

    let ctx = ctx(&server);
    let cmd = ApiKeysCmd {
        action: ApiKeysAction::Me,
    };
    crate::handlers::api_keys::handle(&ctx, cmd).await?;
    mock.assert();
    Ok(())
}

#[tokio::test]
async fn posts_list_hits_endpoint() -> Result<(), CliError> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method("GET")
            .path("/api/v1/posts")
            .query_param("status", "published")
            .query_param("tag", "rust")
            .query_param("limit", "10");
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });

    let ctx = ctx(&server);
    posts::handle(
        &ctx,
        PostsCmd::List {
            status: Some(PostStatusArg::Published),
            tag: Some("rust".into()),
            search: None,
            month: None,
            limit: 10,
            cursor: None,
        },
    )
    .await?;
    mock.assert();
    Ok(())
}

#[tokio::test]
async fn posts_create_reads_body_file() -> Result<(), CliError> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method("POST")
            .path("/api/v1/posts")
            .json_body_partial(r#"{"title":"T","excerpt":"E","body_markdown":"BODY","summary_markdown":"SUM"}"#);
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });

    let body_file = tmp_file("BODY");
    let summary_file = tmp_file("SUM");
    let ctx = ctx(&server);
    posts::handle(
        &ctx,
        PostsCmd::Create {
            title: "T".into(),
            excerpt: "E".into(),
            body: None,
            body_file: Some(body_file.path().to_path_buf()),
            summary: None,
            summary_file: Some(summary_file.path().to_path_buf()),
            status: PostStatusArg::Draft,
            pinned: false,
            scheduled_at: None,
            published_at: None,
            archived_at: None,
        },
    )
    .await?;
    mock.assert();
    Ok(())
}

#[tokio::test]
async fn navigation_patch_open_hits_endpoint() -> Result<(), CliError> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method("POST");
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });

    let ctx = ctx(&server);
    navigation::handle(
        &ctx,
        NavCmd::PatchOpen {
            id: Uuid::nil(),
            open_in_new_tab: true,
        },
    )
    .await?;
    mock.assert();
    Ok(())
}

#[tokio::test]
async fn settings_patch_reads_favicon_file() -> Result<(), CliError> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method("PATCH")
            .path("/api/v1/site/settings")
            .json_body_partial(r#"{"favicon_svg":"<svg></svg>"}"#);
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });

    let favicon = tmp_file("<svg></svg>");
    let ctx = ctx(&server);
    settings::handle(
        &ctx,
        SettingsCmd::Patch(Box::new(SettingsPatchArgs {
            brand_title: None,
            brand_href: None,
            footer_copy: None,
            homepage_size: None,
            admin_page_size: None,
            show_tag_aggregations: None,
            show_month_aggregations: None,
            tag_filter_limit: None,
            month_filter_limit: None,
            timezone: None,
            meta_title: None,
            meta_description: None,
            og_title: None,
            og_description: None,
            public_site_url: None,
            global_toc_enabled: None,
            favicon_svg: None,
            favicon_svg_file: Some(favicon.path().to_path_buf()),
        })),
    )
    .await?;
    mock.assert();
    Ok(())
}

#[tokio::test]
async fn audit_list_filters() -> Result<(), CliError> {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method("GET")
            .path("/api/v1/audit")
            .query_param("actor", "alice")
            .query_param("action", "update_post")
            .query_param("limit", "5");
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });

    let ctx = ctx(&server);
    audit::handle(
        &ctx,
        AuditCmd::List {
            actor: Some("alice".into()),
            action: Some("update_post".into()),
            entity_type: None,
            search: None,
            limit: 5,
            cursor: None,
        },
    )
    .await?;
    mock.assert();
    Ok(())
}
