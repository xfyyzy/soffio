use std::{
    fs,
    io::{self, ErrorKind, Write},
    path::PathBuf,
    process::{Command, Stdio},
    time::Instant,
};

use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub(crate) enum MermaidRenderError {
    #[error("failed to prepare cache directory: {0}")]
    CacheInit(io::Error),
    #[error("failed to write temporary file: {0}")]
    Io(io::Error),
    #[error("mermaid CLI invocation failed (exit {exit_code:?}): {stderr}")]
    Cli {
        exit_code: Option<i32>,
        stderr: String,
    },
    #[error("mermaid CLI unavailable: {0}")]
    NotFound(io::Error),
    #[error("failed to read rendered SVG: {0}")]
    Read(io::Error),
}

#[derive(Debug, Clone)]
pub(crate) struct MermaidRenderer {
    cli_path: PathBuf,
    cache_dir: PathBuf,
}

impl MermaidRenderer {
    pub(crate) fn new(cli_path: PathBuf, cache_dir: PathBuf) -> Result<Self, MermaidRenderError> {
        fs::create_dir_all(&cache_dir).map_err(MermaidRenderError::CacheInit)?;
        Ok(Self {
            cli_path,
            cache_dir,
        })
    }

    pub(crate) fn render_svg(&self, source: &str) -> Result<String, MermaidRenderError> {
        let started_at = Instant::now();
        let cache_key = hash_source(source);
        let cache_path = self.cache_dir.join(format!("{cache_key}.svg"));
        match fs::read_to_string(&cache_path) {
            Ok(svg) => {
                info!(
                    target = "application::render::mermaid",
                    op = "mermaid::render_svg",
                    result = "cache_hit",
                    elapsed_ms = started_at.elapsed().as_millis() as u64,
                    cache_path = %cache_path.display(),
                    svg_bytes = svg.len(),
                    "Mermaid diagram served from cache"
                );
                return Ok(svg);
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                warn!(
                    target = "application::render::mermaid",
                    op = "mermaid::render_svg",
                    result = "cache_read_error",
                    elapsed_ms = started_at.elapsed().as_millis() as u64,
                    cache_path = %cache_path.display(),
                    error = %err,
                    "Failed to read cached Mermaid diagram; re-rendering"
                );
            }
        }

        let mut input_file = NamedTempFile::new().map_err(MermaidRenderError::Io)?;
        input_file
            .write_all(source.as_bytes())
            .map_err(MermaidRenderError::Io)?;
        input_file.flush().map_err(MermaidRenderError::Io)?;

        let output_file = tempfile::Builder::new()
            .suffix(".svg")
            .tempfile_in(&self.cache_dir)
            .map_err(MermaidRenderError::Io)?;
        let output_path = output_file.path().to_path_buf();

        let cli_started_at = Instant::now();
        let output = Command::new(&self.cli_path)
            .arg("--input")
            .arg(input_file.path())
            .arg("--output")
            .arg(&output_path)
            .arg("--outputFormat")
            .arg("svg")
            .arg("--quiet")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|err| {
                warn!(
                    target = "application::render::mermaid",
                    op = "mermaid::render_svg",
                    result = "error",
                    elapsed_ms = started_at.elapsed().as_millis() as u64,
                    cli_elapsed_ms = cli_started_at.elapsed().as_millis() as u64,
                    error_code = "spawn_cli",
                    error = %err,
                    "Failed to spawn Mermaid CLI"
                );
                if err.kind() == ErrorKind::NotFound {
                    MermaidRenderError::NotFound(err)
                } else {
                    MermaidRenderError::Io(err)
                }
            })?;

        if !output.status.success() {
            let exit_code = output.status.code();
            let exit_code_value = exit_code.map(i64::from).unwrap_or(-1);
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            warn!(
                target = "application::render::mermaid",
                op = "mermaid::render_svg",
                result = "error",
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                cli_elapsed_ms = cli_started_at.elapsed().as_millis() as u64,
                exit_code = exit_code_value,
                error_code = "mermaid_cli",
                stderr = %stderr,
                "Mermaid CLI invocation failed"
            );
            return Err(MermaidRenderError::Cli { exit_code, stderr });
        }

        match output_file.persist(&cache_path) {
            Ok(_) => {}
            Err(err) if err.error.kind() == ErrorKind::AlreadyExists => {
                // Another renderer persisted the same diagram concurrently; fall through.
            }
            Err(err) => return Err(MermaidRenderError::Io(err.error)),
        }

        let svg = fs::read_to_string(&cache_path).map_err(|err| {
            warn!(
                target = "application::render::mermaid",
                op = "mermaid::render_svg",
                result = "error",
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                cli_elapsed_ms = cli_started_at.elapsed().as_millis() as u64,
                error_code = "cache_read",
                error = %err,
                "Failed to read rendered Mermaid SVG from cache"
            );
            MermaidRenderError::Read(err)
        })?;

        info!(
            target = "application::render::mermaid",
            op = "mermaid::render_svg",
            result = "cache_miss",
            elapsed_ms = started_at.elapsed().as_millis() as u64,
            cli_elapsed_ms = cli_started_at.elapsed().as_millis() as u64,
            cache_path = %cache_path.display(),
            svg_bytes = svg.len(),
            "Mermaid diagram rendered via CLI"
        );

        Ok(svg)
    }
}

fn hash_source(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};
    use tempfile::TempDir;

    fn make_executable(path: &PathBuf) {
        let mut perms = fs::metadata(path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("set perms");
    }

    #[test]
    fn renders_svg_with_valid_cli() {
        let dir = TempDir::new().expect("temp dir");
        let script_path = dir.path().join("fake-mmdc");
        let args_path = dir.path().join("args.log");
        let script = format!(
            r#"#!/bin/sh
set -eu
echo "$@" > "{args_file}"
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output)
      shift
      out="$1"
      ;;
    --outputFormat)
      shift
      fmt="$1"
      ;;
    --input)
      shift
      input="$1"
      ;;
    --quiet)
      shift
      ;;
    *)
      shift
      ;;
  esac
done
if [ -z "${{out:-}}" ]; then
  echo "missing --output" >&2
  exit 2
fi
case "$out" in
  *.svg) ;;
  *)
    echo "invalid output suffix: $out" >&2
    exit 9
    ;;
esac
cat <<'SVG' > "$out"
<svg>ok</svg>
SVG
"#,
            args_file = args_path.display()
        );
        fs::write(&script_path, script).expect("write script");
        make_executable(&script_path);

        let cache_dir = dir.path().join("cache");
        let renderer =
            MermaidRenderer::new(script_path.clone(), cache_dir.clone()).expect("renderer");

        let svg = renderer
            .render_svg("flowchart LR\n  A --> B")
            .expect("svg rendered");
        assert!(
            svg.contains("<svg>ok</svg>"),
            "unexpected svg output: {svg}"
        );

        let args = fs::read_to_string(&args_path).expect("read args");
        assert!(
            args.contains("--outputFormat"),
            "CLI args missing --outputFormat: {args}"
        );
        assert!(
            args.contains("--output"),
            "CLI args missing --output: {args}"
        );
    }

    #[test]
    fn surfaces_cli_errors() {
        let dir = TempDir::new().expect("temp dir");
        let script_path = dir.path().join("fake-mmdc");
        fs::write(
            &script_path,
            r#"#!/bin/sh
echo "boom" >&2
exit 42
"#,
        )
        .expect("write script");
        make_executable(&script_path);

        let cache_dir = dir.path().join("cache");
        let renderer =
            MermaidRenderer::new(script_path.clone(), cache_dir.clone()).expect("renderer");

        let err = renderer
            .render_svg("flowchart LR\n  A --> B")
            .expect_err("expected cli failure");
        match err {
            MermaidRenderError::Cli { exit_code, stderr } => {
                assert_eq!(exit_code, Some(42));
                assert!(
                    stderr.contains("boom"),
                    "stderr did not propagate: {stderr}"
                );
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }
}
