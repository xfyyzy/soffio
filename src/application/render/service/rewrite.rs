use std::vec::Vec;

use comrak::nodes::{AstNode, NodeHtmlBlock, NodeValue};
use syntect::html::ClassStyle;
use syntect::parsing::SyntaxSet;
use tracing::warn;

use crate::{application::render::types::RenderError, domain::slug::AnchorSlugger};

use super::{highlight, math, mermaid::MermaidRenderer};

#[path = "rewrite/media.rs"]
mod media;
#[path = "rewrite/utils.rs"]
mod utils;

#[cfg(test)]
#[path = "rewrite/tests.rs"]
mod tests;

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
    let mut walker = RewriteWalker::new(syntax_set, class_style, mermaid, slug);
    walker.visit_nodes(root)?;
    Ok(walker.outcome)
}

struct RewriteWalker<'a> {
    syntax_set: &'a SyntaxSet,
    class_style: &'a ClassStyle,
    outcome: RewriteOutcome,
    slugger: AnchorSlugger,
    heading_stack: Vec<usize>,
    mermaid: Option<&'a MermaidRenderer>,
    slug: &'a str,
}

impl<'a> RewriteWalker<'a> {
    fn new(
        syntax_set: &'a SyntaxSet,
        class_style: &'a ClassStyle,
        mermaid: Option<&'a MermaidRenderer>,
        slug: &'a str,
    ) -> Self {
        Self {
            syntax_set,
            class_style,
            outcome: RewriteOutcome::default(),
            slugger: AnchorSlugger::new(),
            heading_stack: Vec::new(),
            mermaid,
            slug,
        }
    }

    fn visit_nodes(&mut self, node: &AstNode<'_>) -> Result<(), RenderError> {
        if {
            let data = node.data.borrow();
            matches!(data.value, NodeValue::Image(_))
        } {
            media::process_image_node(node)?;
        }

        if let Some(level) = utils::heading_level(node) {
            let text = utils::collect_heading_text(node);
            let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
            let slug = self.slugger.anchor_for(normalized.trim()).map_err(|err| {
                RenderError::Anchoring {
                    message: err.to_string(),
                }
            })?;
            while let Some(&idx) = self.heading_stack.last() {
                if self.outcome.headings[idx].level < level {
                    break;
                }
                self.heading_stack.pop();
            }
            self.outcome.headings.push(HeadingInfo {
                level,
                slug,
                text: normalized.trim().to_string(),
                has_block_code: false,
                has_math_block: false,
                has_inline_math: false,
                has_mermaid_block: false,
            });
            self.heading_stack.push(self.outcome.headings.len() - 1);
        }

        if self.handle_math_node(node)? {
            // Math nodes are fully handled; skip further processing.
        } else if let Some((info, literal)) = utils::extract_code_block(node) {
            let mut segments = info.split_whitespace();
            let language_owned = segments.next().map(|s| s.to_string());
            let meta_string = segments.collect::<Vec<_>>().join(" ");
            let language_ref = language_owned.as_deref();

            if self.handle_mermaid_block(node, language_ref, &literal)? {
                // Mermaid block handled (successfully rendered or gracefully degraded).
                // Skip syntax highlighting path.
            } else {
                let meta_ref = (!meta_string.is_empty()).then_some(meta_string.as_str());
                let html = highlight::highlight_code(
                    language_ref,
                    meta_ref,
                    &literal,
                    self.syntax_set,
                    self.class_style,
                )?;
                self.outcome.contains_code = true;
                if let Some(&idx) = self.heading_stack.last() {
                    self.outcome.headings[idx].has_block_code = true;
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
            self.visit_nodes(next)?;
            child = next.next_sibling();
        }

        Ok(())
    }

    fn handle_math_node(&mut self, node: &AstNode<'_>) -> Result<bool, RenderError> {
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
                    format!(
                        "<div data-role=\"math-block\" data-math-style=\"display\">{html}</div>"
                    )
                } else {
                    format!(
                        "<span data-role=\"math-inline\" data-math-style=\"inline\">{html}</span>"
                    )
                };

                let placeholder = format!("__KATEX_PLACEHOLDER_{}__", self.outcome.math_counter);
                self.outcome.math_counter = self.outcome.math_counter.saturating_add(1);
                self.outcome.math_fragments.push(MathFragment {
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

                self.outcome.contains_math = true;
                if let Some(&idx) = self.heading_stack.last() {
                    if display_mode {
                        self.outcome.headings[idx].has_math_block = true;
                    } else {
                        self.outcome.headings[idx].has_inline_math = true;
                    }
                }
                Ok(true)
            }
            Err(err) => {
                warn!(
                    target = "application::render::math",
                    slug = self.slug,
                    "KaTeX rendering failed: {err}"
                );
                self.apply_math_fallback(node, literal.as_str(), display_mode)?;
                Ok(true)
            }
        }
    }

    fn apply_math_fallback(
        &mut self,
        node: &AstNode<'_>,
        literal: &str,
        display_mode: bool,
    ) -> Result<(), RenderError> {
        self.outcome.contains_code = true;

        if display_mode {
            let highlighted = highlight::highlight_code(
                Some("math"),
                None,
                literal,
                self.syntax_set,
                self.class_style,
            )
            .unwrap_or_else(|_| utils::build_plain_code_block("math", literal));

            let mut data = node.data.borrow_mut();
            data.value = NodeValue::HtmlBlock(NodeHtmlBlock {
                block_type: 0,
                literal: highlighted,
            });

            if let Some(&idx) = self.heading_stack.last() {
                self.outcome.headings[idx].has_block_code = true;
                self.outcome.headings[idx].has_math_block = true;
            }
        } else {
            let escaped = ammonia::clean_text(literal);
            let fallback = format!("<code data-math-style=\"inline\">{escaped}</code>",);
            let mut data = node.data.borrow_mut();
            data.value = NodeValue::HtmlInline(fallback);
            if let Some(&idx) = self.heading_stack.last() {
                self.outcome.headings[idx].has_inline_math = true;
            }
        }

        Ok(())
    }

    fn handle_mermaid_block(
        &mut self,
        node: &AstNode<'_>,
        language: Option<&str>,
        literal: &str,
    ) -> Result<bool, RenderError> {
        let Some(lang) = language.map(|lang| lang.to_ascii_lowercase()) else {
            return Ok(false);
        };

        if !matches!(lang.as_str(), "mermaid" | "mermind") {
            return Ok(false);
        }

        let Some(renderer) = self.mermaid else {
            warn!(
                target = "application::render::mermaid",
                slug = self.slug,
                "Mermaid renderer unavailable; falling back to code block"
            );
            self.apply_mermaid_fallback(node, language, literal)?;
            return Ok(true);
        };

        match renderer.render_svg(literal) {
            Ok(svg) => {
                let fragment = format!("<figure data-role=\"diagram-mermaid\">{svg}</figure>");
                let placeholder_key =
                    format!("__MERMAID_PLACEHOLDER_{}__", self.outcome.mermaid_counter);
                self.outcome.mermaid_counter = self.outcome.mermaid_counter.saturating_add(1);
                self.outcome.mermaid_fragments.push(MermaidFragment {
                    placeholder: placeholder_key.clone(),
                    html: fragment,
                });
                let placeholder_html = format!("<div>{placeholder_key}</div>");

                let mut data = node.data.borrow_mut();
                data.value = NodeValue::HtmlBlock(NodeHtmlBlock {
                    block_type: 0,
                    literal: placeholder_html,
                });
                self.outcome.contains_mermaid = true;
                if let Some(&idx) = self.heading_stack.last() {
                    self.outcome.headings[idx].has_mermaid_block = true;
                }
                Ok(true)
            }
            Err(err) => {
                warn!(
                    target = "application::render::mermaid",
                    slug = self.slug,
                    "Mermaid CLI failed: {err}"
                );
                self.apply_mermaid_fallback(node, language, literal)?;
                Ok(true)
            }
        }
    }

    fn apply_mermaid_fallback(
        &mut self,
        node: &AstNode<'_>,
        language: Option<&str>,
        literal: &str,
    ) -> Result<(), RenderError> {
        let highlighted =
            highlight::highlight_code(language, None, literal, self.syntax_set, self.class_style)
                .unwrap_or_else(|_| {
                    utils::build_plain_code_block(language.unwrap_or("text"), literal)
                });

        self.outcome.contains_code = true;
        if let Some(&idx) = self.heading_stack.last() {
            self.outcome.headings[idx].has_block_code = true;
        }

        let mut data = node.data.borrow_mut();
        data.value = NodeValue::HtmlBlock(NodeHtmlBlock {
            block_type: 0,
            literal: highlighted,
        });

        Ok(())
    }
}
