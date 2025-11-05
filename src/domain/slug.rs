//! Utilities for generating deterministic, human-friendly slugs.
//!
//! The helpers here bridge ASCII slugification (`slug` crate) with Chinese
//! transliteration (`pinyin` crate) so inputs like “基线对齐” become
//! `ji-xian-dui-qi`. Consumers can provide their own uniqueness predicate to
//! avoid persistence conflicts while keeping the slug generation logic pure.

use std::collections::HashMap;
use std::future::Future;

use pinyin::{Pinyin, ToPinyin};
use slug::slugify;
use thiserror::Error;

const MAX_SUFFIX_ATTEMPTS: usize = 32;

/// Errors that can occur while generating a slug.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SlugError {
    #[error("slug source text is empty")]
    EmptyInput,
    #[error("failed to derive slug from `{input}`")]
    Unrepresentable { input: String },
    #[error("exhausted attempts to find a unique slug for `{base}`")]
    Exhausted { base: String },
}

/// Errors that can occur while generating a slug via an async uniqueness check.
#[derive(Debug, Error)]
pub enum SlugAsyncError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    #[error(transparent)]
    Slug(#[from] SlugError),
    #[error(transparent)]
    Predicate(E),
}

/// Derive a base slug from the provided human-readable text.
pub fn derive_slug(input: &str) -> Result<String, SlugError> {
    if input.trim().is_empty() {
        return Err(SlugError::EmptyInput);
    }

    let transliterated = transliterate_to_ascii(input);
    let candidate = slugify(&transliterated);

    if candidate.is_empty() {
        return Err(SlugError::Unrepresentable {
            input: input.to_string(),
        });
    }

    Ok(candidate)
}

/// Produce a slug that does not collide according to the supplied predicate.
///
/// The `is_unique` closure must return `true` when the provided slug does not
/// already exist (for example, after checking a repository or database). The
/// helper will retry by suffixing a monotonic counter (`-2`, `-3`, …).
pub fn generate_unique_slug<F>(input: &str, mut is_unique: F) -> Result<String, SlugError>
where
    F: FnMut(&str) -> bool,
{
    let base = derive_slug(input)?;

    if is_unique(&base) {
        return Ok(base);
    }

    for attempt in 2..=MAX_SUFFIX_ATTEMPTS + 1 {
        let candidate = format!("{base}-{attempt}");
        if is_unique(&candidate) {
            return Ok(candidate);
        }
    }

    Err(SlugError::Exhausted { base })
}

/// Async variant of [`generate_unique_slug`] that awaits the uniqueness predicate.
pub async fn generate_unique_slug_async<F, Fut, E>(
    input: &str,
    mut is_unique: F,
) -> Result<String, SlugAsyncError<E>>
where
    F: FnMut(&str) -> Fut,
    Fut: Future<Output = Result<bool, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    let base = derive_slug(input)?;

    if is_unique(&base).await.map_err(SlugAsyncError::Predicate)? {
        return Ok(base);
    }

    for attempt in 2..=MAX_SUFFIX_ATTEMPTS + 1 {
        let candidate = format!("{base}-{attempt}");
        if is_unique(&candidate)
            .await
            .map_err(SlugAsyncError::Predicate)?
        {
            return Ok(candidate);
        }
    }

    Err(SlugAsyncError::Slug(SlugError::Exhausted { base }))
}

/// Deterministically generate unique anchor slugs within a single document.
///
/// Headings processed in order will receive monotonic suffixes when duplicates
/// occur (e.g. `section`, `section-2`, `section-3`).
#[derive(Default, Debug)]
pub struct AnchorSlugger {
    occurrences: HashMap<String, usize>,
}

impl AnchorSlugger {
    /// Create a new slugger instance.
    pub fn new() -> Self {
        Self {
            occurrences: HashMap::new(),
        }
    }

    /// Generate a slug for the provided heading text, ensuring uniqueness
    /// within this slugger. Returns an error when the heading cannot produce a
    /// slug (empty or unrepresentable input).
    pub fn anchor_for(&mut self, heading: &str) -> Result<String, SlugError> {
        let base = derive_slug(heading)?;
        let count = self.occurrences.entry(base.clone()).or_insert(0);
        *count += 1;

        if *count == 1 {
            Ok(base)
        } else {
            Ok(format!("{base}-{}", *count))
        }
    }
}

fn transliterate_to_ascii(input: &str) -> String {
    let mut output = String::with_capacity(input.len());

    for ch in input.chars() {
        if ch.is_ascii() {
            output.push(ch);
            continue;
        }

        match ch.to_pinyin() {
            Some(py) => append_pinyin(&mut output, py),
            None if ch.is_whitespace() => output.push(' '),
            None => {
                // Preserve unhandled characters so slugify can decide how to filter them.
                output.push(ch);
            }
        }
    }

    output
}

fn append_pinyin(buffer: &mut String, pinyin: Pinyin) {
    if !buffer.is_empty() && !buffer.ends_with(' ') {
        buffer.push(' ');
    }
    buffer.push_str(pinyin.plain());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_slug_transliterates_chinese() {
        let slug = derive_slug("Rust 基础教程").expect("slug");
        assert_eq!(slug, "rust-ji-chu-jiao-cheng");
    }

    #[test]
    fn generate_unique_slug_appends_counter() {
        let mut existing = vec!["pattern-library".to_string()];
        let slug = generate_unique_slug("Pattern Library", |candidate| {
            if existing.contains(&candidate.to_string()) {
                false
            } else {
                existing.push(candidate.to_string());
                true
            }
        })
        .expect("unique slug");

        assert_eq!(slug, "pattern-library-2");
        assert!(existing.contains(&slug));
    }

    #[test]
    fn generate_unique_slug_exhausted() {
        let result =
            generate_unique_slug("Example", |_| false).expect_err("should exhaust attempts");
        assert_eq!(
            result,
            SlugError::Exhausted {
                base: "example".to_string()
            }
        );
    }

    #[test]
    fn anchor_slugger_produces_unique_slugs() {
        let mut slugger = AnchorSlugger::new();

        let first = slugger.anchor_for("Overview").expect("slug");
        let second = slugger.anchor_for("Overview").expect("slug");
        let third = slugger.anchor_for("深入理解").expect("slug");

        assert_eq!(first, "overview");
        assert_eq!(second, "overview-2");
        assert_eq!(third, "shen-ru-li-jie");
    }

    #[tokio::test]
    async fn generate_unique_slug_async_works() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let existing = Arc::new(Mutex::new(vec!["pattern-library".to_string()]));

        let slug = generate_unique_slug_async("Pattern Library", |candidate| {
            let existing = existing.clone();
            let candidate = candidate.to_string();
            async move {
                let mut guard = existing.lock().await;
                if guard.contains(&candidate) {
                    Ok::<bool, std::convert::Infallible>(false)
                } else {
                    guard.push(candidate);
                    Ok::<bool, std::convert::Infallible>(true)
                }
            }
        })
        .await
        .expect("unique slug");

        assert_eq!(slug, "pattern-library-2");
        let guard = existing.lock().await;
        assert!(guard.contains(&slug));
    }
}
