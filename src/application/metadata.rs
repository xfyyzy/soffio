//! Shared asset metadata extraction infrastructure.

use std::{borrow::Cow, num::NonZeroU32, path::Path};

use imagesize::{ImageError, ImageSize};
use once_cell::sync::Lazy;
use thiserror::Error;

use crate::domain::uploads::{
    METADATA_HEIGHT, METADATA_WIDTH, MetadataValidationError, UploadMetadata,
};

pub(crate) const MAX_DIMENSION: u32 = 10_000;

/// Global registry accessor.
pub fn metadata_registry() -> &'static AssetMetadataRegistry {
    &REGISTRY
}

static IMAGE_EXTRACTOR: ImageMetadataExtractor = ImageMetadataExtractor;
static EXTRACTORS: [&'static dyn AssetMetadataExtractor; 1] = [&IMAGE_EXTRACTOR];
static REGISTRY: Lazy<AssetMetadataRegistry> =
    Lazy::new(|| AssetMetadataRegistry::new(&EXTRACTORS));

/// Error raised while deriving metadata.
#[derive(Debug, Error)]
pub enum AssetMetadataError {
    #[error(transparent)]
    Validation(#[from] MetadataValidationError),
    #[error("unsupported asset format for extractor")]
    Unsupported,
    #[error("failed to read asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to inspect image: {0}")]
    Probe(String),
}

/// Contract implemented by metadata extractors.
pub trait AssetMetadataExtractor: Sync {
    /// Unique identifier for diagnostics.
    fn key(&self) -> &'static str;

    /// Whether this extractor should run for the provided MIME type.
    fn matches_content_type(&self, content_type: &str) -> bool;

    /// Whether this extractor should run for the provided HTML element.
    fn matches_element(&self, element: &str) -> bool;

    /// Derive metadata from a stored file on disk.
    fn extract_from_file(
        &self,
        path: &Path,
        content_type: &str,
    ) -> Result<Option<UploadMetadata>, AssetMetadataError>;

    /// Derive metadata from query string parameters.
    fn extract_from_query<'a>(
        &self,
        params: &[(Cow<'a, str>, Cow<'a, str>)],
    ) -> Result<Option<UploadMetadata>, AssetMetadataError>;
}

/// Service orchestrating all registered extractors.
pub struct AssetMetadataRegistry {
    extractors: &'static [&'static dyn AssetMetadataExtractor],
}

impl AssetMetadataRegistry {
    const fn new(extractors: &'static [&'static dyn AssetMetadataExtractor]) -> Self {
        Self { extractors }
    }

    /// Run all extractors matching the MIME type and merge their output.
    pub fn extract_from_file(
        &self,
        content_type: &str,
        path: &Path,
    ) -> Result<UploadMetadata, AssetMetadataError> {
        let mut combined = UploadMetadata::new();
        for extractor in self.extractors {
            if !extractor.matches_content_type(content_type) {
                continue;
            }

            if let Some(metadata) = extractor.extract_from_file(path, content_type)? {
                combined.extend_from(&metadata)?;
            }
        }
        Ok(combined)
    }

    /// Run extractors that match the HTML element and return the first successful result.
    pub fn extract_from_query<'a>(
        &self,
        element: &str,
        params: &[(Cow<'a, str>, Cow<'a, str>)],
    ) -> Result<Option<UploadMetadata>, AssetMetadataError> {
        for extractor in self.extractors {
            if !extractor.matches_element(element) {
                continue;
            }
            if let Some(metadata) = extractor.extract_from_query(params)? {
                return Ok(Some(metadata));
            }
        }
        Ok(None)
    }
}

struct ImageMetadataExtractor;

impl ImageMetadataExtractor {
    fn parse_dimension(value: &str) -> Option<NonZeroU32> {
        if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }
        let parsed: u32 = value.parse().ok()?;
        if parsed == 0 || parsed > MAX_DIMENSION {
            return None;
        }
        NonZeroU32::new(parsed)
    }

    fn dimensions_to_metadata(
        &self,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> Result<UploadMetadata, AssetMetadataError> {
        let mut metadata = UploadMetadata::new();
        metadata.set_integer(METADATA_WIDTH, width)?;
        metadata.set_integer(METADATA_HEIGHT, height)?;
        Ok(metadata)
    }

    fn image_size(path: &Path) -> Result<ImageSize, AssetMetadataError> {
        match imagesize::size(path) {
            Ok(size) => Ok(size),
            Err(ImageError::NotSupported) => Err(AssetMetadataError::Unsupported),
            Err(ImageError::CorruptedImage) => {
                Err(AssetMetadataError::Probe("corrupted image".to_string()))
            }
            Err(ImageError::IoError(err)) => Err(AssetMetadataError::Io(err)),
        }
    }
}

impl AssetMetadataExtractor for ImageMetadataExtractor {
    fn key(&self) -> &'static str {
        "image"
    }

    fn matches_content_type(&self, content_type: &str) -> bool {
        content_type.starts_with("image/")
    }

    fn matches_element(&self, element: &str) -> bool {
        element.eq_ignore_ascii_case("img")
    }

    fn extract_from_file(
        &self,
        path: &Path,
        _content_type: &str,
    ) -> Result<Option<UploadMetadata>, AssetMetadataError> {
        let size = match Self::image_size(path) {
            Ok(size) => size,
            Err(AssetMetadataError::Unsupported) => return Ok(None),
            Err(err) => return Err(err),
        };

        let width = u32::try_from(size.width)
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or(AssetMetadataError::Unsupported)?;
        let height = u32::try_from(size.height)
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or(AssetMetadataError::Unsupported)?;

        self.dimensions_to_metadata(width, height).map(Some)
    }

    fn extract_from_query<'a>(
        &self,
        params: &[(Cow<'a, str>, Cow<'a, str>)],
    ) -> Result<Option<UploadMetadata>, AssetMetadataError> {
        let mut width: Option<NonZeroU32> = None;
        let mut height: Option<NonZeroU32> = None;

        for (key, value) in params {
            match key.as_ref() {
                METADATA_WIDTH => {
                    width = Self::parse_dimension(value.as_ref());
                }
                METADATA_HEIGHT => {
                    height = Self::parse_dimension(value.as_ref());
                }
                _ => continue,
            }
        }

        match (width, height) {
            (Some(w), Some(h)) => self.dimensions_to_metadata(w, h).map(Some),
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{METADATA_HEIGHT, METADATA_WIDTH, metadata_registry};

    #[test]
    fn parses_image_query_dimensions() {
        let params = vec![
            (Cow::Borrowed(METADATA_WIDTH), Cow::Borrowed("640")),
            (Cow::Borrowed(METADATA_HEIGHT), Cow::Borrowed("360")),
        ];

        let metadata = metadata_registry()
            .extract_from_query("img", &params)
            .expect("extract metadata")
            .expect("metadata present");

        assert_eq!(metadata.integer(METADATA_WIDTH), Some(640));
        assert_eq!(metadata.integer(METADATA_HEIGHT), Some(360));
    }

    #[test]
    fn rejects_invalid_image_query_dimensions() {
        let params = vec![
            (Cow::Borrowed(METADATA_WIDTH), Cow::Borrowed("0")),
            (Cow::Borrowed(METADATA_HEIGHT), Cow::Borrowed("-5")),
        ];

        let metadata = metadata_registry()
            .extract_from_query("img", &params)
            .expect("extract metadata");

        assert!(metadata.is_none());
    }
}
