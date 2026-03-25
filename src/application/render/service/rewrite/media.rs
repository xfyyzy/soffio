use std::{borrow::Cow, num::NonZeroU32};

use comrak::nodes::{AstNode, NodeValue};
use url::form_urlencoded;

use crate::{
    application::{
        metadata::{MAX_DIMENSION, metadata_registry},
        render::types::RenderError,
    },
    domain::uploads::{METADATA_HEIGHT, METADATA_WIDTH},
};

use super::utils::{collect_inline_text, escape_attribute};

pub(super) fn process_image_node(node: &AstNode<'_>) -> Result<(), RenderError> {
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
