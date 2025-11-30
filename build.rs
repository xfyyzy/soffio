use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use syntect::dumps::dump_to_uncompressed_file;
use syntect::highlighting::ThemeSet;
use syntect::html::{ClassStyle, css_for_theme_with_class_style};
use two_face::syntax;
use walkdir::WalkDir;

fn main() {
    prepare_public_assets().expect("failed to prepare static public assets");
    prepare_common_assets().expect("failed to prepare shared static assets");
    prepare_admin_assets().expect("failed to prepare admin static assets");

    let static_dir = Path::new("static");

    println!("cargo:rerun-if-changed={}", static_dir.display());
    println!("cargo:rerun-if-changed=frontend/ts/datastar-init.ts");
    println!("cargo:rerun-if-changed=frontend/ts/datastar.d.ts");
    println!("cargo:rerun-if-changed=tsconfig.json");

    if static_dir.is_dir() {
        for entry in WalkDir::new(static_dir).into_iter().flatten() {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }
}

fn prepare_admin_assets() -> Result<(), String> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").map_err(|err| err.to_string())?);
    let source_admin = Path::new("static").join("admin");
    let dest_admin = out_dir.join("static_admin");

    if dest_admin.exists() {
        fs::remove_dir_all(&dest_admin)
            .map_err(|err| format!("failed to clean {}: {err}", dest_admin.display()))?;
    }

    copy_dir(&source_admin, &dest_admin)?;

    // Remove split sources from the bundled output; we only need the concatenated app.css at runtime.
    let split_dir = dest_admin.join("app");
    if split_dir.exists() {
        fs::remove_dir_all(&split_dir)
            .map_err(|err| format!("failed to clean split dir {}: {err}", split_dir.display()))?;
    }

    // Concatenate admin CSS parts into a single app.css in the bundled output.
    let parts_dir = source_admin.join("app");
    let mut parts: Vec<_> = fs::read_dir(&parts_dir)
        .map_err(|err| format!("failed to read {}: {err}", parts_dir.display()))?
        .filter_map(|entry| match entry {
            Ok(e) if e.file_type().map(|ft| ft.is_file()).unwrap_or(false) => Some(e),
            _ => None,
        })
        .collect();

    parts.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    if parts.is_empty() {
        return Err(format!(
            "no admin css parts found in {}",
            parts_dir.display()
        ));
    }

    let mut combined = String::new();
    for entry in parts {
        let content = fs::read_to_string(entry.path())
            .map_err(|err| format!("failed to read {}: {err}", entry.path().display()))?;
        combined.push_str(&content);
    }

    let dest_file = dest_admin.join("app.css");
    fs::write(&dest_file, combined)
        .map_err(|err| format!("failed to write {}: {err}", dest_file.display()))?;

    Ok(())
}

fn prepare_public_assets() -> Result<(), String> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").map_err(|err| err.to_string())?);
    let source_public = Path::new("static").join("public");
    let dest_public = out_dir.join("static_public");

    if dest_public.exists() {
        fs::remove_dir_all(&dest_public)
            .map_err(|err| format!("failed to clean {}: {err}", dest_public.display()))?;
    }

    copy_dir(&source_public, &dest_public)?;
    append_theme_css(&dest_public.join("styles/code.css"))?;
    write_syntax_pack(&out_dir)
}

fn prepare_common_assets() -> Result<(), String> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").map_err(|err| err.to_string())?);
    let source_common = Path::new("static").join("common");
    let dest_common = out_dir.join("static_common");

    if dest_common.exists() {
        fs::remove_dir_all(&dest_common)
            .map_err(|err| format!("failed to clean {}: {err}", dest_common.display()))?;
    }

    copy_dir(&source_common, &dest_common)?;
    compile_typescript(&dest_common)?;

    Ok(())
}

fn compile_typescript(out_dir: &Path) -> Result<(), String> {
    let output = Command::new("tsc")
        .arg("--project")
        .arg("tsconfig.json")
        .arg("--outDir")
        .arg(out_dir)
        .output()
        .map_err(|err| format!("failed to execute tsc: {err}"))?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "tsc failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            stdout,
            stderr
        ));
    }

    Ok(())
}

fn copy_dir(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|err| format!("failed to create {}: {err}", destination.display()))?;

    for entry in WalkDir::new(source).into_iter().flatten() {
        let relative = entry
            .path()
            .strip_prefix(source)
            .map_err(|err| format!("failed to strip prefix: {err}"))?;
        let target_path = destination.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target_path)
                .map_err(|err| format!("failed to create {}: {err}", target_path.display()))?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
            }
            fs::copy(entry.path(), &target_path)
                .map_err(|err| format!("failed to copy {}: {err}", target_path.display()))?;
        }
    }

    Ok(())
}

fn append_theme_css(base_path: &Path) -> Result<(), String> {
    let base_css = fs::read_to_string(base_path)
        .map_err(|err| format!("failed to read {}: {err}", base_path.display()))?;
    let theme_css = render_theme_css()?;

    let mut combined = String::with_capacity(base_css.len() + theme_css.len() + 200);
    combined.push_str(base_css.trim_end());
    combined.push_str(
        "\n\n/* --- Syntect theme (base16-ocean.light), generated at build time --- */\n",
    );
    combined.push_str(&theme_css);
    combined.push('\n');

    fs::write(base_path, combined)
        .map_err(|err| format!("failed to write {}: {err}", base_path.display()))
}

fn render_theme_css() -> Result<String, String> {
    let theme_set = ThemeSet::load_defaults();
    let theme = theme_set
        .themes
        .get("base16-ocean.light")
        .ok_or_else(|| "theme `base16-ocean.light` not found".to_string())?;

    css_for_theme_with_class_style(theme, ClassStyle::SpacedPrefixed { prefix: "syntax-" })
        .map_err(|err| err.to_string())
}

fn write_syntax_pack(out_dir: &Path) -> Result<(), String> {
    let syntax_set = syntax::extra_newlines();
    let pack_path = out_dir.join("syntaxes.packdump");
    dump_to_uncompressed_file(&syntax_set, &pack_path)
        .map_err(|err| format!("failed to encode syntax set: {err}"))?;

    println!("cargo:rustc-env=SYNTAX_PACK_FILE={}", pack_path.display());

    Ok(())
}
