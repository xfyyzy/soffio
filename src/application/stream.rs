//! Helpers for building server-driven datastar SSE responses.

use std::convert::Infallible;

use async_stream::stream;
use axum::response::{
    IntoResponse, Response,
    sse::{Event, Sse},
};
use datastar::prelude::{ElementPatchMode, ExecuteScript, PatchElements, PatchSignals};

/// Builder for composing datastar-compatible SSE responses.
pub struct StreamBuilder {
    events: Vec<Event>,
}

impl StreamBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Append an element patch targeting the supplied selector.
    pub fn push_patch(
        &mut self,
        html: String,
        selector: &str,
        mode: ElementPatchMode,
    ) -> &mut Self {
        let event = PatchElements::new(html)
            .selector(selector)
            .mode(mode)
            .write_as_axum_sse_event();
        self.events.push(event);
        self
    }

    /// Queue an inline script for execution on the client.
    pub fn push_script(&mut self, script: String) -> &mut Self {
        let event = ExecuteScript::new(script).write_as_axum_sse_event();
        self.events.push(event);
        self
    }

    /// Queue a datastar signal patch.
    pub fn push_signals(&mut self, payload: &str) -> &mut Self {
        let event = PatchSignals::new(payload).write_as_axum_sse_event();
        self.events.push(event);
        self
    }

    /// Append a pre-built SSE event.
    pub fn push_event(&mut self, event: Event) -> &mut Self {
        self.events.push(event);
        self
    }

    /// Finalise the builder into an Axum response.
    pub fn into_response(self) -> Response {
        let stream = stream! {
            for event in self.events {
                yield Ok::<Event, Infallible>(event);
            }
        };
        Sse::new(stream).into_response()
    }

    /// Returns true when no events have been scheduled.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for StreamBuilder {
    fn default() -> Self {
        Self::new()
    }
}
