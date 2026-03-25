use comrak::nodes::{AstNode, NodeValue};

pub(super) fn build_plain_code_block(language: &str, literal: &str) -> String {
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

pub(super) fn extract_code_block(node: &AstNode<'_>) -> Option<(String, String)> {
    let data = node.data.borrow();
    if let NodeValue::CodeBlock(block) = &data.value {
        let info = block.info.trim().to_string();
        let literal = block.literal.clone();
        Some((info, literal))
    } else {
        None
    }
}

pub(super) fn heading_level(node: &AstNode<'_>) -> Option<u8> {
    let data = node.data.borrow();
    if let NodeValue::Heading(heading) = &data.value {
        Some(heading.level)
    } else {
        None
    }
}

pub(super) fn collect_heading_text(node: &AstNode<'_>) -> String {
    collect_inline_text(node)
}

pub(super) fn collect_inline_text(node: &AstNode<'_>) -> String {
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

pub(super) fn escape_attribute(value: &str) -> String {
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
