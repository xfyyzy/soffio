#![deny(clippy::all, clippy::pedantic)]

use reqwest::Method;
use soffio::domain::types::NavigationDestinationType;
use soffio::infra::http::api::models::{
    NavigationCreateRequest, NavigationDestinationRequest, NavigationLabelRequest,
    NavigationOpenInNewTabRequest, NavigationSortOrderRequest, NavigationUpdateRequest,
    NavigationVisibilityRequest,
};
use uuid::Uuid;

use crate::args::{NavCmd, NavDestArg};
use crate::client::{CliError, Ctx};
use crate::io::to_value;
use crate::print::print_json;

pub async fn handle(ctx: &Ctx, cmd: NavCmd) -> Result<(), CliError> {
    match cmd {
        NavCmd::List {
            visible,
            search,
            limit,
            cursor,
        } => list(ctx, visible, search, limit, cursor).await,
        NavCmd::Get { id } => get(ctx, id).await,
        NavCmd::Create {
            label,
            destination_type,
            destination_page_id,
            destination_url,
            sort_order,
            visible,
            open_in_new_tab,
        } => {
            create(
                ctx,
                NavigationCreateInput {
                    label,
                    destination_type,
                    destination_page_id,
                    destination_url,
                    sort_order,
                    visible,
                    open_in_new_tab,
                },
            )
            .await
        }
        NavCmd::Update {
            id,
            label,
            destination_type,
            destination_page_id,
            destination_url,
            sort_order,
            visible,
            open_in_new_tab,
        } => {
            update(
                ctx,
                NavigationUpdateInput {
                    id,
                    label,
                    destination_type,
                    destination_page_id,
                    destination_url,
                    sort_order,
                    visible,
                    open_in_new_tab,
                },
            )
            .await
        }
        NavCmd::PatchLabel { id, label } => patch_label(ctx, id, label).await,
        NavCmd::PatchDestination {
            id,
            destination_type,
            destination_page_id,
            destination_url,
        } => {
            patch_destination(
                ctx,
                id,
                destination_type,
                destination_page_id,
                destination_url,
            )
            .await
        }
        NavCmd::PatchSort { id, sort_order } => patch_sort(ctx, id, sort_order).await,
        NavCmd::PatchVisibility { id, visible } => patch_visibility(ctx, id, visible).await,
        NavCmd::PatchOpen {
            id,
            open_in_new_tab,
        } => patch_open(ctx, id, open_in_new_tab).await,
        NavCmd::Delete { id } => delete(ctx, id).await,
    }
}

struct NavigationCreateInput {
    label: String,
    destination_type: NavDestArg,
    destination_page_id: Option<Uuid>,
    destination_url: Option<String>,
    sort_order: i32,
    visible: bool,
    open_in_new_tab: bool,
}

struct NavigationUpdateInput {
    id: Uuid,
    label: String,
    destination_type: NavDestArg,
    destination_page_id: Option<Uuid>,
    destination_url: Option<String>,
    sort_order: i32,
    visible: bool,
    open_in_new_tab: bool,
}

async fn list(
    ctx: &Ctx,
    visible: Option<bool>,
    search: Option<String>,
    limit: u32,
    cursor: Option<String>,
) -> Result<(), CliError> {
    let mut q = vec![("limit", limit.to_string())];
    if let Some(v) = visible {
        q.push(("visible", v.to_string()));
    }
    if let Some(s) = search {
        q.push(("search", s));
    }
    if let Some(c) = cursor {
        q.push(("cursor", c));
    }
    let res: serde_json::Value = ctx
        .request(Method::GET, "api/v1/navigation", Some(&q), None)
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn get(ctx: &Ctx, id: Uuid) -> Result<(), CliError> {
    let path = format!("api/v1/navigation/{id}");
    let res: serde_json::Value = ctx.request(Method::GET, &path, None, None).await?;
    print_json(&res)?;
    Ok(())
}

async fn create(ctx: &Ctx, input: NavigationCreateInput) -> Result<(), CliError> {
    let NavigationCreateInput {
        label,
        destination_type,
        destination_page_id,
        destination_url,
        sort_order,
        visible,
        open_in_new_tab,
    } = input;

    let payload = NavigationCreateRequest {
        label,
        destination_type: destination_type.into(),
        destination_page_id,
        destination_url,
        sort_order,
        visible,
        open_in_new_tab,
    };
    let res: serde_json::Value = ctx
        .request(
            Method::POST,
            "api/v1/navigation",
            None,
            Some(to_value(payload)?),
        )
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn update(ctx: &Ctx, input: NavigationUpdateInput) -> Result<(), CliError> {
    let NavigationUpdateInput {
        id,
        label,
        destination_type,
        destination_page_id,
        destination_url,
        sort_order,
        visible,
        open_in_new_tab,
    } = input;

    let payload = NavigationUpdateRequest {
        label,
        destination_type: destination_type.into(),
        destination_page_id,
        destination_url,
        sort_order,
        visible,
        open_in_new_tab,
    };
    let path = format!("api/v1/navigation/{id}");
    let res: serde_json::Value = ctx
        .request(Method::PATCH, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_label(ctx: &Ctx, id: Uuid, label: String) -> Result<(), CliError> {
    let payload = NavigationLabelRequest { label };
    let path = format!("api/v1/navigation/{id}/label");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_destination(
    ctx: &Ctx,
    id: Uuid,
    destination_type: NavDestArg,
    destination_page_id: Option<Uuid>,
    destination_url: Option<String>,
) -> Result<(), CliError> {
    let payload = NavigationDestinationRequest {
        destination_type: destination_type.into(),
        destination_page_id,
        destination_url,
    };
    let path = format!("api/v1/navigation/{id}/destination");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_sort(ctx: &Ctx, id: Uuid, sort_order: i32) -> Result<(), CliError> {
    let payload = NavigationSortOrderRequest { sort_order };
    let path = format!("api/v1/navigation/{id}/sort-order");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_visibility(ctx: &Ctx, id: Uuid, visible: bool) -> Result<(), CliError> {
    let payload = NavigationVisibilityRequest { visible };
    let path = format!("api/v1/navigation/{id}/visibility");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn patch_open(ctx: &Ctx, id: Uuid, open_in_new_tab: bool) -> Result<(), CliError> {
    let payload = NavigationOpenInNewTabRequest { open_in_new_tab };
    let path = format!("api/v1/navigation/{id}/open-in-new-tab");
    let res: serde_json::Value = ctx
        .request(Method::POST, &path, None, Some(to_value(payload)?))
        .await?;
    print_json(&res)?;
    Ok(())
}

async fn delete(ctx: &Ctx, id: Uuid) -> Result<(), CliError> {
    let path = format!("api/v1/navigation/{id}");
    ctx.request_no_body(Method::DELETE, &path, None).await?;
    println!("deleted");
    Ok(())
}

impl From<NavDestArg> for NavigationDestinationType {
    fn from(value: NavDestArg) -> Self {
        match value {
            NavDestArg::Internal => NavigationDestinationType::Internal,
            NavDestArg::External => NavigationDestinationType::External,
        }
    }
}
