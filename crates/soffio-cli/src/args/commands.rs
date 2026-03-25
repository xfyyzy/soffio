#[path = "commands/api_keys.rs"]
mod api_keys;
#[path = "commands/audit.rs"]
mod audit;
#[path = "commands/jobs.rs"]
mod jobs;
#[path = "commands/navigation.rs"]
mod navigation;
#[path = "commands/pages.rs"]
mod pages;
#[path = "commands/posts.rs"]
mod posts;
#[path = "commands/settings.rs"]
mod settings;
#[path = "commands/snapshots.rs"]
mod snapshots;
#[path = "commands/tags.rs"]
mod tags;
#[path = "commands/uploads.rs"]
mod uploads;

// Re-export the full CLI command surface from split modules.
// Some consumers (e.g. doc generators that `#[path]` this module) only use
// a subset, so these imports can look unused in those specific targets.
#[allow(unused_imports)]
pub use api_keys::{ApiKeysAction, ApiKeysCmd};
#[allow(unused_imports)]
pub use audit::{AuditArgs, AuditCmd};
#[allow(unused_imports)]
pub use jobs::{JobsArgs, JobsCmd};
#[allow(unused_imports)]
pub use navigation::{NavArgs, NavCmd};
#[allow(unused_imports)]
pub use pages::{PagesArgs, PagesCmd};
#[allow(unused_imports)]
pub use posts::{PostsArgs, PostsCmd};
#[allow(unused_imports)]
pub use settings::{SettingsArgs, SettingsCmd, SettingsPatchArgs};
#[allow(unused_imports)]
pub use snapshots::{SnapshotsArgs, SnapshotsCmd};
#[allow(unused_imports)]
pub use tags::{TagsArgs, TagsCmd};
#[allow(unused_imports)]
pub use uploads::{UploadsArgs, UploadsCmd};
