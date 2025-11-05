mod config;
mod highlight;
mod math;
mod mermaid;
mod rewrite;
mod sections;

use std::{path::PathBuf, sync::Arc};

use comrak::{Arena, format_html, nodes::AstNode, parse_document};
use once_cell::sync::{Lazy, OnceCell};
use syntect::{dumps::from_uncompressed_data, html::ClassStyle, parsing::SyntaxSet};
use thiserror::Error;
use tracing::warn;

use crate::application::render::types::{
    RenderError, RenderOutput, RenderRequest, RenderService, RenderTarget,
};
use crate::config::{DEFAULT_MERMAID_CACHE_DIR, DEFAULT_MERMAID_CLI_PATH};

use self::mermaid::{MermaidRenderError, MermaidRenderer};
use config::{build_page_sanitizer, build_post_sanitizer, default_options};
use rewrite::rewrite_ast;
use sections::{ProcessedHtml, post_process};

/// Default Comrak-based rendering pipeline with Syntect highlighting and Ammonia sanitisation.
pub struct ComrakRenderService {
    options: comrak::Options<'static>,
    syntax_set: SyntaxSet,
    class_style: ClassStyle,
    post_sanitizer: ammonia::Builder<'static>,
    page_sanitizer: ammonia::Builder<'static>,
    mermaid: Option<MermaidRenderer>,
}

impl ComrakRenderService {
    /// Construct a new renderer with all markdown extensions enabled and
    /// syntax highlighting configured to emit `syntax-` prefixed CSS classes.
    fn new() -> Self {
        let options = default_options();
        let syntax_bytes = include_bytes!(env!("SYNTAX_PACK_FILE"));
        let syntax_set: SyntaxSet =
            from_uncompressed_data(syntax_bytes).expect("syntax pack must be valid");
        let class_style = ClassStyle::SpacedPrefixed { prefix: "syntax-" };
        let post_sanitizer = build_post_sanitizer();
        let page_sanitizer = build_page_sanitizer();
        let config = active_render_config();
        let mermaid = match MermaidRenderer::new(
            config.mermaid_cli_path.clone(),
            config.mermaid_cache_dir.clone(),
        ) {
            Ok(renderer) => Some(renderer),
            Err(err) => {
                log_mermaid_init_error(&err, &config);
                None
            }
        };

        Self {
            options,
            syntax_set,
            class_style,
            post_sanitizer,
            page_sanitizer,
            mermaid,
        }
    }
}

static RENDER_SERVICE: Lazy<Arc<ComrakRenderService>> =
    Lazy::new(|| Arc::new(ComrakRenderService::new()));

/// Access the shared render service instance, initialised on first use.
pub fn render_service() -> Arc<ComrakRenderService> {
    Arc::clone(&RENDER_SERVICE)
}

impl Default for ComrakRenderService {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderService for ComrakRenderService {
    fn render(&self, request: &RenderRequest) -> Result<RenderOutput, RenderError> {
        let arena = Arena::new();
        let root = parse_document(&arena, &request.markdown, &self.options);

        let rewrite_outcome = rewrite_stage(
            root,
            &self.syntax_set,
            &self.class_style,
            self.mermaid.as_ref(),
            request.target.slug(),
        )?;

        let rendered_html = render_html_stage(root, &self.options)?;

        let sanitized_html = sanitize_stage(
            rendered_html,
            &request.target,
            &self.post_sanitizer,
            &self.page_sanitizer,
        )?;

        let restored_html = restore_stage(sanitized_html, &rewrite_outcome);

        let processed =
            post_process_stage(&restored_html, &request.target, &rewrite_outcome.headings)?;
        let ProcessedHtml {
            html,
            sections,
            contains_code: processed_contains_code,
            contains_math: processed_contains_math,
            contains_mermaid: processed_contains_mermaid,
            resource_hints,
            content_metrics,
        } = processed;

        let contains_code = rewrite_outcome.contains_code || processed_contains_code;
        let contains_math = rewrite_outcome.contains_math || processed_contains_math;
        let contains_mermaid = rewrite_outcome.contains_mermaid || processed_contains_mermaid;

        let mut output = match (&request.target, sections) {
            (RenderTarget::PostBody { .. }, Some(sections)) => RenderOutput::for_post(
                html,
                sections,
                contains_code,
                contains_math,
                contains_mermaid,
            ),
            _ => {
                RenderOutput::without_sections(html, contains_code, contains_math, contains_mermaid)
            }
        };

        output.resource_hints = resource_hints;
        output.content_metrics = content_metrics;

        Ok(output)
    }
}

impl ComrakRenderService {
    /// Render markdown into HTML while skipping the sanitisation stage. This is
    /// intended for diagnostics when refining sanitizer rules.
    pub fn render_unsanitized(&self, request: &RenderRequest) -> Result<String, RenderError> {
        let arena = Arena::new();
        let root = parse_document(&arena, &request.markdown, &self.options);

        let rewrite_outcome = rewrite_stage(
            root,
            &self.syntax_set,
            &self.class_style,
            self.mermaid.as_ref(),
            request.target.slug(),
        )?;

        let rendered_html = render_html_stage(root, &self.options)?;
        let restored_html = restore_stage(rendered_html, &rewrite_outcome);

        Ok(restored_html)
    }
}

#[derive(Debug, Clone)]
pub struct RenderPipelineConfig {
    pub mermaid_cli_path: PathBuf,
    pub mermaid_cache_dir: PathBuf,
}

impl Default for RenderPipelineConfig {
    fn default() -> Self {
        Self {
            mermaid_cli_path: PathBuf::from(DEFAULT_MERMAID_CLI_PATH),
            mermaid_cache_dir: PathBuf::from(DEFAULT_MERMAID_CACHE_DIR),
        }
    }
}

impl From<&crate::config::RenderSettings> for RenderPipelineConfig {
    fn from(settings: &crate::config::RenderSettings) -> Self {
        Self {
            mermaid_cli_path: settings.mermaid_cli_path.clone(),
            mermaid_cache_dir: settings.mermaid_cache_dir.clone(),
        }
    }
}

#[derive(Debug, Error)]
pub enum RenderConfigError {
    #[error("render service already configured")]
    AlreadyConfigured,
}

static RENDER_PIPELINE_CONFIG: OnceCell<RenderPipelineConfig> = OnceCell::new();

pub fn configure_render_service(config: RenderPipelineConfig) -> Result<(), RenderConfigError> {
    RENDER_PIPELINE_CONFIG
        .set(config)
        .map_err(|_| RenderConfigError::AlreadyConfigured)
}

fn active_render_config() -> RenderPipelineConfig {
    RENDER_PIPELINE_CONFIG.get().cloned().unwrap_or_default()
}

fn log_mermaid_init_error(error: &MermaidRenderError, config: &RenderPipelineConfig) {
    warn!(
        target = "application::render::mermaid",
        cli_path = %config.mermaid_cli_path.display(),
        cache_dir = %config.mermaid_cache_dir.display(),
        error = %error,
        "Mermaid renderer disabled"
    );
}

fn rewrite_stage<'a>(
    root: &'a AstNode<'a>,
    syntax_set: &SyntaxSet,
    class_style: &ClassStyle,
    mermaid_renderer: Option<&MermaidRenderer>,
    slug: &str,
) -> Result<rewrite::RewriteOutcome, RenderError> {
    rewrite_ast(root, syntax_set, class_style, mermaid_renderer, slug)
}

fn render_html_stage<'a>(
    root: &'a AstNode<'a>,
    options: &comrak::Options<'static>,
) -> Result<String, RenderError> {
    let mut html = String::new();
    format_html(root, options, &mut html).map_err(|err| RenderError::Markdown {
        message: err.to_string(),
    })?;
    Ok(html)
}

fn sanitize_stage(
    html: String,
    target: &RenderTarget,
    post_sanitizer: &ammonia::Builder<'static>,
    page_sanitizer: &ammonia::Builder<'static>,
) -> Result<String, RenderError> {
    let sanitizer = match target {
        RenderTarget::PageBody { .. } => page_sanitizer,
        _ => post_sanitizer,
    };
    Ok(sanitizer.clean(&html).to_string())
}

fn restore_stage(html: String, rewrite_outcome: &rewrite::RewriteOutcome) -> String {
    let with_math = rewrite_outcome
        .math_fragments
        .iter()
        .fold(html, |acc, fragment| {
            if fragment.is_block {
                let placeholder = format!("<div>{}</div>", fragment.placeholder);
                acc.replace(&placeholder, &fragment.html)
            } else {
                acc.replace(&fragment.placeholder, &fragment.html)
            }
        });

    rewrite_outcome
        .mermaid_fragments
        .iter()
        .fold(with_math, |acc, fragment| {
            let placeholder = format!("<div>{}</div>", fragment.placeholder);
            acc.replace(&placeholder, &fragment.html)
        })
}

fn post_process_stage(
    html: &str,
    target: &RenderTarget,
    headings: &[rewrite::HeadingInfo],
) -> Result<ProcessedHtml, RenderError> {
    post_process(html, target, headings)
}
