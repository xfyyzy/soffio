use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
};

use sqlx::postgres::PgPoolOptions;

use soffio::application::error::AppError;
use soffio::config::{DatabaseSettings, MigrationsReconcileArgs};

#[derive(Debug, Clone)]
struct MigrationEntry {
    version: i64,
    checksum: String,
}

pub async fn reconcile_archive(
    database: &DatabaseSettings,
    args: &MigrationsReconcileArgs,
) -> Result<(), AppError> {
    let database_url = database.url.as_deref().ok_or_else(|| {
        AppError::validation(
            "database url is required (provide --database-url or set SOFFIO__DATABASE__URL/DATABASE_URL)",
        )
    })?;

    let migrations = fetch_migrations(database_url).await?;
    rewrite_archive(&args.file, &migrations)
        .map_err(|e| AppError::unexpected(format!("failed to rewrite archive: {e}")))?;

    println!(
        "Updated {} migration entries to match database ({} versions)",
        args.file.display(),
        migrations.len()
    );

    Ok(())
}

async fn fetch_migrations(database_url: &str) -> Result<Vec<MigrationEntry>, AppError> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .map_err(|e| AppError::unexpected(e.to_string()))?;

    let rows = sqlx::query!(
        r#"SELECT version, encode(checksum, 'hex') AS "checksum!" FROM _sqlx_migrations ORDER BY version"#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::unexpected(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|row| MigrationEntry {
            version: row.version,
            checksum: row.checksum,
        })
        .collect())
}

fn rewrite_archive(
    archive_path: &std::path::Path,
    migrations: &[MigrationEntry],
) -> Result<(), Box<dyn std::error::Error>> {
    let input = File::open(archive_path)?;
    let reader = BufReader::new(input);

    let tmp_path = archive_path.with_extension("toml.tmp");
    let output = File::create(&tmp_path)?;
    let mut writer = BufWriter::new(output);

    let mut state = State::Before;

    for line in reader.lines() {
        let line = line?;
        match state {
            State::Before => {
                if line.trim_start().starts_with("[[migrations.entries]]") {
                    write_migrations(&mut writer, migrations)?;
                    state = State::Skipping;
                    continue;
                }
                writeln!(writer, "{}", line)?;
            }
            State::Skipping => {
                let trimmed = line.trim_start();
                if trimmed.starts_with('[') && !trimmed.starts_with("[[migrations.entries]]") {
                    writeln!(writer, "{}", line)?;
                    state = State::Rest;
                }
            }
            State::Rest => {
                writeln!(writer, "{}", line)?;
            }
        }
    }

    if !matches!(state, State::Skipping | State::Rest) {
        return Err("archive missing migrations.entries section".into());
    }

    writer.flush()?;
    std::fs::rename(&tmp_path, archive_path)?;
    Ok(())
}

fn write_migrations<W: Write>(
    writer: &mut W,
    migrations: &[MigrationEntry],
) -> Result<(), Box<dyn std::error::Error>> {
    for (idx, mig) in migrations.iter().enumerate() {
        writeln!(writer, "[[migrations.entries]]")?;
        writeln!(writer, "version = {}", mig.version)?;
        writeln!(writer, "checksum = \"{}\"", mig.checksum)?;
        if idx + 1 != migrations.len() {
            writeln!(writer)?;
        }
    }
    writeln!(writer)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Before,
    Skipping,
    Rest,
}
