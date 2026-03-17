#![deny(clippy::all, clippy::pedantic)]

use reqwest::Method;

use crate::args::JobsCmd;
use crate::client::{CliError, Ctx};
use crate::print::print_json;

pub async fn handle(ctx: &Ctx, cmd: JobsCmd) -> Result<(), CliError> {
    match cmd {
        JobsCmd::List {
            state,
            job_type,
            search,
            limit,
            cursor,
        } => list(ctx, state, job_type, search, limit, cursor).await,
    }
}

async fn list(
    ctx: &Ctx,
    state: Option<String>,
    job_type: Option<String>,
    search: Option<String>,
    limit: u32,
    cursor: Option<String>,
) -> Result<(), CliError> {
    let mut q = vec![("limit", limit.to_string())];
    if let Some(s) = state {
        q.push(("state", s));
    }
    if let Some(t) = job_type {
        q.push(("job_type", t));
    }
    if let Some(s) = search {
        q.push(("search", s));
    }
    if let Some(c) = cursor {
        q.push(("cursor", c));
    }
    let res: serde_json::Value = ctx
        .request(Method::GET, "api/v1/jobs", Some(&q), None)
        .await?;
    print_json(&res)?;
    Ok(())
}
