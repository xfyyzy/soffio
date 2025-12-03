#![deny(clippy::all, clippy::pedantic)]

use reqwest::Method;
use soffio::infra::http::api::models::ApiKeyInfoResponse;

use crate::args::ApiKeysCmd;
use crate::client::{CliError, Ctx};
use crate::print::print_json;

pub async fn handle(ctx: &Ctx, cmd: ApiKeysCmd) -> Result<(), CliError> {
    match cmd.action {
        crate::args::ApiKeysAction::Me => {
            let res: ApiKeyInfoResponse = ctx
                .request(Method::GET, "api/v1/api-keys/me", None, None)
                .await?;
            print_json(&res)?;
        }
    }
    Ok(())
}
