use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// Compute a deterministic fingerprint for the selected input paths plus a caller-provided salt.
///
/// The fingerprint includes file paths and file contents. Directory traversal order is normalized.
pub fn fingerprint_inputs(inputs: &[&Path], salt: &str) -> Result<u64, String> {
    let mut hasher = DefaultHasher::new();
    salt.hash(&mut hasher);

    for input in inputs {
        hash_input(input, &mut hasher)?;
    }

    Ok(hasher.finish())
}

/// Return true when the output directory exists and the stored stamp matches `fingerprint`.
pub fn stamp_matches(
    stamp_path: &Path,
    output_dir: &Path,
    fingerprint: u64,
) -> Result<bool, String> {
    if !output_dir.exists() {
        return Ok(false);
    }

    let stamp = match fs::read_to_string(stamp_path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(format!(
                "failed to read stamp {}: {err}",
                stamp_path.display()
            ));
        }
    };

    Ok(stamp.trim() == format!("{fingerprint:016x}"))
}

/// Persist the fingerprint stamp.
pub fn write_stamp(stamp_path: &Path, fingerprint: u64) -> Result<(), String> {
    fs::write(stamp_path, format!("{fingerprint:016x}\n"))
        .map_err(|err| format!("failed to write stamp {}: {err}", stamp_path.display()))
}

fn hash_input(path: &Path, hasher: &mut DefaultHasher) -> Result<(), String> {
    if path.is_file() {
        return hash_file(path, hasher);
    }

    if path.is_dir() {
        let mut files = Vec::new();
        collect_files(path, &mut files)?;
        files.sort();

        for file in files {
            hash_file(&file, hasher)?;
        }

        return Ok(());
    }

    path.to_string_lossy().hash(hasher);
    "__missing__".hash(hasher);
    Ok(())
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|err| format!("failed to read directory {}: {err}", dir.display()))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|err| format!("failed to iterate {}: {err}", dir.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    entries.sort();

    for entry in entries {
        if entry.is_dir() {
            collect_files(&entry, files)?;
        } else if entry.is_file() {
            files.push(entry);
        }
    }

    Ok(())
}

fn hash_file(path: &Path, hasher: &mut DefaultHasher) -> Result<(), String> {
    path.to_string_lossy().hash(hasher);
    fs::read(path)
        .map_err(|err| format!("failed to read input {}: {err}", path.display()))?
        .hash(hasher);
    Ok(())
}
