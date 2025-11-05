use crate::domain::entities::TagRecord;
use crate::presentation::admin::views as admin_views;

pub(super) fn build_tag_edit_view(tag: &TagRecord) -> admin_views::AdminTagEditView {
    admin_views::AdminTagEditView {
        heading: format!("Edit Tag: {}", tag.name),
        id: tag.id.to_string(),
        name: tag.name.clone(),
        description: tag.description.clone(),
        pinned: tag.pinned,
        form_action: format!("/tags/{}/edit", tag.id),
        submit_label: "Save Changes".to_string(),
        pin_label: "Pin to top of filter".to_string(),
    }
}

pub(super) fn build_new_tag_view() -> admin_views::AdminTagEditView {
    admin_views::AdminTagEditView {
        heading: "Create Tag".to_string(),
        id: "new".to_string(),
        name: String::new(),
        description: None,
        pinned: false,
        form_action: "/tags/create".to_string(),
        submit_label: "Create Tag".to_string(),
        pin_label: "Pin to top of filter".to_string(),
    }
}
