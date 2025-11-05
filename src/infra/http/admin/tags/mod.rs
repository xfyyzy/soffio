mod editor;
mod errors;
mod forms;
mod handlers;
mod panel;
mod status;

pub(super) use handlers::{
    admin_tag_create, admin_tag_delete, admin_tag_edit, admin_tag_new, admin_tag_pin,
    admin_tag_unpin, admin_tag_update, admin_tags, admin_tags_panel,
};
