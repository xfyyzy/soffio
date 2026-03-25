use askama::Template;
use soffio::presentation::admin::views::*;
use uuid::Uuid;

macro_rules! assert_admin_snapshot {
    ($name:expr, $rendered:expr) => {
        insta::with_settings!({ prepend_module_to_snapshot => false }, {
            insta::assert_snapshot!($name, $rendered);
        });
    };
}

#[path = "admin_panels/api_keys.rs"]
mod api_keys;
#[path = "admin_panels/navigation.rs"]
mod navigation;
#[path = "admin_panels/pages.rs"]
mod pages;
#[path = "admin_panels/posts.rs"]
mod posts;
#[path = "admin_panels/settings.rs"]
mod settings;
#[path = "admin_panels/tags.rs"]
mod tags;
#[path = "admin_panels/uploads.rs"]
mod uploads;
