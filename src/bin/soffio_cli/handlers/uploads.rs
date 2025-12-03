#![deny(clippy::all, clippy::pedantic)]

use std::fs;
use std::path::PathBuf;

use reqwest::Method;
use uuid::Uuid;

use crate::args::UploadsCmd;
use crate::client::{CliError, Ctx};
use crate::print::print_json;

pub async fn handle(ctx: &Ctx, cmd: UploadsCmd) -> Result<(), CliError> {
    match cmd {
        UploadsCmd::List {
            content_type,
            search,
            month,
            limit,
            cursor,
        } => list(ctx, content_type, search, month, limit, cursor).await,
        UploadsCmd::Upload { file } => upload(ctx, file).await,
        UploadsCmd::Delete { id } => delete(ctx, id).await,
    }
}

async fn list(
    ctx: &Ctx,
    content_type: Option<String>,
    search: Option<String>,
    month: Option<String>,
    limit: u32,
    cursor: Option<String>,
) -> Result<(), CliError> {
    let mut q = vec![("limit", limit.to_string())];
    if let Some(ct) = content_type {
        q.push(("content_type", ct));
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
        .request(Method::GET, "api/v1/uploads", Some(&q), None)
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn upload(ctx: &Ctx, file: PathBuf) -> Result<(), CliError> {
    let url = ctx.url("api/v1/uploads")?;
    let data = fs::read(&file).map_err(CliError::KeyFile)?;
    let part = reqwest::multipart::Part::bytes(data).file_name(
        file.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("upload.bin")
            .to_string(),
    );
    let form = reqwest::multipart::Form::new().part("file", part);
    let resp = ctx
        .client
        .post(url)
        .header(axum::http::header::AUTHORIZATION, ctx.auth_header()?)
        .multipart(form)
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(CliError::Server(format!("status {status} body {text}")));
    }
    println!("{text}");
    Ok(())
}

async fn delete(ctx: &Ctx, id: Uuid) -> Result<(), CliError> {
    let path = format!("api/v1/uploads/{id}");
    ctx.request_no_body(Method::DELETE, &path, None).await?;
    println!("deleted");
    Ok(())
}
