mod editor;
mod errors;
mod forms;
mod handlers;
mod panel;
mod status;

pub(super) use handlers::{
    admin_page_archive, admin_page_create, admin_page_delete, admin_page_edit,
    admin_page_move_to_draft, admin_page_new, admin_page_panel, admin_page_publish,
    admin_page_update, admin_pages,
};
