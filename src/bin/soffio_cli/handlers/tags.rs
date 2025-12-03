#![deny(clippy::all, clippy::pedantic)]

use std::path::PathBuf;

use reqwest::Method;
use soffio::infra::http::api::models::{
    TagCreateRequest, TagDescriptionRequest, TagNameRequest, TagPinRequest, TagUpdateRequest,
};
use uuid::Uuid;

use crate::args::TagsCmd;
use crate::client::{CliError, Ctx};
use crate::io::{read_opt_value, to_value};
use crate::print::print_json;

pub async fn handle(ctx: &Ctx, cmd: TagsCmd) -> Result<(), CliError> {
    match cmd {
        TagsCmd::List {
            pinned,
            search,
            month,
            limit,
            cursor,
        } => list(ctx, pinned, search, month, limit, cursor).await,
        TagsCmd::Create {
            name,
            description,
            description_file,
            pinned,
        } => create(ctx, name, description, description_file, pinned).await,
        TagsCmd::Update {
            id,
            name,
            description,
            description_file,
            pinned,
        } => update(ctx, id, name, description, description_file, pinned).await,
        TagsCmd::PatchPin { id, pinned } => patch_pin(ctx, id, pinned).await,
        TagsCmd::PatchName { id, name } => patch_name(ctx, id, name).await,
        TagsCmd::PatchDescription {
            id,
            description,
            description_file,
        } => patch_description(ctx, id, description, description_file).await,
        TagsCmd::Delete { id } => delete(ctx, id).await,
    }
}

async fn list(
    ctx: &Ctx,
    pinned: Option<bool>,
    search: Option<String>,
    month: Option<String>,
    limit: u32,
    cursor: Option<String>,
) -> Result<(), CliError> {
    let mut q = vec![("limit", limit.to_string())];
    if let Some(p) = pinned {
        q.push(("pinned", p.to_string()));
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
        .request(Method::GET, "api/v1/tags", Some(&q), None)
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn create(
    ctx: &Ctx,
    name: String,
    description: Option<String>,
    description_file: Option<PathBuf>,
    pinned: bool,
) -> Result<(), CliError> {
    let description = read_opt_value(description, description_file)?;
    let payload = TagCreateRequest {
        name,
        description,
        pinned,
    };
    let res: serde_json::Value = ctx
        .request(Method::POST, "api/v1/tags", None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn update(
    ctx: &Ctx,
    id: Uuid,
    name: String,
    description: Option<String>,
    description_file: Option<PathBuf>,
    pinned: bool,
) -> Result<(), CliError> {
    let description = read_opt_value(description, description_file)?;
    let payload = TagUpdateRequest {
        name,
        description,
        pinned,
    };
    let path = format!("api/v1/tags/{id}");
    let res: serde_json::Value = ctx
        .request(Method::PATCH, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_pin(ctx: &Ctx, id: Uuid, pinned: bool) -> Result<(), CliError> {
    let payload = TagPinRequest { pinned };
    let path = format!("api/v1/tags/{id}/pin");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_name(ctx: &Ctx, id: Uuid, name: String) -> Result<(), CliError> {
    let payload = TagNameRequest { name };
    let path = format!("api/v1/tags/{id}/name");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_description(
    ctx: &Ctx,
    id: Uuid,
    description: Option<String>,
    description_file: Option<PathBuf>,
) -> Result<(), CliError> {
    let description = read_opt_value(description, description_file)?;
    let payload = TagDescriptionRequest { description };
    let path = format!("api/v1/tags/{id}/description");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn delete(ctx: &Ctx, id: Uuid) -> Result<(), CliError> {
    let path = format!("api/v1/tags/{id}");
    ctx.request_no_body(Method::DELETE, &path, None).await?;
    println!("deleted");
    Ok(())
}
