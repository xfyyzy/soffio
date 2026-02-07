use super::*;

#[test]
fn cli_overrides_take_highest_precedence() {
    let mut raw = RawSettings::default();
    raw.server.public_port = Some(4000);
    raw.logging.level = Some("info".to_string());

    let overrides = ServeOverrides {
        public_port: Some(4321),
        log_level: Some("debug".to_string()),
        ..Default::default()
    };

    raw.apply_serve_overrides(&overrides);
    let settings = Settings::from_raw(raw).expect("valid settings");

    assert_eq!(settings.server.public_addr.port(), 4321);
    assert_eq!(settings.logging.level, LevelFilter::DEBUG);
}

#[test]
fn uploads_limit_defaults_to_10_mib() {
    let raw = RawSettings::default();
    let settings = Settings::from_raw(raw).expect("valid settings");
    assert_eq!(
        settings.uploads.max_request_bytes.get(),
        DEFAULT_UPLOAD_REQUEST_LIMIT_BYTES
    );
}

#[test]
fn uploads_limit_can_be_overridden_via_cli() {
    let mut raw = RawSettings::default();
    let overrides = ServeOverrides {
        uploads_max_request_bytes: Some(1_572_864),
        ..Default::default()
    };

    raw.apply_serve_overrides(&overrides);
    let settings = Settings::from_raw(raw).expect("valid settings");
    assert_eq!(settings.uploads.max_request_bytes.get(), 1_572_864);
}

#[test]
fn cli_json_logging_enforces_format() {
    let mut raw = RawSettings::default();
    let overrides = ServeOverrides {
        log_json: Some(true),
        ..Default::default()
    };

    raw.apply_serve_overrides(&overrides);
    let settings = Settings::from_raw(raw).expect("valid settings");

    assert!(matches!(settings.logging.format, LogFormat::Json));
}

#[test]
fn default_to_serve_command() {
    let args = CliArgs::parse_from(["soffio"]);
    let command = args
        .command
        .unwrap_or(Command::Serve(Box::<ServeArgs>::default()));
    assert!(matches!(command, Command::Serve(_)));
}

#[test]
fn parse_renderall_arguments() {
    let args = CliArgs::parse_from([
        "soffio",
        "renderall",
        "--database-url",
        "postgres://example",
        "--posts",
        "--concurrency",
        "8",
    ]);

    match args.command.expect("renderall command") {
        Command::RenderAll(render) => {
            assert_eq!(
                render.overrides.database.database_url.as_deref(),
                Some("postgres://example")
            );
            assert!(render.posts);
            assert!(!render.pages);
            assert_eq!(render.concurrency, 8);
        }
        _ => panic!("wrong command parsed"),
    }
}

#[test]
fn parse_export_arguments() {
    let args = CliArgs::parse_from([
        "soffio",
        "export",
        "--database-url",
        "postgres://example",
        "/tmp/site.toml",
    ]);

    match args.command.expect("export command") {
        Command::ExportSite(export) => {
            assert_eq!(
                export.database.database_url.as_deref(),
                Some("postgres://example")
            );
            assert_eq!(export.file, std::path::Path::new("/tmp/site.toml"));
        }
        _ => panic!("wrong command parsed"),
    }
}

#[test]
fn parse_import_arguments() {
    let args = CliArgs::parse_from([
        "soffio",
        "import",
        "--database-url",
        "postgres://example",
        "/tmp/site.toml",
    ]);

    match args.command.expect("import command") {
        Command::ImportSite(import) => {
            assert_eq!(
                import.database.database_url.as_deref(),
                Some("postgres://example")
            );
            assert_eq!(import.file, std::path::Path::new("/tmp/site.toml"));
        }
        _ => panic!("wrong command parsed"),
    }
}

#[test]
fn parse_migrations_reconcile_arguments() {
    let args = CliArgs::parse_from([
        "soffio",
        "migrations",
        "reconcile",
        "--database-url",
        "postgres://example",
        "/tmp/archive.toml",
    ]);

    match args.command.expect("migrations command") {
        Command::Migrations(mig) => match mig.command {
            MigrationsCommand::Reconcile(rec) => {
                assert_eq!(
                    rec.database.database_url.as_deref(),
                    Some("postgres://example")
                );
                assert_eq!(rec.file, std::path::Path::new("/tmp/archive.toml"));
            }
        },
        _ => panic!("wrong command parsed"),
    }
}

#[test]
fn parse_serve_overrides() {
    let args = CliArgs::parse_from([
        "soffio",
        "serve",
        "--server-host",
        "0.0.0.0",
        "--database-url",
        "postgres://override",
    ]);

    match args.command.expect("serve command") {
        Command::Serve(serve) => {
            assert_eq!(serve.overrides.server_host.as_deref(), Some("0.0.0.0"));
            assert_eq!(
                serve.overrides.database_url.as_deref(),
                Some("postgres://override")
            );
        }
        _ => panic!("wrong command parsed"),
    }
}

#[test]
fn cache_settings_use_correct_defaults() {
    let raw = RawSettings::default();
    let settings = Settings::from_raw(raw).expect("valid settings");

    assert!(settings.cache.enable_l0_cache);
    assert!(settings.cache.enable_l1_cache);
    assert_eq!(settings.cache.l0_post_limit, 500);
    assert_eq!(settings.cache.l0_page_limit, 100);
    assert_eq!(settings.cache.l0_api_key_limit, 100);
    assert_eq!(settings.cache.l0_post_list_limit, 50);
    assert_eq!(settings.cache.l1_response_limit, 200);
    assert_eq!(settings.cache.l1_response_body_limit_bytes, 1_048_576);
    assert_eq!(settings.cache.auto_consume_interval_ms, 5000);
    assert_eq!(settings.cache.consume_batch_limit, 100);
    assert_eq!(settings.cache.max_event_queue_len, 2048);
}

#[test]
fn cache_settings_can_be_overridden_via_cli() {
    let mut raw = RawSettings::default();
    let overrides = ServeOverrides {
        cache_enable_l0_cache: Some(false),
        cache_enable_l1_cache: Some(false),
        cache_l0_post_limit: Some(1000),
        cache_l1_response_limit: Some(500),
        cache_l1_response_body_limit_bytes: Some(2_000_000),
        cache_max_event_queue_len: Some(4096),
        ..Default::default()
    };

    raw.apply_serve_overrides(&overrides);
    let settings = Settings::from_raw(raw).expect("valid settings");

    assert!(!settings.cache.enable_l0_cache);
    assert!(!settings.cache.enable_l1_cache);
    assert_eq!(settings.cache.l0_post_limit, 1000);
    assert_eq!(settings.cache.l1_response_limit, 500);
    assert_eq!(settings.cache.l1_response_body_limit_bytes, 2_000_000);
    assert_eq!(settings.cache.max_event_queue_len, 4096);
    // Other fields should still use defaults
    assert_eq!(settings.cache.l0_page_limit, 100);
}

#[test]
fn parse_cache_cli_arguments() {
    let args = CliArgs::parse_from([
        "soffio",
        "serve",
        "--cache-enable-l0-cache=false",
        "--cache-l0-post-limit",
        "1000",
        "--cache-l1-response-body-limit-bytes",
        "2048",
        "--cache-max-event-queue-len",
        "4096",
    ]);

    match args.command.expect("serve command") {
        Command::Serve(serve) => {
            assert_eq!(serve.overrides.cache_enable_l0_cache, Some(false));
            assert_eq!(serve.overrides.cache_l0_post_limit, Some(1000));
            assert_eq!(
                serve.overrides.cache_l1_response_body_limit_bytes,
                Some(2048)
            );
            assert_eq!(serve.overrides.cache_max_event_queue_len, Some(4096));
        }
        _ => panic!("wrong command parsed"),
    }
}
