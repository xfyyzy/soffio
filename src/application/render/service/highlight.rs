use syntect::{
    html::{ClassStyle, ClassedHTMLGenerator},
    parsing::{SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};

use crate::application::render::types::RenderError;

pub(crate) fn highlight_code(
    language: Option<&str>,
    meta: Option<&str>,
    code: &str,
    syntax_set: &SyntaxSet,
    class_style: &ClassStyle,
) -> Result<String, RenderError> {
    let lang_token = language.unwrap_or("text");
    let syntax =
        find_syntax(syntax_set, lang_token).unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let mut code_with_newline = code.to_string();
    if !code_with_newline.ends_with('\n') {
        code_with_newline.push('\n');
    }

    let mut generator =
        ClassedHTMLGenerator::new_with_class_style(syntax, syntax_set, *class_style);

    for line in LinesWithEndings::from(code_with_newline.as_str()) {
        generator
            .parse_html_for_line_which_includes_newline(line)
            .map_err(|err| RenderError::Highlighting {
                language: lang_token.to_string(),
                message: err.to_string(),
            })?;
    }

    let highlighted = generator.finalize();
    let mut code_classes = vec![format!("language-{}", lang_token.to_ascii_lowercase())];
    code_classes.push("syntax-code".to_string());
    let pre_class = format!(
        "syntax-highlight syntax-lang-{}",
        lang_token.to_ascii_lowercase()
    );

    let meta_attr = meta
        .filter(|m| !m.is_empty())
        .map(|m| format!(" data-meta=\"{}\"", ammonia::clean_text(m)))
        .unwrap_or_default();

    let lang_attr = format!(" data-language=\"{}\"", lang_token);
    Ok(format!(
        "<pre class=\"{pre_class}\"{lang_attr}><code class=\"{}\"{meta_attr}>{}</code></pre>",
        code_classes.join(" "),
        highlighted
    ))
}

fn find_syntax<'a>(syntax_set: &'a SyntaxSet, token: &str) -> Option<&'a SyntaxReference> {
    let lowercase = token.to_ascii_lowercase();
    syntax_set
        .find_syntax_by_token(&lowercase)
        .or_else(|| syntax_set.find_syntax_by_name(&lowercase))
        .or_else(|| syntax_set.find_syntax_by_extension(&lowercase))
}
