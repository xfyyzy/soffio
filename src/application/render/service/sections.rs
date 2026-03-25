use url::Url;

use crate::application::render::types::{
    ContentMetrics, RenderError, RenderTarget, RenderedSection, ResourceHints,
};

use super::rewrite::HeadingInfo;

#[path = "sections/outline.rs"]
mod outline;
#[path = "sections/semantics.rs"]
mod semantics;

use outline::{apply_heading_ids, build_sections};
use semantics::{
    AugmentOutcome, augment_code_blocks_only, augment_semantics, build_content_metrics,
    build_resource_hints,
};

pub(crate) struct ProcessedHtml {
    pub(crate) html: String,
    pub(crate) sections: Option<Vec<RenderedSection>>,
    pub(crate) contains_code: bool,
    pub(crate) contains_math: bool,
    pub(crate) contains_mermaid: bool,
    pub(crate) resource_hints: ResourceHints,
    pub(crate) content_metrics: ContentMetrics,
}

pub(crate) fn post_process(
    sanitized_html: &str,
    target: &RenderTarget,
    headings: &[HeadingInfo],
    public_site_url: Option<&str>,
) -> Result<ProcessedHtml, RenderError> {
    match target {
        RenderTarget::PostBody { .. } => {
            process_post_html(sanitized_html, headings, public_site_url)
        }
        _ => {
            let html = augment_code_blocks_only(sanitized_html)?;
            let contains_code =
                html.contains("syntax-") || html.contains("<pre") || html.contains("<code");
            let contains_math = html.contains("data-math-style");
            let contains_mermaid = html.contains("data-role=\"diagram-mermaid\"");
            Ok(ProcessedHtml {
                html,
                sections: None,
                contains_code,
                contains_math,
                contains_mermaid,
                resource_hints: ResourceHints::default(),
                content_metrics: ContentMetrics::default(),
            })
        }
    }
}

fn process_post_html(
    sanitized_html: &str,
    headings: &[HeadingInfo],
    public_site_url: Option<&str>,
) -> Result<ProcessedHtml, RenderError> {
    let site_url = public_site_url.and_then(|value| Url::parse(value).ok());

    if headings.is_empty() {
        let augmentation = augment_semantics(sanitized_html, site_url.as_ref())?;
        let metrics = build_content_metrics(&augmentation);
        let resource_hints = build_resource_hints(&augmentation);
        let contains_code = metrics.code_blocks_count > 0;
        let contains_math = metrics.math_blocks_count > 0;
        let contains_mermaid = metrics.mermaid_diagram_count > 0;
        let AugmentOutcome {
            html: augmented_html,
            ..
        } = augmentation;

        return Ok(ProcessedHtml {
            html: augmented_html,
            sections: Some(Vec::new()),
            contains_code,
            contains_math,
            contains_mermaid,
            resource_hints,
            content_metrics: metrics,
        });
    }

    let html_with_ids = apply_heading_ids(sanitized_html, headings)?;
    let augmentation = augment_semantics(&html_with_ids, site_url.as_ref())?;
    let sections = build_sections(&augmentation.html, headings)?;
    let metrics = build_content_metrics(&augmentation);
    let resource_hints = build_resource_hints(&augmentation);
    let contains_code =
        sections.iter().any(|section| section.contains_code) || metrics.code_blocks_count > 0;
    let contains_math =
        sections.iter().any(|section| section.contains_math) || metrics.math_blocks_count > 0;
    let contains_mermaid = sections.iter().any(|section| section.contains_mermaid)
        || metrics.mermaid_diagram_count > 0;
    let AugmentOutcome {
        html: augmented_html,
        ..
    } = augmentation;

    Ok(ProcessedHtml {
        html: augmented_html,
        sections: Some(sections),
        contains_code,
        contains_math,
        contains_mermaid,
        resource_hints,
        content_metrics: metrics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(url: &str) -> Option<Url> {
        Url::parse(url).ok()
    }

    #[test]
    fn absolute_same_origin_counts_as_internal() {
        let html = "<p><a href=\"https://example.com/path\">link</a></p>";
        let outcome =
            augment_semantics(html, site("https://example.com/").as_ref()).expect("augment");

        assert!(outcome.html.contains("data-link-kind=\"internal\""));
        assert!(!outcome.html.contains("target=\"_blank\""));

        let metrics = build_content_metrics(&outcome);
        assert_eq!(metrics.internal_links_count, 1);
        assert_eq!(metrics.external_links_count, 0);
    }

    #[test]
    fn external_links_open_in_new_tab_with_rel() {
        let html = "<p><a href=\"https://other.com/page\">out</a></p>";
        let outcome =
            augment_semantics(html, site("https://example.com/").as_ref()).expect("augment");

        assert!(outcome.html.contains("data-link-kind=\"external\""));
        assert!(outcome.html.contains("target=\"_blank\""));
        assert!(outcome.html.contains("rel=\"noopener noreferrer\""));

        let metrics = build_content_metrics(&outcome);
        assert_eq!(metrics.external_links_count, 1);
        assert_eq!(metrics.internal_links_count, 0);
    }

    #[test]
    fn augment_semantics_injects_code_copy_button_for_highlighted_pre() {
        let html = "<pre class=\"syntax-highlight syntax-lang-rust\" data-language=\"rust\"><code class=\"language-rust syntax-code\">fn main() {}\n</code></pre>";
        let outcome = augment_semantics(html, None).expect("augment");

        assert!(outcome.html.contains("data-role=\"code-copy-button\""));
        assert!(outcome.html.contains("@copyCodeBlockText()"));
        assert!(outcome.html.contains("data-copy-label-success=\"Copied\""));
    }

    #[test]
    fn page_target_post_process_also_injects_copy_button() {
        let html = "<pre class=\"syntax-highlight syntax-lang-rust\" data-language=\"rust\"><code class=\"language-rust syntax-code\">fn main() {}\n</code></pre>";
        let output = post_process(
            html,
            &RenderTarget::PageBody {
                slug: "about".to_string(),
            },
            &[],
            None,
        )
        .expect("post process");

        assert!(output.html.contains("data-role=\"code-copy-button\""));
        assert!(output.html.contains("@copyCodeBlockText()"));
    }
}
