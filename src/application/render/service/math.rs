use katex::{OptsBuilder, OutputType};

use crate::application::render::types::RenderError;

/// Render a KaTeX expression to HTML, returning an inline (`<span>`) or block (`<div>`) fragment.
pub(crate) fn render_math_html(literal: &str, display_mode: bool) -> Result<String, RenderError> {
    let mut builder = OptsBuilder::default();
    builder.display_mode(display_mode);
    builder.output_type(OutputType::Html);

    let opts = builder.build().map_err(|err| RenderError::Document {
        message: format!("failed to build KaTeX options: {err}"),
    })?;

    katex::render_with_opts(literal, opts).map_err(|err| RenderError::Document {
        message: format!("KaTeX rendering failed: {err}"),
    })
}
