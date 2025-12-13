//! Application services for the administrative surface.

pub mod audit;
pub mod chrome;
pub mod dashboard;
pub mod jobs;
pub mod navigation;
pub mod pages;
pub mod posts;
pub mod settings;
pub mod snapshot_types;
pub mod snapshots;
pub mod tags;
pub mod uploads;

pub use snapshots::AdminSnapshotService;
