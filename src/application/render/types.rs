use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Identifies what is being rendered so callers can persist results appropriately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderTarget {
    /// Render the full post body, producing section metadata.
    PostBody { slug: String },
    /// Render the author-provided summary for a post.
    PostSummary { slug: String },
    /// Render a standalone static page.
    PageBody { slug: String },
}

impl RenderTarget {
    /// Returns the slug associated with the target. Useful for post-processing like
    /// anchoring or cache invalidation.
    pub fn slug(&self) -> &str {
        match self {
            RenderTarget::PostBody { slug }
            | RenderTarget::PostSummary { slug }
            | RenderTarget::PageBody { slug } => slug.as_str(),
        }
    }
}

/// Rendering request passed into the pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderRequest {
    /// Unique identifier for downstream storage.
    pub target: RenderTarget,
    /// Source markdown captured from the CMS/editor.
    pub markdown: String,
    /// Optional front matter or contextual metadata encoded as JSON for future-proofing.
    pub context: Option<serde_json::Value>,
    /// Normalised public site URL used for same-origin checks during link classification.
    #[serde(default)]
    pub public_site_url: Option<String>,
}

impl RenderRequest {
    pub fn new(target: RenderTarget, markdown: impl Into<String>) -> Self {
        Self {
            target,
            markdown: markdown.into(),
            context: None,
            public_site_url: None,
        }
    }

    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_public_site_url(mut self, public_site_url: impl Into<String>) -> Self {
        let normalized = normalize_public_site_url(public_site_url.into().as_str());
        if !normalized.is_empty() {
            self.public_site_url = Some(normalized);
        }
        self
    }
}

fn normalize_public_site_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let without_trailing = trimmed.trim_end_matches('/');
    format!("{without_trailing}/")
}

/// Section produced when rendering full posts. Pages and summaries omit sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderedSection {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub anchor_slug: String,
    pub heading_html: String,
    pub heading_text: String,
    pub body_html: String,
    pub level: u8,
    pub contains_code: bool,
    pub contains_math: bool,
    pub contains_mermaid: bool,
    /// 1-indexed position among siblings sharing the same parent.
    pub position: u32,
}

/// Aggregated resource hint recommendations extracted during rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResourceHints {
    /// Domains that should be preconnected before page load.
    pub preconnect_domains: Vec<String>,
    /// Domains that benefit from DNS prefetch.
    pub dns_prefetch_domains: Vec<String>,
}

/// Content-level metrics surfaced alongside rendered HTML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ContentMetrics {
    pub reading_time_minutes: u32,
    pub internal_links_count: u32,
    pub external_links_count: u32,
    pub images_count: u32,
    pub images_missing_alt: u32,
    pub images_missing_dimensions: u32,
    pub code_blocks_count: u32,
    pub math_blocks_count: u32,
    pub mermaid_diagram_count: u32,
}

/// Deterministic rendering result returned to callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderOutput {
    /// Sanitised HTML ready for persistence.
    pub html: String,
    /// Optional structured sections for posts.
    pub sections: Option<Vec<RenderedSection>>,
    /// Indicates whether the rendered HTML contains any code blocks.
    pub contains_code: bool,
    /// Indicates whether the rendered HTML contains rendered math fragments.
    pub contains_math: bool,
    /// Indicates whether the rendered HTML contains Mermaid diagrams.
    pub contains_mermaid: bool,
    /// Resource hints callers can surface in surrounding templates.
    #[serde(default)]
    pub resource_hints: ResourceHints,
    /// Basic content metrics that support editorial and SEO tooling.
    #[serde(default)]
    pub content_metrics: ContentMetrics,
}

impl RenderOutput {
    pub fn for_post(
        html: String,
        sections: Vec<RenderedSection>,
        contains_code: bool,
        contains_math: bool,
        contains_mermaid: bool,
    ) -> Self {
        Self {
            html,
            sections: Some(sections),
            contains_code,
            contains_math,
            contains_mermaid,
            resource_hints: ResourceHints::default(),
            content_metrics: ContentMetrics::default(),
        }
    }

    pub fn without_sections(
        html: String,
        contains_code: bool,
        contains_math: bool,
        contains_mermaid: bool,
    ) -> Self {
        Self {
            html,
            sections: None,
            contains_code,
            contains_math,
            contains_mermaid,
            resource_hints: ResourceHints::default(),
            content_metrics: ContentMetrics::default(),
        }
    }
}

/// Structured errors surfaced by the rendering pipeline. These should map cleanly
/// to job failure reasons without leaking implementation details.
#[derive(Debug, Clone, Error)]
pub enum RenderError {
    #[error("markdown parsing failed: {message}")]
    Markdown { message: String },
    #[error("syntax highlighting failed: {language}: {message}")]
    Highlighting { language: String, message: String },
    #[error("sanitisation rejected content: {message}")]
    Sanitisation { message: String },
    #[error("unsupported render target: {reason}")]
    Unsupported { reason: String },
    #[error("document processing failed: {message}")]
    Document { message: String },
    #[error("anchor slug generation failed: {message}")]
    Anchoring { message: String },
}

/// Trait exposed by the rendering pipeline. Implementations must be pure and
/// deterministic: given the same input, they return identical outputs or errors.
pub trait RenderService: Send + Sync {
    fn render(&self, request: &RenderRequest) -> Result<RenderOutput, RenderError>;
}
