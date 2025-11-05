use std::{borrow::Cow, num::NonZeroU32, vec::Vec};

use comrak::nodes::{AstNode, NodeHtmlBlock, NodeValue};
use syntect::html::ClassStyle;
use syntect::parsing::SyntaxSet;
use tracing::warn;

use crate::{
    application::{
        metadata::{MAX_DIMENSION, metadata_registry},
        render::types::RenderError,
    },
    domain::{
        slug::AnchorSlugger,
        uploads::{METADATA_HEIGHT, METADATA_WIDTH},
    },
};
use url::form_urlencoded;

use super::{highlight, math, mermaid::MermaidRenderer};

#[derive(Debug, Clone)]
pub(crate) struct HeadingInfo {
    pub(crate) level: u8,
    pub(crate) slug: String,
    pub(crate) text: String,
    pub(crate) has_block_code: bool,
    pub(crate) has_math_block: bool,
    pub(crate) has_inline_math: bool,
    pub(crate) has_mermaid_block: bool,
}

#[derive(Default)]
pub(crate) struct RewriteOutcome {
    pub(crate) contains_code: bool,
    pub(crate) contains_math: bool,
    pub(crate) contains_mermaid: bool,
    pub(crate) headings: Vec<HeadingInfo>,
    pub(crate) mermaid_fragments: Vec<MermaidFragment>,
    pub(crate) math_fragments: Vec<MathFragment>,
    mermaid_counter: usize,
    math_counter: usize,
}

#[derive(Clone)]
pub(crate) struct MermaidFragment {
    pub(crate) placeholder: String,
    pub(crate) html: String,
}

#[derive(Clone)]
pub(crate) struct MathFragment {
    pub(crate) placeholder: String,
    pub(crate) html: String,
    pub(crate) is_block: bool,
}

pub(crate) fn rewrite_ast<'a>(
    root: &'a AstNode<'a>,
    syntax_set: &SyntaxSet,
    class_style: &ClassStyle,
    mermaid: Option<&MermaidRenderer>,
    slug: &str,
) -> Result<RewriteOutcome, RenderError> {
    let mut outcome = RewriteOutcome::default();
    let mut slugger = AnchorSlugger::new();
    let mut heading_stack = Vec::new();
    visit_nodes(
        root,
        syntax_set,
        class_style,
        &mut outcome,
        &mut slugger,
        &mut heading_stack,
        mermaid,
        slug,
    )?;
    Ok(outcome)
}

#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
fn visit_nodes(
    node: &AstNode<'_>,
    syntax_set: &SyntaxSet,
    class_style: &ClassStyle,
    outcome: &mut RewriteOutcome,
    slugger: &mut AnchorSlugger,
    heading_stack: &mut Vec<usize>,
    mermaid: Option<&MermaidRenderer>,
    slug: &str,
) -> Result<(), RenderError> {
    if {
        let data = node.data.borrow();
        matches!(data.value, NodeValue::Image(_))
    } {
        process_image_node(node)?;
    }

    if let Some(level) = heading_level(node) {
        let text = collect_heading_text(node);
        let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let slug = slugger
            .anchor_for(normalized.trim())
            .map_err(|err| RenderError::Anchoring {
                message: err.to_string(),
            })?;
        while let Some(&idx) = heading_stack.last() {
            if outcome.headings[idx].level < level {
                break;
            }
            heading_stack.pop();
        }
        outcome.headings.push(HeadingInfo {
            level,
            slug,
            text: normalized.trim().to_string(),
            has_block_code: false,
            has_math_block: false,
            has_inline_math: false,
            has_mermaid_block: false,
        });
        heading_stack.push(outcome.headings.len() - 1);
    }

    if handle_math_node(node, outcome, heading_stack, syntax_set, class_style, slug)? {
        // Math nodes are fully handled; skip further processing.
    } else if let Some((info, literal)) = extract_code_block(node) {
        let mut segments = info.split_whitespace();
        let language_owned = segments.next().map(|s| s.to_string());
        let meta_string = segments.collect::<Vec<_>>().join(" ");
        let language_ref = language_owned.as_deref();

        if handle_mermaid_block(
            node,
            language_ref,
            &literal,
            syntax_set,
            class_style,
            outcome,
            heading_stack,
            mermaid,
            slug,
        )? {
            // Mermaid block handled (successfully rendered or gracefully degraded).
            // Skip syntax highlighting path.
        } else {
            let meta_ref = (!meta_string.is_empty()).then_some(meta_string.as_str());
            let html = highlight::highlight_code(
                language_ref,
                meta_ref,
                &literal,
                syntax_set,
                class_style,
            )?;
            outcome.contains_code = true;
            if let Some(&idx) = heading_stack.last() {
                outcome.headings[idx].has_block_code = true;
            }
            let mut data = node.data.borrow_mut();
            data.value = NodeValue::HtmlBlock(NodeHtmlBlock {
                block_type: 0,
                literal: html,
            });
        }
    }

    let mut child = node.first_child();
    while let Some(next) = child {
        visit_nodes(
            next,
            syntax_set,
            class_style,
            outcome,
            slugger,
            heading_stack,
            mermaid,
            slug,
        )?;
        child = next.next_sibling();
    }

    Ok(())
}

fn handle_math_node(
    node: &AstNode<'_>,
    outcome: &mut RewriteOutcome,
    heading_stack: &mut Vec<usize>,
    syntax_set: &SyntaxSet,
    class_style: &ClassStyle,
    slug: &str,
) -> Result<bool, RenderError> {
    let math_data = {
        let data = node.data.borrow();
        if let NodeValue::Math(math_node) = &data.value {
            Some((math_node.literal.clone(), math_node.display_math))
        } else {
            None
        }
    };

    let Some((literal_bytes, display_mode)) = math_data else {
        return Ok(false);
    };

    let literal = literal_bytes;

    match math::render_math_html(&literal, display_mode) {
        Ok(html) => {
            let container = if display_mode {
                format!("<div data-role=\"math-block\" data-math-style=\"display\">{html}</div>")
            } else {
                format!("<span data-role=\"math-inline\" data-math-style=\"inline\">{html}</span>")
            };

            let placeholder = format!("__KATEX_PLACEHOLDER_{}__", outcome.math_counter);
            outcome.math_counter = outcome.math_counter.saturating_add(1);
            outcome.math_fragments.push(MathFragment {
                placeholder: placeholder.clone(),
                html: container,
                is_block: display_mode,
            });

            if display_mode {
                let mut data = node.data.borrow_mut();
                data.value = NodeValue::HtmlBlock(NodeHtmlBlock {
                    block_type: 0,
                    literal: format!("<div>{placeholder}</div>"),
                });
            } else {
                let mut data = node.data.borrow_mut();
                data.value = NodeValue::HtmlInline(placeholder.clone());
            }

            outcome.contains_math = true;
            if let Some(&idx) = heading_stack.last() {
                if display_mode {
                    outcome.headings[idx].has_math_block = true;
                } else {
                    outcome.headings[idx].has_inline_math = true;
                }
            }
            Ok(true)
        }
        Err(err) => {
            warn!(
                target = "application::render::math",
                slug, "KaTeX rendering failed: {err}"
            );
            apply_math_fallback(
                node,
                literal.as_str(),
                display_mode,
                outcome,
                heading_stack,
                syntax_set,
                class_style,
            )?;
            Ok(true)
        }
    }
}

#[allow(clippy::ptr_arg)]
fn apply_math_fallback(
    node: &AstNode<'_>,
    literal: &str,
    display_mode: bool,
    outcome: &mut RewriteOutcome,
    heading_stack: &mut Vec<usize>,
    syntax_set: &SyntaxSet,
    class_style: &ClassStyle,
) -> Result<(), RenderError> {
    outcome.contains_code = true;

    if display_mode {
        let highlighted =
            highlight::highlight_code(Some("math"), None, literal, syntax_set, class_style)
                .unwrap_or_else(|_| build_plain_code_block("math", literal));

        let mut data = node.data.borrow_mut();
        data.value = NodeValue::HtmlBlock(NodeHtmlBlock {
            block_type: 0,
            literal: highlighted,
        });

        if let Some(&idx) = heading_stack.last() {
            outcome.headings[idx].has_block_code = true;
            outcome.headings[idx].has_math_block = true;
        }
    } else {
        let escaped = ammonia::clean_text(literal);
        let fallback = format!("<code data-math-style=\"inline\">{escaped}</code>",);
        let mut data = node.data.borrow_mut();
        data.value = NodeValue::HtmlInline(fallback);
        if let Some(&idx) = heading_stack.last() {
            outcome.headings[idx].has_inline_math = true;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
fn handle_mermaid_block(
    node: &AstNode<'_>,
    language: Option<&str>,
    literal: &str,
    syntax_set: &SyntaxSet,
    class_style: &ClassStyle,
    outcome: &mut RewriteOutcome,
    heading_stack: &mut Vec<usize>,
    renderer: Option<&MermaidRenderer>,
    slug: &str,
) -> Result<bool, RenderError> {
    let Some(lang) = language.map(|lang| lang.to_ascii_lowercase()) else {
        return Ok(false);
    };

    if !matches!(lang.as_str(), "mermaid" | "mermind") {
        return Ok(false);
    }

    let Some(renderer) = renderer else {
        warn!(
            target = "application::render::mermaid",
            slug, "Mermaid renderer unavailable; falling back to code block"
        );
        apply_mermaid_fallback(
            node,
            language,
            literal,
            syntax_set,
            class_style,
            outcome,
            heading_stack,
        )?;
        return Ok(true);
    };

    match renderer.render_svg(literal) {
        Ok(svg) => {
            let fragment = format!("<figure data-role=\"diagram-mermaid\">{svg}</figure>");
            let placeholder_key = format!("__MERMAID_PLACEHOLDER_{}__", outcome.mermaid_counter);
            outcome.mermaid_counter = outcome.mermaid_counter.saturating_add(1);
            outcome.mermaid_fragments.push(MermaidFragment {
                placeholder: placeholder_key.clone(),
                html: fragment,
            });
            let placeholder_html = format!("<div>{placeholder_key}</div>");

            let mut data = node.data.borrow_mut();
            data.value = NodeValue::HtmlBlock(NodeHtmlBlock {
                block_type: 0,
                literal: placeholder_html,
            });
            outcome.contains_mermaid = true;
            if let Some(&idx) = heading_stack.last() {
                outcome.headings[idx].has_mermaid_block = true;
            }
            Ok(true)
        }
        Err(err) => {
            warn!(
                target = "application::render::mermaid",
                slug, "Mermaid CLI failed: {err}"
            );
            apply_mermaid_fallback(
                node,
                language,
                literal,
                syntax_set,
                class_style,
                outcome,
                heading_stack,
            )?;
            Ok(true)
        }
    }
}

#[allow(clippy::ptr_arg)]
fn apply_mermaid_fallback(
    node: &AstNode<'_>,
    language: Option<&str>,
    literal: &str,
    syntax_set: &SyntaxSet,
    class_style: &ClassStyle,
    outcome: &mut RewriteOutcome,
    heading_stack: &mut Vec<usize>,
) -> Result<(), RenderError> {
    let highlighted = highlight::highlight_code(language, None, literal, syntax_set, class_style)
        .unwrap_or_else(|_| build_plain_code_block(language.unwrap_or("text"), literal));

    outcome.contains_code = true;
    if let Some(&idx) = heading_stack.last() {
        outcome.headings[idx].has_block_code = true;
    }

    let mut data = node.data.borrow_mut();
    data.value = NodeValue::HtmlBlock(NodeHtmlBlock {
        block_type: 0,
        literal: highlighted,
    });

    Ok(())
}

fn build_plain_code_block(language: &str, literal: &str) -> String {
    let escaped_code = ammonia::clean_text(literal);
    let mut html = String::from("<pre class=\"syntax-highlight\"");
    if !language.is_empty() {
        html.push_str(" data-language=\"");
        html.push_str(&escape_attribute(language));
        html.push('"');
    }
    html.push_str("><code>");
    html.push_str(&escaped_code);
    if !escaped_code.ends_with('\n') {
        html.push('\n');
    }
    html.push_str("</code></pre>");
    html
}

fn process_image_node(node: &AstNode<'_>) -> Result<(), RenderError> {
    let (src, title) = {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Image(link) => (link.url.clone(), link.title.clone()),
            _ => return Ok(()),
        }
    };

    let alt_raw = collect_inline_text(node);
    let alt = alt_raw.split_whitespace().collect::<Vec<_>>().join(" ");

    let mut width: Option<NonZeroU32> = None;
    let mut height: Option<NonZeroU32> = None;

    if let Some((explicit_width, explicit_height)) = extract_markdown_dimensions(node) {
        width = Some(explicit_width);
        height = Some(explicit_height);
    }

    if width.is_none() || height.is_none() {
        let query_pairs = parse_image_query_pairs(&src);
        let query_metadata = metadata_registry()
            .extract_from_query("img", &query_pairs)
            .map_err(|err| RenderError::Document {
                message: err.to_string(),
            })?;

        if let Some(metadata) = query_metadata.as_ref() {
            if width.is_none() {
                width = metadata.integer(METADATA_WIDTH).and_then(NonZeroU32::new);
            }
            if height.is_none() {
                height = metadata.integer(METADATA_HEIGHT).and_then(NonZeroU32::new);
            }
        }
    }

    let html = build_image_html(
        &src,
        alt.trim(),
        (!title.is_empty()).then_some(title.as_str()),
        width,
        height,
    );

    {
        let mut data = node.data.borrow_mut();
        data.value = NodeValue::HtmlInline(html);
    }

    while let Some(child) = node.first_child() {
        child.detach();
    }

    Ok(())
}
fn build_image_html(
    src: &str,
    alt: &str,
    title: Option<&str>,
    width: Option<NonZeroU32>,
    height: Option<NonZeroU32>,
) -> String {
    let mut html = String::with_capacity(src.len() + alt.len() + 64);
    html.push_str("<img data-role=\"post-image\"");
    html.push_str(" src=\"");
    html.push_str(&escape_attribute(src));
    html.push('\"');

    let escaped_alt = escape_attribute(alt);
    html.push_str(" alt=\"");
    html.push_str(&escaped_alt);
    html.push('\"');

    if let Some(title) = title.and_then(|t| (!t.is_empty()).then_some(t)) {
        html.push_str(" title=\"");
        html.push_str(&escape_attribute(title));
        html.push('\"');
    }

    if let (Some(w), Some(h)) = (width, height) {
        let width_str = w.get().to_string();
        let height_str = h.get().to_string();
        html.push_str(" width=\"");
        html.push_str(&width_str);
        html.push('\"');
        html.push_str(" height=\"");
        html.push_str(&height_str);
        html.push('\"');
    }

    html.push_str(" />");
    html
}

fn parse_image_query_pairs(url: &str) -> Vec<(Cow<'_, str>, Cow<'_, str>)> {
    let before_fragment = url.split('#').next().unwrap_or(url);
    if let Some((_, query)) = before_fragment.split_once('?') {
        form_urlencoded::parse(query.as_bytes()).collect()
    } else {
        Vec::new()
    }
}

fn escape_attribute(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\n' | '\r' | '\t' => escaped.push(' '),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn collect_inline_text(node: &AstNode<'_>) -> String {
    fn walk(node: &AstNode<'_>, buffer: &mut String) {
        {
            let data = node.data.borrow();
            match &data.value {
                NodeValue::Text(text) => buffer.push_str(text),
                NodeValue::Code(code) => buffer.push_str(&code.literal),
                NodeValue::LineBreak | NodeValue::SoftBreak => buffer.push(' '),
                _ => {}
            }
        }
        let mut child = node.first_child();
        while let Some(next) = child {
            walk(next, buffer);
            child = next.next_sibling();
        }
    }

    let mut text = String::new();
    let mut child = node.first_child();
    while let Some(next) = child {
        walk(next, &mut text);
        child = next.next_sibling();
    }
    text
}

fn extract_markdown_dimensions(node: &AstNode<'_>) -> Option<(NonZeroU32, NonZeroU32)> {
    let mut sibling = node.next_sibling();
    while let Some(current) = sibling {
        sibling = current.next_sibling();
        let mut should_continue = false;
        let mut should_detach = false;
        let dimensions = {
            let data = current.data.borrow();
            match &data.value {
                NodeValue::Text(text) => {
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        should_continue = true;
                        should_detach = true;
                        None
                    } else if let Some(dims) = parse_dimension_block(trimmed) {
                        should_detach = true;
                        Some(dims)
                    } else {
                        None
                    }
                }
                NodeValue::SoftBreak | NodeValue::LineBreak => {
                    should_continue = true;
                    should_detach = true;
                    None
                }
                _ => None,
            }
        };

        if let Some((width, height)) = dimensions {
            if should_detach {
                current.detach();
            }
            return Some((width, height));
        }

        if should_continue {
            continue;
        }

        break;
    }

    None
}

fn parse_dimension_block(value: &str) -> Option<(NonZeroU32, NonZeroU32)> {
    if !value.starts_with('{') || !value.ends_with('}') {
        return None;
    }

    let mut inner = &value[1..value.len() - 1];
    inner = inner.trim();
    if let Some(stripped) = inner.strip_prefix(':') {
        inner = stripped.trim_start();
    }

    if inner.is_empty() {
        return None;
    }

    let mut width: Option<NonZeroU32> = None;
    let mut height: Option<NonZeroU32> = None;

    for token in inner.split_whitespace() {
        if let Some((key, raw)) = token.split_once('=') {
            let key = key.trim();
            let raw_trimmed = raw.trim_matches('"');
            match key {
                METADATA_WIDTH => {
                    if let Some(value) = parse_dimension_value(raw_trimmed) {
                        width = Some(value);
                    }
                }
                METADATA_HEIGHT => {
                    if let Some(value) = parse_dimension_value(raw_trimmed) {
                        height = Some(value);
                    }
                }
                _ => {}
            }
        }
    }

    match (width, height) {
        (Some(w), Some(h)) => Some((w, h)),
        _ => None,
    }
}

fn parse_dimension_value(raw: &str) -> Option<NonZeroU32> {
    if raw.is_empty() || !raw.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }

    let value: u32 = raw.parse().ok()?;
    if value == 0 || value > MAX_DIMENSION {
        return None;
    }

    NonZeroU32::new(value)
}

fn extract_code_block(node: &AstNode<'_>) -> Option<(String, String)> {
    let data = node.data.borrow();
    if let NodeValue::CodeBlock(block) = &data.value {
        let info = block.info.trim().to_string();
        let literal = block.literal.clone();
        Some((info, literal))
    } else {
        None
    }
}

fn heading_level(node: &AstNode<'_>) -> Option<u8> {
    let data = node.data.borrow();
    if let NodeValue::Heading(heading) = &data.value {
        Some(heading.level)
    } else {
        None
    }
}

fn collect_heading_text(node: &AstNode<'_>) -> String {
    collect_inline_text(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use comrak::{Arena, format_html, parse_document};
    use syntect::parsing::SyntaxSet;

    fn syntax_and_style() -> (SyntaxSet, ClassStyle) {
        (
            SyntaxSet::load_defaults_newlines(),
            ClassStyle::SpacedPrefixed { prefix: "syntax-" },
        )
    }

    #[test]
    fn rewrite_inline_math_renders_with_katex() {
        let options = crate::application::render::service::config::default_options();
        let arena = Arena::new();
        let root = parse_document(&arena, "$a^2$", &options);
        let (syntax_set, class_style) = syntax_and_style();

        let outcome =
            rewrite_ast(root, &syntax_set, &class_style, None, "math-test").expect("rewrite");
        assert!(outcome.contains_math);
        assert_eq!(outcome.math_fragments.len(), 1);

        let mut html = String::new();
        format_html(root, &options, &mut html).expect("html");
        assert!(html.contains("__KATEX_PLACEHOLDER_0__"));
        assert!(!html.contains("class=\"katex"));

        let restored = outcome.math_fragments.iter().fold(html, |acc, fragment| {
            if fragment.is_block {
                let placeholder = format!("<div>{}</div>", fragment.placeholder);
                acc.replace(&placeholder, &fragment.html)
            } else {
                acc.replace(&fragment.placeholder, &fragment.html)
            }
        });

        assert!(restored.contains("data-role=\"math-inline\""));
        assert!(restored.contains("class=\"katex"));
    }

    #[test]
    fn rewrite_mermaid_without_renderer_falls_back_to_code() {
        let options = crate::application::render::service::config::default_options();
        let arena = Arena::new();
        let markdown = "```mermaid\ngraph TD;A-->B;\n```";
        let root = parse_document(&arena, markdown, &options);
        let (syntax_set, class_style) = syntax_and_style();

        let outcome =
            rewrite_ast(root, &syntax_set, &class_style, None, "mermaid-test").expect("rewrite");
        assert!(outcome.contains_code);
        assert!(!outcome.contains_mermaid);

        let mut html = String::new();
        format_html(root, &options, &mut html).expect("html");
        assert!(html.contains("<pre"));
        assert!(html.contains("syntax-highlight"));
    }
}
