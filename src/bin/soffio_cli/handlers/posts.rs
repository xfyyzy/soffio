#![deny(clippy::all, clippy::pedantic)]

use std::path::PathBuf;

use reqwest::Method;
use soffio::domain::types::PostStatus;
use soffio::infra::http::api::models::{
    PostBodyRequest, PostCreateRequest, PostExcerptRequest, PostPinRequest, PostStatusRequest,
    PostSummaryRequest, PostTagsRequest, PostTitleSlugRequest, PostUpdateRequest,
};
use uuid::Uuid;

use crate::args::{PostStatusArg, PostsCmd};
use crate::client::{CliError, Ctx};
use crate::io::{parse_time_opt, read_opt_value, read_value, to_value};
use crate::print::print_json;

pub async fn handle(ctx: &Ctx, cmd: PostsCmd) -> Result<(), CliError> {
    match cmd {
        PostsCmd::List {
            status,
            tag,
            search,
            month,
            limit,
            cursor,
        } => list(ctx, status, tag, search, month, limit, cursor).await,
        PostsCmd::Get { slug } => get(ctx, slug).await,
        PostsCmd::Create {
            title,
            excerpt,
            body,
            body_file,
            summary,
            summary_file,
            status,
            pinned,
            scheduled_at,
            published_at,
            archived_at,
        } => {
            let input = PostCreateInput {
                title,
                excerpt,
                body,
                body_file,
                summary,
                summary_file,
                status,
                pinned,
                scheduled_at,
                published_at,
                archived_at,
            };
            create(ctx, input).await
        }
        PostsCmd::Update {
            id,
            slug,
            title,
            excerpt,
            body,
            body_file,
            summary,
            summary_file,
            pinned,
        } => {
            let input = PostUpdateInput {
                id,
                slug,
                title,
                excerpt,
                body,
                body_file,
                summary,
                summary_file,
                pinned,
            };
            update(ctx, input).await
        }
        PostsCmd::PatchTitleSlug { id, title, slug } => {
            patch_title_slug(ctx, id, title, slug).await
        }
        PostsCmd::PatchExcerpt { id, excerpt } => patch_excerpt(ctx, id, excerpt).await,
        PostsCmd::PatchBody {
            id,
            body,
            body_file,
        } => patch_body(ctx, id, body, body_file).await,
        PostsCmd::PatchSummary {
            id,
            summary,
            summary_file,
        } => patch_summary(ctx, id, summary, summary_file).await,
        PostsCmd::Status {
            id,
            status,
            scheduled_at,
            published_at,
            archived_at,
        } => update_status(ctx, id, status, scheduled_at, published_at, archived_at).await,
        PostsCmd::Tags { id, tag_ids } => replace_tags(ctx, id, tag_ids).await,
        PostsCmd::Pin { id, pinned } => pin(ctx, id, pinned).await,
        PostsCmd::Delete { id } => delete(ctx, id).await,
    }
}

struct PostCreateInput {
    title: String,
    excerpt: String,
    body: Option<String>,
    body_file: Option<PathBuf>,
    summary: Option<String>,
    summary_file: Option<PathBuf>,
    status: PostStatusArg,
    pinned: bool,
    scheduled_at: Option<String>,
    published_at: Option<String>,
    archived_at: Option<String>,
}

struct PostUpdateInput {
    id: Uuid,
    slug: String,
    title: String,
    excerpt: String,
    body: Option<String>,
    body_file: Option<PathBuf>,
    summary: Option<String>,
    summary_file: Option<PathBuf>,
    pinned: bool,
}

async fn list(
    ctx: &Ctx,
    status: Option<PostStatusArg>,
    tag: Option<String>,
    search: Option<String>,
    month: Option<String>,
    limit: u32,
    cursor: Option<String>,
) -> Result<(), CliError> {
    let mut q = vec![("limit", limit.to_string())];
    if let Some(s) = status {
        q.push(("status", s.as_str().to_string()));
    }
    if let Some(t) = tag {
        q.push(("tag", t));
    }
    if let Some(s) = search {
        q.push(("search", s));
    }
    if let Some(m) = month {
        q.push(("month", m));
    }
    if let Some(c) = cursor {
        q.push(("cursor", c));
    }
    let res: serde_json::Value = ctx
        .request(Method::GET, "api/v1/posts", Some(&q), None)
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn get(ctx: &Ctx, slug: String) -> Result<(), CliError> {
    let path = format!("api/v1/posts/slug/{slug}");
    let res: serde_json::Value = ctx.request(Method::GET, &path, None, None).await?;
    print_json(&res)?;
    Ok(())
}

async fn create(ctx: &Ctx, input: PostCreateInput) -> Result<(), CliError> {
    let PostCreateInput {
        title,
        excerpt,
        body,
        body_file,
        summary,
        summary_file,
        status,
        pinned,
        scheduled_at,
        published_at,
        archived_at,
    } = input;

    let body_markdown = read_value(body, body_file)?;
    let summary_markdown = read_opt_value(summary, summary_file)?;
    let payload = PostCreateRequest {
        title,
        excerpt,
        body_markdown,
        summary_markdown,
        status: status.into(),
        pinned,
        scheduled_at: parse_time_opt(scheduled_at)?,
        published_at: parse_time_opt(published_at)?,
        archived_at: parse_time_opt(archived_at)?,
    };
    let res: serde_json::Value = ctx
        .request(Method::POST, "api/v1/posts", None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn update(ctx: &Ctx, input: PostUpdateInput) -> Result<(), CliError> {
    let PostUpdateInput {
        id,
        slug,
        title,
        excerpt,
        body,
        body_file,
        summary,
        summary_file,
        pinned,
    } = input;

    let body_markdown = read_value(body, body_file)?;
    let summary_markdown = read_opt_value(summary, summary_file)?;
    let payload = PostUpdateRequest {
        slug,
        title,
        excerpt,
        body_markdown,
        summary_markdown,
        pinned,
    };
    let path = format!("api/v1/posts/{id}");
    let res: serde_json::Value = ctx
        .request(Method::PATCH, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_title_slug(
    ctx: &Ctx,
    id: Uuid,
    title: Option<String>,
    slug: Option<String>,
) -> Result<(), CliError> {
    let payload = PostTitleSlugRequest { title, slug };
    let path = format!("api/v1/posts/{id}/title-slug");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_excerpt(ctx: &Ctx, id: Uuid, excerpt: String) -> Result<(), CliError> {
    let payload = PostExcerptRequest { excerpt };
    let path = format!("api/v1/posts/{id}/excerpt");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_body(
    ctx: &Ctx,
    id: Uuid,
    body: Option<String>,
    body_file: Option<PathBuf>,
) -> Result<(), CliError> {
    let body_markdown = read_value(body, body_file)?;
    let payload = PostBodyRequest { body_markdown };
    let path = format!("api/v1/posts/{id}/body");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_summary(
    ctx: &Ctx,
    id: Uuid,
    summary: Option<String>,
    summary_file: Option<PathBuf>,
) -> Result<(), CliError> {
    let summary_markdown = read_opt_value(summary, summary_file)?;
    let payload = PostSummaryRequest { summary_markdown };
    let path = format!("api/v1/posts/{id}/summary");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn update_status(
    ctx: &Ctx,
    id: Uuid,
    status: PostStatusArg,
    scheduled_at: Option<String>,
    published_at: Option<String>,
    archived_at: Option<String>,
) -> Result<(), CliError> {
    let payload = PostStatusRequest {
        status: status.into(),
        scheduled_at: parse_time_opt(scheduled_at)?,
        published_at: parse_time_opt(published_at)?,
        archived_at: parse_time_opt(archived_at)?,
    };
    let path = format!("api/v1/posts/{id}/status");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn replace_tags(ctx: &Ctx, id: Uuid, tag_ids: String) -> Result<(), CliError> {
    let ids: Vec<Uuid> = tag_ids
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| Uuid::parse_str(s.trim()))
        .collect::<Result<_, _>>()
        .map_err(|e| CliError::InvalidInput(e.to_string()))?;
    let payload = PostTagsRequest { tag_ids: ids };
    let path = format!("api/v1/posts/{id}/tags");
    ctx.request::<serde_json::Value>(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    println!("tags replaced");
    Ok(())
}

async fn pin(ctx: &Ctx, id: Uuid, pinned: bool) -> Result<(), CliError> {
    let payload = PostPinRequest { pinned };
    let path = format!("api/v1/posts/{id}/pin");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn delete(ctx: &Ctx, id: Uuid) -> Result<(), CliError> {
    let path = format!("api/v1/posts/{id}");
    ctx.request_no_body(Method::DELETE, &path, None).await?;
    println!("deleted");
    Ok(())
}

impl From<PostStatusArg> for PostStatus {
    fn from(value: PostStatusArg) -> Self {
        match value {
            PostStatusArg::Draft => PostStatus::Draft,
            PostStatusArg::Published => PostStatus::Published,
            PostStatusArg::Archived => PostStatus::Archived,
            PostStatusArg::Error => PostStatus::Error,
        }
    }
}
