#![deny(clippy::all, clippy::pedantic)]

use reqwest::Method;
use serde_json::json;

use crate::args::SnapshotsCmd;
use crate::client::{CliError, Ctx};
use crate::print;

pub async fn handle(ctx: &Ctx, cmd: SnapshotsCmd) -> Result<(), CliError> {
    match cmd {
        SnapshotsCmd::List {
            entity_type,
            entity_id,
            search,
            limit,
            cursor,
        } => {
            let mut q = vec![("limit", limit.to_string())];
            if let Some(t) = entity_type {
                q.push(("entity_type", t));
            }
            if let Some(id) = entity_id {
                q.push(("entity_id", id.to_string()));
            }
            if let Some(s) = search {
                q.push(("search", s));
            }
            if let Some(c) = cursor {
                q.push(("cursor", c));
            }
            let resp: serde_json::Value = ctx
                .request(Method::GET, "/api/v1/snapshots", Some(&q), None)
                .await?;
            print::json_value(&resp)?;
        }
        SnapshotsCmd::Get { id } => {
            let path = format!("/api/v1/snapshots/{id}");
            let resp: serde_json::Value = ctx.request(Method::GET, &path, None, None).await?;
            print::json_value(&resp)?;
        }
        SnapshotsCmd::Create {
            entity_type,
            entity_id,
            description,
        } => {
            let body = json!({
                "entity_type": entity_type,
                "entity_id": entity_id,
                "description": description,
            });
            let resp: serde_json::Value = ctx
                .request(Method::POST, "/api/v1/snapshots", None, Some(body))
                .await?;
            print::json_value(&resp)?;
        }
        SnapshotsCmd::Rollback { id } => {
            let path = format!("/api/v1/snapshots/{id}/rollback");
            let _: serde_json::Value = ctx.request(Method::POST, &path, None, None).await?;
            println!("Rolled back snapshot {id}");
        }
    }

    Ok(())
}
