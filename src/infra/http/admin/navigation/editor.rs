//! Navigation editor view building functions.

use askama::Template;

use crate::application::error::HttpError;
use crate::domain::entities::NavigationItemRecord;
use crate::domain::types::NavigationDestinationType;
use crate::infra::http::admin::AdminState;
use crate::infra::http::admin::shared::{blank_to_none_opt, template_render_http_error};
use crate::presentation::admin::views as admin_views;

use super::forms::AdminNavigationForm;
use super::panel::admin_navigation_error;
use super::status::{
    navigation_destination_options, navigation_type_key, parse_navigation_type, parse_optional_uuid,
};

pub(super) async fn build_navigation_editor_view(
    state: &AdminState,
    item: Option<&NavigationItemRecord>,
    form: Option<&AdminNavigationForm>,
) -> Result<admin_views::AdminNavigationEditorView, HttpError> {
    let pages = state
        .navigation
        .published_pages()
        .await
        .map_err(|err| admin_navigation_error("infra::http::admin_navigation_editor", err))?;

    let destination_type = form
        .and_then(|f| parse_navigation_type(&f.destination_type).ok())
        .or_else(|| item.map(|i| i.destination_type))
        .unwrap_or(NavigationDestinationType::Internal);

    let destination_page_id = form
        .and_then(|f| parse_optional_uuid(f.destination_page_id.as_deref()))
        .or_else(|| item.and_then(|i| i.destination_page_id));

    let page_has_selection = destination_page_id.is_some();

    let destination_url = if destination_type == NavigationDestinationType::External {
        form.and_then(|f| blank_to_none_opt(f.destination_url.clone()))
            .or_else(|| item.and_then(|i| i.destination_url.clone()))
    } else {
        form.and_then(|f| blank_to_none_opt(f.destination_url.clone()))
    };

    let label = form
        .map(|f| f.label.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| item.map(|i| i.label.clone()))
        .unwrap_or_default();

    let sort_order = form
        .map(|f| f.sort_order)
        .or_else(|| item.map(|i| i.sort_order))
        .unwrap_or(0);

    let visible = form
        .map(|f| f.visible.is_some())
        .or_else(|| item.map(|i| i.visible))
        .unwrap_or(true);

    let open_in_new_tab = form
        .map(|f| f.open_in_new_tab.is_some())
        .or_else(|| item.map(|i| i.open_in_new_tab))
        .unwrap_or(false);

    let destination_type_options = navigation_destination_options(destination_type);

    let mut page_options: Vec<admin_views::AdminNavigationPageOption> = pages
        .into_iter()
        .map(|page| admin_views::AdminNavigationPageOption {
            id: page.id.to_string(),
            title: page.title,
            slug: page.slug,
            selected: Some(page.id) == destination_page_id,
        })
        .collect();
    page_options.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

    let id_value = item.map(|i| i.id.to_string());
    let toggle_suffix = id_value.as_deref().unwrap_or("new");
    let visible_input_id = format!("navigation-visible-{}", toggle_suffix);
    let open_in_new_tab_input_id = format!("navigation-open-in-new-tab-{}", toggle_suffix);

    Ok(admin_views::AdminNavigationEditorView {
        heading: match item {
            Some(item) => format!("Edit Navigation Item: {}", item.label),
            None => "Create Navigation Item".to_string(),
        },
        id: id_value,
        label,
        destination_type_options,
        page_options,
        destination_url,
        sort_order,
        visible,
        open_in_new_tab,
        page_has_selection,
        form_action: item
            .map(|i| format!("/navigation/{}/edit", i.id))
            .unwrap_or_else(|| "/navigation/create".to_string()),
        submit_label: if item.is_some() {
            "Save Changes".to_string()
        } else {
            "Create Item".to_string()
        },
        enable_live_submit: true,
        active_destination_type: navigation_type_key(destination_type).to_string(),
        preview_action: item
            .map(|i| format!("/navigation/{}/destination-preview", i.id))
            .unwrap_or_else(|| "/navigation/destination-preview".to_string()),
        visible_input_id,
        open_in_new_tab_input_id,
    })
}

pub(super) fn render_navigation_editor_panel(
    content: &admin_views::AdminNavigationEditorView,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let template = admin_views::AdminNavigationEditPanelTemplate {
        content: content.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}
