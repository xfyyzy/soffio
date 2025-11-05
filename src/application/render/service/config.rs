use std::{borrow::Cow, collections::HashSet};

use ammonia::Builder as AmmoniaBuilder;
use comrak::options::{ListStyleType, Options};

pub(crate) fn default_options() -> Options<'static> {
    let mut options = Options::default();
    configure_extensions(&mut options);
    options
}

pub(crate) fn build_post_sanitizer() -> AmmoniaBuilder<'static> {
    base_builder()
}

pub(crate) fn build_page_sanitizer() -> AmmoniaBuilder<'static> {
    let mut builder = base_builder();

    builder.add_tags(&["style", "video", "audio", "source", "picture", "track"]);
    builder.rm_clean_content_tags(&["style"]);
    builder.add_generic_attributes(&["style"]);
    builder.add_generic_attribute_prefixes(&["data-"]);

    builder.attribute_filter(|_element, attribute, value| {
        if attribute.eq_ignore_ascii_case("style") {
            sanitize_style_attribute(value).map(Cow::Owned)
        } else {
            Some(Cow::Borrowed(value))
        }
    });

    builder
}

fn base_builder() -> AmmoniaBuilder<'static> {
    let mut builder = AmmoniaBuilder::default();

    let tags: HashSet<&'static str> = HashSet::from([
        "a",
        "abbr",
        "blockquote",
        "br",
        "code",
        "div",
        "em",
        "figcaption",
        "figure",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "hr",
        "i",
        "img",
        "input",
        "ins",
        "kbd",
        "li",
        "ol",
        "p",
        "pre",
        "s",
        "section",
        "span",
        "strong",
        "sub",
        "sup",
        "u",
        "table",
        "tbody",
        "td",
        "th",
        "thead",
        "tr",
        "ul",
        "dl",
        "dt",
        "dd",
        "del",
        "svg",
        "g",
        "path",
        "rect",
        "circle",
        "ellipse",
        "polygon",
        "polyline",
        "line",
        "marker",
        "defs",
        "lineargradient",
        "linearGradient",
        "stop",
        "title",
        "desc",
        "text",
        "tspan",
        "use",
        "clipPath",
        "clippath",
        "mark",
    ]);
    builder.tags(tags);

    let generic: HashSet<&'static str> = HashSet::from([
        "class",
        "id",
        "title",
        "lang",
        "dir",
        "aria-hidden",
        "aria-label",
        "role",
        "data-footnote-ref",
        "data-footnotes",
        "data-footnote-backref",
        "data-footnote-backref-idx",
        "data-math-style",
        "data-sourcepos",
    ]);
    builder.generic_attributes(generic);

    builder.add_tag_attributes("a", &["target"]);
    builder.add_tag_attributes(
        "img",
        &[
            "title",
            "width",
            "height",
            "alt",
            "loading",
            "decoding",
            "fetchpriority",
        ],
    );
    builder.add_tag_attributes(
        "code",
        &["data-meta", "data-language", "class", "data-math-style"],
    );
    builder.add_tag_attributes("pre", &["class", "data-language"]);
    builder.add_tag_attributes("div", &["class", "data-footnotes"]);
    builder.add_tag_attributes("span", &["class", "data-math-style"]);
    builder.add_tag_attributes("th", &["align", "colspan", "rowspan", "scope"]);
    builder.add_tag_attributes("td", &["align", "colspan", "rowspan"]);
    builder.add_tag_attributes("input", &["type", "checked", "disabled", "class"]);
    builder.add_tag_attributes(
        "svg",
        &[
            "viewBox",
            "xmlns",
            "xmlns:xlink",
            "width",
            "height",
            "preserveAspectRatio",
            "version",
        ],
    );
    builder.add_tag_attributes("g", &["transform", "class", "id", "data-name"]);
    builder.add_tag_attributes(
        "path",
        &[
            "d",
            "fill",
            "stroke",
            "stroke-width",
            "stroke-linecap",
            "stroke-linejoin",
            "marker-end",
            "marker-start",
            "opacity",
            "class",
        ],
    );
    builder.add_tag_attributes(
        "rect",
        &[
            "x",
            "y",
            "width",
            "height",
            "rx",
            "ry",
            "fill",
            "stroke",
            "stroke-width",
            "class",
            "opacity",
        ],
    );
    builder.add_tag_attributes(
        "circle",
        &[
            "cx",
            "cy",
            "r",
            "fill",
            "stroke",
            "stroke-width",
            "class",
            "opacity",
        ],
    );
    builder.add_tag_attributes(
        "ellipse",
        &[
            "cx",
            "cy",
            "rx",
            "ry",
            "fill",
            "stroke",
            "stroke-width",
            "class",
            "opacity",
        ],
    );
    builder.add_tag_attributes(
        "polygon",
        &[
            "points",
            "fill",
            "stroke",
            "stroke-width",
            "class",
            "opacity",
        ],
    );
    builder.add_tag_attributes(
        "polyline",
        &[
            "points",
            "fill",
            "stroke",
            "stroke-width",
            "class",
            "opacity",
        ],
    );
    builder.add_tag_attributes(
        "line",
        &[
            "x1",
            "x2",
            "y1",
            "y2",
            "stroke",
            "stroke-width",
            "class",
            "opacity",
        ],
    );
    builder.add_tag_attributes(
        "marker",
        &[
            "id",
            "refX",
            "refY",
            "orient",
            "markerWidth",
            "markerHeight",
            "viewBox",
        ],
    );
    builder.add_tag_attributes(
        "text",
        &[
            "x",
            "y",
            "fill",
            "stroke",
            "stroke-width",
            "text-anchor",
            "dominant-baseline",
            "class",
            "font-size",
        ],
    );
    builder.add_tag_attributes(
        "tspan",
        &["x", "y", "dx", "dy", "font-size", "fill", "class"],
    );
    builder.add_tag_attributes(
        "lineargradient",
        &["id", "gradientUnits", "x1", "x2", "y1", "y2"],
    );
    builder.add_tag_attributes(
        "linearGradient",
        &["id", "gradientUnits", "x1", "x2", "y1", "y2"],
    );
    builder.add_tag_attributes("stop", &["offset", "stop-color", "stop-opacity"]);
    builder.add_tag_attributes("use", &["href", "xlink:href", "x", "y", "width", "height"]);
    builder.add_tag_attributes("clipPath", &["id"]);
    builder.add_tag_attributes("clippath", &["id"]);

    builder.add_url_schemes(["http", "https", "mailto", "tel"].iter().copied());

    builder
}

fn configure_extensions(options: &mut Options<'static>) {
    let ext = &mut options.extension;
    ext.strikethrough = true;
    ext.tagfilter = false;
    ext.table = true;
    ext.autolink = true;
    ext.tasklist = true;
    ext.superscript = true;
    ext.footnotes = true;
    ext.inline_footnotes = true;
    ext.description_lists = true;
    ext.front_matter_delimiter = Some("---".to_string());
    ext.multiline_block_quotes = true;
    ext.alerts = true;
    ext.math_dollars = true;
    ext.math_code = true;
    ext.wikilinks_title_after_pipe = true;
    ext.underline = true;
    ext.subscript = true;
    ext.spoiler = true;
    ext.greentext = true;
    ext.cjk_friendly_emphasis = true;

    let render = &mut options.render;
    render.github_pre_lang = true;
    render.full_info_string = true;
    render.tasklist_classes = true;
    render.list_style = ListStyleType::Dash;
    render.r#unsafe = true;
    render.figure_with_caption = true;
    render.sourcepos = false;
    render.escaped_char_spans = true;
    render.gfm_quirks = true;
}

fn sanitize_style_attribute(value: &str) -> Option<String> {
    let mut sanitized = Vec::new();

    for declaration in value.split(';') {
        let decl = declaration.trim();
        if decl.is_empty() {
            continue;
        }

        if is_safe_style_declaration(decl) {
            sanitized.push(decl);
        }
    }

    if sanitized.is_empty() {
        None
    } else {
        Some(sanitized.join("; "))
    }
}

fn is_safe_style_declaration(decl: &str) -> bool {
    let lower = decl.to_ascii_lowercase();

    const FORBIDDEN_SUBSTRINGS: [&str; 6] = [
        "expression(",
        "javascript:",
        "vbscript:",
        "-moz-binding",
        "behavior:",
        "behaviour:",
    ];

    if FORBIDDEN_SUBSTRINGS
        .iter()
        .any(|needle| lower.contains(needle))
    {
        return false;
    }

    if lower.contains("@import") {
        return false;
    }

    !contains_unsafe_url(&lower)
}

fn contains_unsafe_url(lower_decl: &str) -> bool {
    let mut offset = 0;

    while let Some(start) = lower_decl[offset..].find("url(") {
        let open = offset + start + 4; // skip "url("
        let rest = &lower_decl[open..];
        if let Some(close_rel) = rest.find(')') {
            let close = open + close_rel;
            let target = &lower_decl[open..close];
            let trimmed = target.trim_matches(|c: char| c.is_whitespace() || c == '\'');
            let trimmed = trimmed.trim_matches('"');

            if is_unsafe_url(trimmed) {
                return true;
            }

            offset = close + 1;
        } else {
            // malformed url, treat as unsafe
            return true;
        }
    }

    false
}

fn is_unsafe_url(url: &str) -> bool {
    if url.starts_with("data:image/") {
        return false;
    }

    url.starts_with("javascript:")
        || url.starts_with("vbscript:")
        || url.starts_with("data:")
        || url.starts_with("file:")
        || url.contains("javascript:")
        || url.contains("vbscript:")
}

#[cfg(test)]
mod tests {
    use super::{contains_unsafe_url, sanitize_style_attribute};

    #[test]
    fn sanitize_style_attribute_preserves_safe_rules() {
        let input = "color: red; padding: 4px;";
        let output = sanitize_style_attribute(input);
        assert_eq!(output.unwrap(), "color: red; padding: 4px");
    }

    #[test]
    fn sanitize_style_attribute_drops_unsafe_rules() {
        let input = "color: red; background: url('javascript:alert(1)'); expression(test);";
        let output = sanitize_style_attribute(input);
        assert_eq!(output.unwrap(), "color: red");
    }

    #[test]
    fn sanitize_style_attribute_returns_none_when_only_unsafe() {
        let input = "background-image: url('javascript:alert(1)');";
        assert!(sanitize_style_attribute(input).is_none());
    }

    #[test]
    fn detects_unsafe_urls() {
        assert!(contains_unsafe_url("background:url(javascript:alert(1))"));
        assert!(!contains_unsafe_url(
            "background:url('https://example.com/bg.png')"
        ));
        assert!(!contains_unsafe_url(
            "background:url('data:image/png;base64,AAAA')"
        ));
    }

    #[test]
    fn page_sanitizer_allows_style_tag_and_attributes() {
        let sanitizer = super::build_page_sanitizer();
        let html = sanitizer
            .clean("<style>.hero { color: red; }</style><div style=\"color: red;\">Hi</div>")
            .to_string();

        assert!(html.contains("<style>"));
        assert!(html.contains("style=\"color: red\""));
    }

    #[test]
    fn post_sanitizer_preserves_strikethrough() {
        let sanitizer = super::build_post_sanitizer();
        let html = sanitizer
            .clean("<p><del>Removed</del> text</p>")
            .to_string();

        assert!(html.contains("<del>Removed</del>"));
    }

    #[test]
    fn post_sanitizer_preserves_underline() {
        let sanitizer = super::build_post_sanitizer();
        let html = sanitizer.clean("<p><u>Underline</u> text</p>").to_string();

        assert!(html.contains("<u>Underline</u>"));
    }
}
