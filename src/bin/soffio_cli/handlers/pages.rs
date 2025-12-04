#![deny(clippy::all, clippy::pedantic)]

use std::path::PathBuf;

use reqwest::Method;
use soffio::domain::types::PageStatus;
use soffio::infra::http::api::models::{
    PageBodyRequest, PageCreateRequest, PageStatusRequest, PageTitleRequest, PageUpdateRequest,
};
use uuid::Uuid;

use crate::args::{PageStatusArg, PagesCmd};
use crate::client::{CliError, Ctx};
use crate::io::{parse_time_opt, read_value, to_value};
use crate::print::print_json;

struct PageCreateInput {
    slug: Option<String>,
    title: String,
    body: Option<String>,
    body_file: Option<PathBuf>,
    status: PageStatusArg,
    scheduled_at: Option<String>,
    published_at: Option<String>,
    archived_at: Option<String>,
}

pub async fn handle(ctx: &Ctx, cmd: PagesCmd) -> Result<(), CliError> {
    match cmd {
        PagesCmd::List {
            status,
            search,
            month,
            limit,
            cursor,
        } => list(ctx, status, search, month, limit, cursor).await,
        PagesCmd::Get { id, slug } => get(ctx, id, slug).await,
        PagesCmd::Create {
            slug,
            title,
            body,
            body_file,
            status,
            scheduled_at,
            published_at,
            archived_at,
        } => {
            let input = PageCreateInput {
                slug,
                title,
                body,
                body_file,
                status,
                scheduled_at,
                published_at,
                archived_at,
            };
            create(ctx, input).await
        }
        PagesCmd::Update {
            id,
            slug,
            title,
            body,
            body_file,
        } => update(ctx, id, slug, title, body, body_file).await,
        PagesCmd::PatchTitle { id, title } => patch_title(ctx, id, title).await,
        PagesCmd::PatchBody {
            id,
            body,
            body_file,
        } => patch_body(ctx, id, body, body_file).await,
        PagesCmd::Status {
            id,
            status,
            scheduled_at,
            published_at,
            archived_at,
        } => update_status(ctx, id, status, scheduled_at, published_at, archived_at).await,
        PagesCmd::Delete { id } => delete(ctx, id).await,
    }
}

async fn list(
    ctx: &Ctx,
    status: Option<PageStatusArg>,
    search: Option<String>,
    month: Option<String>,
    limit: u32,
    cursor: Option<String>,
) -> Result<(), CliError> {
    let mut q = vec![("limit", limit.to_string())];
    if let Some(s) = status {
        q.push(("status", s.as_str().to_string()));
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
        .request(Method::GET, "api/v1/pages", Some(&q), None)
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn get(ctx: &Ctx, id: Option<Uuid>, slug: Option<String>) -> Result<(), CliError> {
    let path = match (id, slug) {
        (Some(id), None) => format!("api/v1/pages/{id}"),
        (None, Some(slug)) => format!("api/v1/pages/slug/{slug}"),
        _ => {
            return Err(CliError::InvalidInput(
                "provide exactly one of --id or --slug".to_string(),
            ));
        }
    };

    let res: serde_json::Value = ctx.request(Method::GET, &path, None, None).await?;
    print_json(&res)?;
    Ok(())
}

async fn create(ctx: &Ctx, input: PageCreateInput) -> Result<(), CliError> {
    let PageCreateInput {
        slug,
        title,
        body,
        body_file,
        status,
        scheduled_at,
        published_at,
        archived_at,
    } = input;

    let body_markdown = read_value(body, body_file)?;
    let payload = PageCreateRequest {
        slug,
        title,
        body_markdown,
        status: status.into(),
        scheduled_at: parse_time_opt(scheduled_at)?,
        published_at: parse_time_opt(published_at)?,
        archived_at: parse_time_opt(archived_at)?,
    };
    let res: serde_json::Value = ctx
        .request(Method::POST, "api/v1/pages", None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn update(
    ctx: &Ctx,
    id: Uuid,
    slug: String,
    title: String,
    body: Option<String>,
    body_file: Option<PathBuf>,
) -> Result<(), CliError> {
    let body_markdown = read_value(body, body_file)?;
    let payload = PageUpdateRequest {
        slug,
        title,
        body_markdown,
    };
    let path = format!("api/v1/pages/{id}");
    let res: serde_json::Value = ctx
        .request(Method::PATCH, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_title(ctx: &Ctx, id: Uuid, title: String) -> Result<(), CliError> {
    let payload = PageTitleRequest { title };
    let path = format!("api/v1/pages/{id}/title");
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
    let payload = PageBodyRequest { body_markdown };
    let path = format!("api/v1/pages/{id}/body");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn update_status(
    ctx: &Ctx,
    id: Uuid,
    status: PageStatusArg,
    scheduled_at: Option<String>,
    published_at: Option<String>,
    archived_at: Option<String>,
) -> Result<(), CliError> {
    let payload = PageStatusRequest {
        status: status.into(),
        scheduled_at: parse_time_opt(scheduled_at)?,
        published_at: parse_time_opt(published_at)?,
        archived_at: parse_time_opt(archived_at)?,
    };
    let path = format!("api/v1/pages/{id}/status");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn delete(ctx: &Ctx, id: Uuid) -> Result<(), CliError> {
    let path = format!("api/v1/pages/{id}");
    ctx.request_no_body(Method::DELETE, &path, None).await?;
    println!("deleted");
    Ok(())
}

impl From<PageStatusArg> for PageStatus {
    fn from(value: PageStatusArg) -> Self {
        match value {
            PageStatusArg::Draft => PageStatus::Draft,
            PageStatusArg::Published => PageStatus::Published,
            PageStatusArg::Archived => PageStatus::Archived,
            PageStatusArg::Error => PageStatus::Error,
        }
    }
}
