//! Runtime upload storage and retrieval helpers.

use std::error::Error as StdError;
use std::fmt::Write as FmtWrite;
use std::path::{Component, Path, PathBuf};

use bytes::Bytes;
use futures::{StreamExt, pin_mut, stream};
use sha2::{Digest, Sha256};
use slug::slugify;
use thiserror::Error;
use tokio::{fs, io::AsyncWriteExt};
use uuid::Uuid;

/// Errors that can occur while interacting with the upload storage backend.
#[derive(Debug, Error)]
pub enum UploadStorageError {
    #[error("invalid stored path")]
    InvalidPath,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("uploaded file exceeds configured body limit")]
    PayloadTooLarge {
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },
    #[error("uploaded file stream failed")]
    PayloadStream {
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },
    #[error("uploaded file is empty")]
    EmptyPayload,
    #[error("uploaded file size exceeds supported range")]
    SizeOverflow,
}

/// Result of storing an upload payload.
#[derive(Debug, Clone)]
pub struct StoredUpload {
    pub stored_path: String,
    pub checksum: String,
    pub size_bytes: i64,
}

/// Filesystem-backed upload storage.
#[derive(Debug)]
pub struct UploadStorage {
    root: PathBuf,
}

impl UploadStorage {
    /// Initialise storage rooted at the provided directory, creating it if necessary.
    pub fn new(root: PathBuf) -> Result<Self, std::io::Error> {
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Store the provided payload and return metadata describing the stored asset.
    ///
    /// The payload is streamed to disk to avoid buffering large files in memory.
    pub async fn store_stream<S>(
        &self,
        original_name: &str,
        stream: S,
    ) -> Result<StoredUpload, UploadStorageError>
    where
        S: futures::Stream<Item = Result<Bytes, UploadStorageError>>,
    {
        let stored_path = self.build_stored_path(original_name);
        let absolute = self.resolve(&stored_path)?;

        if let Some(parent) = absolute.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(&absolute).await?;
        let mut hasher = Sha256::new();
        let mut total_bytes: u64 = 0;
        let mut saw_payload = false;

        pin_mut!(stream);
        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(chunk) => chunk,
                Err(err) => {
                    drop(file);
                    let _ = fs::remove_file(&absolute).await;
                    return Err(err);
                }
            };

            if chunk.is_empty() {
                continue;
            }

            saw_payload = true;
            total_bytes = total_bytes
                .checked_add(chunk.len() as u64)
                .ok_or(UploadStorageError::SizeOverflow)?;
            file.write_all(&chunk).await?;
            hasher.update(&chunk);
        }

        file.flush().await?;

        if !saw_payload {
            drop(file);
            let _ = fs::remove_file(&absolute).await;
            return Err(UploadStorageError::EmptyPayload);
        }

        let digest = hasher.finalize();
        let checksum = hex_from_bytes(&digest);
        let size_bytes =
            i64::try_from(total_bytes).map_err(|_| UploadStorageError::SizeOverflow)?;

        Ok(StoredUpload {
            stored_path,
            checksum,
            size_bytes,
        })
    }

    /// Store a fully-buffered payload. Intended for tests and small assets.
    pub async fn store(
        &self,
        original_name: &str,
        data: Bytes,
    ) -> Result<StoredUpload, UploadStorageError> {
        let stream = stream::once(async move { Ok::<_, UploadStorageError>(data) });
        self.store_stream(original_name, stream).await
    }

    /// Attempt to read the stored payload into memory.
    pub async fn read(&self, stored_path: &str) -> Result<Bytes, UploadStorageError> {
        let absolute = self.resolve(stored_path)?;
        let data = fs::read(absolute).await?;
        Ok(Bytes::from(data))
    }

    /// Remove the stored payload. Missing files are treated as success.
    pub async fn delete(&self, stored_path: &str) -> Result<(), UploadStorageError> {
        let absolute = self.resolve(stored_path)?;
        match fs::remove_file(&absolute).await {
            Ok(_) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(UploadStorageError::Io(err)),
        }
    }

    /// Obtain the absolute filesystem path for a stored upload.
    pub fn absolute_path(&self, stored_path: &str) -> Result<PathBuf, UploadStorageError> {
        self.resolve(stored_path)
    }

    /// Resolve the absolute filesystem path for a stored upload.
    fn resolve(&self, stored_path: &str) -> Result<PathBuf, UploadStorageError> {
        let relative = Path::new(stored_path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
        {
            return Err(UploadStorageError::InvalidPath);
        }

        Ok(self.root.join(relative))
    }

    fn build_stored_path(&self, original_name: &str) -> String {
        let (year, month, day) = time::OffsetDateTime::now_utc().to_calendar_date();
        let directory = format!("{year}/{:02}/{:02}", month as u8, day);
        let identifier = Uuid::new_v4();
        let filename = sanitize_filename(original_name);
        format!("{directory}/{identifier}-{filename}")
    }
}

fn sanitize_filename(original: &str) -> String {
    let path = Path::new(original);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("upload");
    let mut base = slugify(stem);
    if base.is_empty() {
        base = "upload".to_string();
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.trim_matches('.').to_ascii_lowercase())
        .filter(|value| !value.is_empty());

    match extension {
        Some(ext) => format!("{base}.{ext}"),
        None => base,
    }
}

fn hex_from_bytes(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = FmtWrite::write_fmt(&mut output, format_args!("{byte:02x}"));
    }
    output
}
