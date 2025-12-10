use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
};

use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use tokio::runtime::Runtime;

#[derive(Debug, Parser)]
#[command(
    name = "update-migration-version",
    about = "Sync TOML migrations.entries with database migrations"
)]
struct Cli {
    /// Path to TOML file containing [[migrations.entries]]
    #[arg(long, value_name = "FILE")]
    file_path: PathBuf,

    /// Postgres URL; falls back to DATABASE_URL env var
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,
}

#[derive(Debug, Clone)]
struct MigrationEntry {
    version: i64,
    checksum: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let rt = Runtime::new()?;
    let migrations = rt.block_on(fetch_migrations(&cli.database_url))?;

    rewrite_seed(&cli.file_path, &migrations)?;
    println!(
        "Updated {} migration entries to match database ({} versions)",
        cli.file_path.display(),
        migrations.len()
    );
    Ok(())
}

async fn fetch_migrations(
    database_url: &str,
) -> Result<Vec<MigrationEntry>, Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await?;

    let rows = sqlx::query!(
        r#"SELECT version, encode(checksum, 'hex') AS "checksum!" FROM _sqlx_migrations ORDER BY version"#
    )
    .fetch_all(&pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| MigrationEntry {
            version: row.version,
            checksum: row.checksum,
        })
        .collect())
}

fn rewrite_seed(
    seed_path: &PathBuf,
    migrations: &[MigrationEntry],
) -> Result<(), Box<dyn std::error::Error>> {
    let input = File::open(seed_path)?;
    let reader = BufReader::new(input);

    let mut tmp_path = seed_path.clone();
    tmp_path.set_extension("toml.tmp");
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
                    continue; // skip original line
                }
                writeln!(writer, "{}", line)?;
            }
            State::Skipping => {
                // detect start of next top-level table
                let trimmed = line.trim_start();
                if trimmed.starts_with('[') && !trimmed.starts_with("[[migrations.entries]]") {
                    writeln!(writer, "{}", line)?;
                    state = State::Rest;
                }
                // else keep skipping
            }
            State::Rest => {
                writeln!(writer, "{}", line)?;
            }
        }
    }

    // Safety: ensure we actually saw migrations section; otherwise bail.
    if !matches!(state, State::Skipping | State::Rest) {
        return Err("seed file missing migrations.entries section".into());
    }

    writer.flush()?;
    std::fs::rename(&tmp_path, seed_path)?;
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
