#![deny(clippy::all, clippy::pedantic)]

use reqwest::Method;
use soffio::infra::http::api::models::SettingsPatchRequest;

use crate::args::{SettingsCmd, SettingsPatchArgs};
use crate::client::{CliError, Ctx};
use crate::io::{read_opt_value, to_value};
use crate::print::print_json;

pub async fn handle(ctx: &Ctx, cmd: SettingsCmd) -> Result<(), CliError> {
    match cmd {
        SettingsCmd::Get => get(ctx).await,
        SettingsCmd::Patch(settings) => patch(ctx, *settings).await,
    }
}

async fn get(ctx: &Ctx) -> Result<(), CliError> {
    let res: serde_json::Value = ctx
        .request(Method::GET, "api/v1/site/settings", None, None)
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch(ctx: &Ctx, settings: SettingsPatchArgs) -> Result<(), CliError> {
    let SettingsPatchArgs {
        brand_title,
        brand_href,
        footer_copy,
        homepage_size,
        admin_page_size,
        show_tag_aggregations,
        show_month_aggregations,
        tag_filter_limit,
        month_filter_limit,
        timezone,
        meta_title,
        meta_description,
        og_title,
        og_description,
        public_site_url,
        global_toc_enabled,
        favicon_svg,
        favicon_svg_file,
    } = settings;

    let favicon_svg = read_opt_value(favicon_svg, favicon_svg_file)?;
    let payload = SettingsPatchRequest {
        brand_title,
        brand_href,
        footer_copy,
        homepage_size,
        admin_page_size,
        show_tag_aggregations,
        show_month_aggregations,
        tag_filter_limit,
        month_filter_limit,
        timezone,
        meta_title,
        meta_description,
        og_title,
        og_description,
        public_site_url,
        global_toc_enabled,
        favicon_svg,
    };
    let res: serde_json::Value = ctx
        .request(
            Method::PATCH,
            "api/v1/site/settings",
            None,
            Some(to_value(payload)?),
        )
        .await?;
    print_json(&res)?;
    Ok(())
}
