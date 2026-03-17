#![deny(clippy::all, clippy::pedantic)]

use reqwest::Method;

use crate::args::AuditCmd;
use crate::client::{CliError, Ctx};
use crate::print::print_json;

pub async fn handle(ctx: &Ctx, cmd: AuditCmd) -> Result<(), CliError> {
    match cmd {
        AuditCmd::List {
            actor,
            action,
            entity_type,
            search,
            limit,
            cursor,
        } => list(ctx, actor, action, entity_type, search, limit, cursor).await,
    }
}

async fn list(
    ctx: &Ctx,
    actor: Option<String>,
    action: Option<String>,
    entity_type: Option<String>,
    search: Option<String>,
    limit: u32,
    cursor: Option<String>,
) -> Result<(), CliError> {
    let mut q = vec![("limit", limit.to_string())];
    if let Some(a) = actor {
        q.push(("actor", a));
    }
    if let Some(a) = action {
        q.push(("action", a));
    }
    if let Some(e) = entity_type {
        q.push(("entity_type", e));
    }
    if let Some(s) = search {
        q.push(("search", s));
    }
    if let Some(c) = cursor {
        q.push(("cursor", c));
    }
    let res: serde_json::Value = ctx
        .request(Method::GET, "api/v1/audit", Some(&q), None)
        .await?;
    print_json(&res)?;
    Ok(())
}
