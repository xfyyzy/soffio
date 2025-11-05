use super::util::*;
use askama::Template;
use soffio::presentation::views::{
    ErrorPageView, ErrorTemplate, IndexTemplate, PageTemplate, PostTemplate, PostsPartial,
};

#[tokio::test]
async fn snapshot_partial_initial_feed() {
    let feed = feed_service();
    let context = feed
        .page_context(FeedFilter::All, None)
        .await
        .expect("page context");
    let html = PostsPartial { content: context }
        .render()
        .expect("render partial");
    insta::assert_snapshot!("partial_initial_feed", html);
}

#[tokio::test]
async fn snapshot_partial_cursor_feed() {
    let feed = feed_service();
    let page = feed
        .page_context(FeedFilter::All, None)
        .await
        .expect("page context");
    let cursor = page
        .next_cursor
        .expect("expected next cursor for first page");
    let context = feed
        .page_context(FeedFilter::All, Some(&cursor))
        .await
        .expect("cursor page context");
    let html = PostsPartial { content: context }
        .render()
        .expect("render cursor partial");
    insta::assert_snapshot!("partial_cursor_feed", html);
}

#[tokio::test]
async fn snapshot_partial_tag_feed() {
    let feed = feed_service();
    let context = feed
        .page_context(FeedFilter::Tag("community".to_string()), None)
        .await
        .expect("tag page context");
    let html = PostsPartial { content: context }
        .render()
        .expect("render tag partial");
    insta::assert_snapshot!("partial_tag_education", html);
}

#[tokio::test]
async fn snapshot_partial_month_feed() {
    let feed = feed_service();
    let months = posts::compute_month_counts();
    let first_month = months.first().expect("at least one month").key.clone();
    let context = feed
        .page_context(FeedFilter::Month(first_month), None)
        .await
        .expect("month page context");
    let html = PostsPartial { content: context }
        .render()
        .expect("render month partial");
    insta::assert_snapshot!("partial_month_feed", html);
}

#[tokio::test]
async fn snapshot_index_page() {
    let feed = feed_service();
    let context = feed
        .page_context(FeedFilter::All, None)
        .await
        .expect("page context");
    let view = apply_layout(context).await;
    let html = IndexTemplate { view }.render().expect("render index");
    insta::assert_snapshot!("page_index", html);
}

#[tokio::test]
async fn snapshot_post_detail() {
    let feed = feed_service();
    let detail = feed
        .post_detail("incremental-build-pipeline")
        .await
        .expect("detail fetch")
        .expect("detail exists");
    let view = apply_layout(detail).await;
    let html = PostTemplate { view }.render().expect("render post detail");
    insta::assert_snapshot!("post_incremental_build_pipeline", html);
}

#[tokio::test]
async fn snapshot_error_page() {
    let view = apply_layout(ErrorPageView::not_found()).await;
    let html = ErrorTemplate { view }.render().expect("render error page");
    insta::assert_snapshot!("page_error_not_found", html);
}

#[tokio::test]
async fn snapshot_post_cards_append() {
    let feed = feed_service();
    let payload = feed
        .append_payload(FeedFilter::All, None)
        .await
        .expect("append payload");
    let response = feed_render::build_datastar_append_response(payload, String::new())
        .expect("datastar response");
    let body = body_to_string(response.into_body()).await;
    insta::assert_snapshot!("sse_post_cards_append", body);
}

#[tokio::test]
async fn snapshot_about_page() {
    let page = page_service()
        .page_view("about")
        .await
        .expect("about page fetch")
        .expect("about page present");
    let view = apply_layout(page).await;
    let html = PageTemplate { view }.render().expect("render about page");
    insta::assert_snapshot!("page_about", html);
}
