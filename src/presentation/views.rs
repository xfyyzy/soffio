use crate::application::error::{ErrorReport, HttpError};
use askama::{Error as AskamaError, Template};
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{public_message}")]
pub struct TemplateRenderError {
    pub(crate) source: &'static str,
    pub(crate) public_message: &'static str,
    #[source]
    pub(crate) error: AskamaError,
}

impl TemplateRenderError {
    pub fn new(source: &'static str, public_message: &'static str, error: AskamaError) -> Self {
        Self {
            source,
            public_message,
            error,
        }
    }
}

impl From<TemplateRenderError> for HttpError {
    fn from(err: TemplateRenderError) -> Self {
        let TemplateRenderError {
            source,
            public_message,
            error,
        } = err;

        HttpError::from_error(
            source,
            StatusCode::INTERNAL_SERVER_ERROR,
            public_message,
            &error,
        )
    }
}

pub fn render_template<T: Template>(template: T) -> Result<Html<String>, HttpError> {
    template.render().map(Html).map_err(|err| {
        TemplateRenderError::new(
            "presentation::views::render_template",
            "Template rendering failed",
            err,
        )
        .into()
    })
}

pub fn render_template_response<T: Template>(template: T, status: StatusCode) -> Response {
    match render_template(template) {
        Ok(html) => (status, html).into_response(),
        Err(err) => err.into_response(),
    }
}

pub fn render_not_found_response(chrome: LayoutChrome) -> Response {
    let content = ErrorPageView::not_found();
    let view = LayoutContext::new(chrome, content);
    let mut response = render_template_response(ErrorTemplate { view }, StatusCode::NOT_FOUND);
    ErrorReport::from_message(
        "presentation::views::render_not_found_response",
        StatusCode::NOT_FOUND,
        "Resource not found",
    )
    .attach(&mut response);
    response
}

#[derive(Clone)]
pub struct NavigationView {
    pub entries: Vec<NavigationLinkView>,
}

#[derive(Clone)]
pub struct FooterView {
    pub copy: String,
}

#[derive(Clone)]
pub struct BrandView {
    pub title: String,
    pub href: String,
}

#[derive(Clone)]
pub struct NavigationLinkView {
    pub label: String,
    pub href: String,
    pub target: Option<String>,
    pub rel: Option<String>,
}

#[derive(Clone)]
pub struct LayoutChrome {
    pub brand: BrandView,
    pub navigation: NavigationView,
    pub footer: FooterView,
    pub meta: PageMetaView,
}

impl LayoutChrome {
    pub fn with_canonical(self, canonical: String) -> Self {
        Self {
            meta: self.meta.with_canonical(canonical),
            ..self
        }
    }
}

#[derive(Clone)]
pub struct LayoutContext<T> {
    pub brand: BrandView,
    pub navigation: NavigationView,
    pub footer: FooterView,
    pub meta: PageMetaView,
    pub content: T,
}

impl<T> LayoutContext<T> {
    pub fn new(chrome: LayoutChrome, content: T) -> Self {
        Self {
            brand: chrome.brand,
            navigation: chrome.navigation,
            footer: chrome.footer,
            meta: chrome.meta,
            content,
        }
    }
}

#[derive(Clone)]
pub struct TagBadge {
    pub value: String,
    pub label: String,
}

#[derive(Clone)]
pub struct PostCard {
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub iso_date: String,
    pub published: String,
    pub badges: Vec<TagBadge>,
    pub is_pinned: bool,
}

#[derive(Clone)]
pub struct TagSummary {
    pub label: String,
    pub path: String,
    pub count: usize,
    pub is_active: bool,
}

#[derive(Clone)]
pub struct MonthSummary {
    pub label: String,
    pub path: String,
    pub count: usize,
    pub is_active: bool,
}

pub struct PageContext {
    pub posts: Vec<PostCard>,
    pub post_count: usize,
    pub total_count: usize,
    pub has_results: bool,
    pub tags: Vec<TagSummary>,
    pub months: Vec<MonthSummary>,
    pub show_tag_filters: bool,
    pub show_month_filters: bool,
    pub next_cursor: Option<String>,
    pub load_more_query: String,
    pub posts_ld_json: Option<String>,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub view: LayoutContext<PageContext>,
}

#[derive(Template)]
#[template(path = "partials/content.html")]
pub struct PostsPartial {
    pub content: PageContext,
}

pub struct FeedLoaderContext {
    pub has_results: bool,
    pub next_cursor: Option<String>,
    pub load_more_query: String,
}

#[derive(Template)]
#[template(path = "partials/feed_loader.html")]
pub struct FeedLoaderTemplate {
    pub view: FeedLoaderContext,
}

#[derive(Template)]
#[template(path = "partials/post_cards_append.html")]
pub struct PostCardsAppendTemplate {
    pub posts: Vec<PostCard>,
    pub offset: usize,
}

pub struct PostDetailContext {
    pub slug: String,
    pub title: String,
    pub published: String,
    pub iso_date: String,
    pub tags: Vec<TagBadge>,
    pub excerpt: String,
    pub summary_html: Option<String>,
    pub sections: Vec<PostSectionEvent>,
    pub has_code_blocks: bool,
    pub has_math_blocks: bool,
    pub has_mermaid_diagrams: bool,
    pub toc: Option<PostTocView>,
    pub is_pinned: bool,
}

#[derive(Clone)]
pub struct PostTocView {
    pub events: Vec<PostTocEvent>,
}

#[derive(Clone)]
pub enum PostTocEvent {
    StartList,
    EndList,
    StartItem {
        anchor: String,
        title: String,
        level: u8,
    },
    EndItem,
}

#[derive(Clone)]
pub enum PostSectionEvent {
    StartSection {
        anchor: String,
        level: u8,
        heading_html: String,
        body_html: String,
    },
    StartChildren,
    EndChildren,
    EndSection,
}

#[derive(Template)]
#[template(path = "post.html")]
pub struct PostTemplate {
    pub view: LayoutContext<PostDetailContext>,
}

pub struct PageView {
    pub content_html: String,
    pub contains_code: bool,
    pub contains_math: bool,
    pub contains_mermaid: bool,
}

#[derive(Template)]
#[template(path = "page.html")]
pub struct PageTemplate {
    pub view: LayoutContext<PageView>,
}

pub struct ErrorPageView {
    pub title: String,
    pub message: String,
    pub primary_action: Option<ErrorAction>,
    pub contains_code: bool,
    pub contains_math: bool,
    pub contains_mermaid: bool,
}

impl ErrorPageView {
    pub fn not_found() -> Self {
        Self {
            title: "Page Not Found".to_string(),
            message: "The page you requested does not exist. Try returning to the homepage to continue exploring.".to_string(),
            primary_action: Some(ErrorAction::home()),
            contains_code: false,
            contains_math: false,
            contains_mermaid: false,
        }
    }
}

pub struct ErrorAction {
    pub href: String,
    pub label: String,
}

impl ErrorAction {
    pub fn home() -> Self {
        Self {
            href: "/".to_string(),
            label: "Back to home".to_string(),
        }
    }
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub view: LayoutContext<ErrorPageView>,
}

#[derive(Clone)]
pub struct PageMetaView {
    pub title: String,
    pub description: String,
    pub og_title: String,
    pub og_description: String,
    pub canonical: String,
}

impl PageMetaView {
    pub fn with_canonical(self, canonical: String) -> Self {
        Self { canonical, ..self }
    }
}

pub fn build_tag_badges<'a, T>(tags: T) -> Vec<TagBadge>
where
    T: IntoIterator<Item = (&'a str, &'a str)>,
{
    tags.into_iter()
        .map(|(value, name)| TagBadge {
            value: value.to_string(),
            label: format!("#{}", name),
        })
        .collect()
}

pub fn title_case(tag: &str) -> String {
    if tag.eq_ignore_ascii_case("ai") {
        return "AI".to_string();
    }

    let mut words = Vec::new();
    for segment in tag.split(['-', '_']) {
        if segment.is_empty() {
            continue;
        }
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            let mut word = String::new();
            word.extend(first.to_uppercase());
            for ch in chars {
                word.extend(ch.to_lowercase());
            }
            words.push(word);
        }
    }

    if words.is_empty() {
        tag.to_string()
    } else {
        words.join(" ")
    }
}
