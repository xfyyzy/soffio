//! Rendering service interface planned for Phase 3.
//!
//! The rendering pipeline is intentionally kept pure: it accepts markdown input,
//! produces deterministic HTML output, and surfaces structured errors. State
//! changes (such as recording job outcomes) happen in the caller, typically a
//! background worker executing within the Phase 6 scheduler.

mod jobs;
mod runtime;
mod service;
mod types;

pub use jobs::{
    RenderPageJobPayload, RenderPostJobPayload, RenderPostSectionJobPayload,
    RenderPostSectionsJobPayload, RenderSummaryJobPayload, enqueue_render_page_job,
    enqueue_render_post_job, process_render_page_job, process_render_post_job,
    process_render_post_section_job, process_render_post_sections_job, process_render_summary_job,
};
pub use runtime::{InFlightRenders, RenderArtifact, RenderMailbox};
pub use service::{
    ComrakRenderService, RenderConfigError, RenderPipelineConfig, configure_render_service,
    render_service,
};
pub use types::{
    RenderError, RenderOutput, RenderRequest, RenderService, RenderTarget, RenderedSection,
};
