mod errors;
mod forms;
mod handlers;
mod panel;
mod sections;
mod status;

pub(super) use handlers::{
    admin_post_archive, admin_post_create, admin_post_delete, admin_post_edit,
    admin_post_move_to_draft, admin_post_new, admin_post_pin, admin_post_publish,
    admin_post_tags_toggle, admin_post_tags_toggle_new, admin_post_unpin, admin_post_update,
    admin_posts, admin_posts_bulk_action, admin_posts_panel,
};
