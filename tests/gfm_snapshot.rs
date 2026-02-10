use soffio::application::render::{RenderRequest, RenderService, RenderTarget, render_service};

fn load_markdown() -> String {
    include_str!("fixtures/gfm_features.md").to_string()
}

#[test]
fn gfm_fixture_raw_snapshot_matches() {
    let renderer = render_service();
    let markdown = load_markdown();
    let request = RenderRequest::new(
        RenderTarget::PostBody {
            slug: "gfm-fixture".into(),
        },
        markdown,
    );

    let html = renderer
        .render_unsanitized(&request)
        .expect("unsanitized render succeeds");

    let expected = include_str!("fixtures/gfm_post_raw.html");
    assert_eq!(expected.trim_end(), html.trim_end());
}

#[test]
fn gfm_fixture_sanitized_snapshot_matches() {
    let renderer = render_service();
    let markdown = load_markdown();
    let request = RenderRequest::new(
        RenderTarget::PostBody {
            slug: "gfm-fixture".into(),
        },
        markdown,
    );

    let output = renderer
        .render(&request)
        .expect("sanitized render succeeds");

    let expected = include_str!("fixtures/gfm_post_sanitized.html");
    assert_eq!(expected.trim_end(), output.html.trim_end());
}

#[test]
fn gfm_fixture_code_block_has_copy_button() {
    let renderer = render_service();
    let markdown = load_markdown();
    let request = RenderRequest::new(
        RenderTarget::PostBody {
            slug: "gfm-fixture".into(),
        },
        markdown,
    );

    let output = renderer
        .render(&request)
        .expect("sanitized render succeeds");

    assert!(
        output.html.contains("data-role=\"code-copy-button\""),
        "rendered HTML should include a code copy button for public code blocks"
    );
    assert!(
        output.html.contains("@copyCodeBlockText()"),
        "code copy button should bind to dedicated public copy action"
    );
}
