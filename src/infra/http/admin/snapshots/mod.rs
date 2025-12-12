pub mod panel;
pub use panel::{
    admin_page_snapshots, admin_page_snapshots_panel, admin_post_snapshots,
    admin_post_snapshots_panel,
};

pub mod new;
pub use new::{
    admin_page_snapshot_create, admin_page_snapshot_new, admin_post_snapshot_create,
    admin_post_snapshot_new,
};

pub mod edit;
pub use edit::{admin_snapshot_edit, admin_snapshot_update};
