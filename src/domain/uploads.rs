//! Upload-specific helpers and invariants.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{OffsetDateTime, format_description::FormatItem, macros::format_description};

const MONTH_KEY_FORMAT: &[FormatItem<'static>] = format_description!("[year]-[month padding:zero]");
const MONTH_LABEL_FORMAT: &[FormatItem<'static>] = format_description!("[month repr:long] [year]");

const INLINE_PREVIEW_PREFIXES: &[&str] = &["image/", "video/", "audio/", "text/"];
const INLINE_PREVIEW_EXACT: &[&str] = &["application/pdf"];

/// Canonical metadata key for asset width in CSS pixels.
pub const METADATA_WIDTH: &str = "width";

/// Canonical metadata key for asset height in CSS pixels.
pub const METADATA_HEIGHT: &str = "height";

/// Render a YYYY-MM key for grouping uploads by month.
pub fn month_key_for(timestamp: OffsetDateTime) -> String {
    timestamp
        .date()
        .format(MONTH_KEY_FORMAT)
        .expect("valid month key")
}

/// Render a human readable month label, e.g. "October 2025".
pub fn month_label_for(timestamp: OffsetDateTime) -> String {
    timestamp
        .date()
        .format(MONTH_LABEL_FORMAT)
        .expect("valid month label")
}

/// Determine whether the provided MIME type should be rendered with an inline preview link.
pub fn supports_inline_preview(content_type: &str) -> bool {
    INLINE_PREVIEW_PREFIXES
        .iter()
        .any(|prefix| content_type.starts_with(prefix))
        || INLINE_PREVIEW_EXACT.contains(&content_type)
}

/// Structured metadata stored alongside an uploaded asset.
///
/// Internally represented as a flat map so the same keys can be reused verbatim
/// in query strings or other serialized forms. Keys are restricted to
/// lowercase ASCII plus digits and underscores to keep them portable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UploadMetadata {
    #[serde(default)]
    entries: BTreeMap<String, MetadataValue>,
}

impl UploadMetadata {
    /// Construct an empty metadata map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return whether any metadata keys are populated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert or replace an integer metadata value.
    pub fn set_integer(
        &mut self,
        key: &str,
        value: NonZeroU32,
    ) -> Result<(), MetadataValidationError> {
        validate_key(key)?;
        self.entries
            .insert(key.to_string(), MetadataValue::Integer(value.get()));
        Ok(())
    }

    /// Read an integer metadata value.
    #[must_use]
    pub fn integer(&self, key: &str) -> Option<u32> {
        self.entries.get(key).and_then(MetadataValue::as_integer)
    }

    /// Iterate over key/value pairs as owned query parameter strings.
    pub fn query_pairs(&self) -> impl Iterator<Item = (String, String)> + '_ {
        self.entries
            .iter()
            .filter_map(|(key, value)| value.as_query_value().map(|val| (key.clone(), val)))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &MetadataValue)> {
        self.entries
            .iter()
            .map(|(key, value)| (key.as_str(), value))
    }

    pub fn set_text(&mut self, key: &str, value: &str) -> Result<(), MetadataValidationError> {
        validate_key(key)?;
        self.entries
            .insert(key.to_string(), MetadataValue::Text(value.to_string()));
        Ok(())
    }

    pub fn extend_from(&mut self, other: &UploadMetadata) -> Result<(), MetadataValidationError> {
        for (key, value) in other.iter() {
            match value {
                MetadataValue::Integer(raw) => {
                    let non_zero =
                        NonZeroU32::new(*raw).ok_or(MetadataValidationError::InvalidInteger)?;
                    self.set_integer(key, non_zero)?;
                }
                MetadataValue::Text(text) => {
                    self.set_text(key, text)?;
                }
            }
        }
        Ok(())
    }
}

/// A single metadata value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetadataValue {
    /// Positive integers (e.g., dimensions in CSS pixels).
    Integer(u32),
    /// UTF-8 strings (reserved for future extractors).
    Text(String),
}

impl MetadataValue {
    fn as_integer(&self) -> Option<u32> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::Text(_) => None,
        }
    }

    fn as_query_value(&self) -> Option<String> {
        match self {
            Self::Integer(value) => Some(value.to_string()),
            Self::Text(value) => Some(value.clone()),
        }
    }
}

/// Validation errors encountered while mutating metadata.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MetadataValidationError {
    #[error("metadata key must be lowercase ASCII letters, numbers, or underscore")]
    InvalidKey,
    #[error("metadata integer values must be greater than zero")]
    InvalidInteger,
}

fn validate_key(key: &str) -> Result<(), MetadataValidationError> {
    if key.is_empty()
        || key
            .bytes()
            .any(|byte| !matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_'))
    {
        return Err(MetadataValidationError::InvalidKey);
    }
    Ok(())
}
