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

    let outcome = rewrite_ast(root, &syntax_set, &class_style, None, "math-test").expect("rewrite");
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
