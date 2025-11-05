use std::sync::Arc;

use dashmap::DashMap;
use thiserror::Error;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::infra::db::PersistedPostSectionOwned;

/// Coordinated mailbox used by render jobs to exchange in-memory artifacts.
///
/// We rely on single-process deployment: all workers share this structure via
/// [`JobWorkerContext`]. Each child job publishes its result using the
/// `tracking_id` embedded in the payload, while the parent job awaits the
/// matching channel.
#[derive(Default, Clone)]
pub struct RenderMailbox {
    inner: Arc<DashMap<String, oneshot::Sender<RenderArtifact>>>,
}

impl RenderMailbox {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    /// Register a tracking identifier and obtain a receiver for downstream
    /// results.
    pub fn register(&self, tracking_id: String) -> oneshot::Receiver<RenderArtifact> {
        let (tx, rx) = oneshot::channel();
        self.inner.insert(tracking_id, tx);
        rx
    }

    /// Deliver an artifact to a previously registered tracking identifier.
    pub fn deliver(
        &self,
        tracking_id: &str,
        artifact: RenderArtifact,
    ) -> Result<(), RenderMailboxError> {
        match self.inner.remove(tracking_id) {
            Some((_id, sender)) => sender
                .send(artifact)
                .map_err(|_| RenderMailboxError::ChannelClosed),
            None => Err(RenderMailboxError::UnknownTrackingId),
        }
    }

    /// Cancel a pending receiver with a specific reason.
    pub fn cancel(&self, tracking_id: &str, reason: RenderMailboxError) {
        if let Some((_id, sender)) = self.inner.remove(tracking_id) {
            let _ = sender.send(RenderArtifact::Cancelled(reason));
        }
    }
}

#[derive(Debug, Error, Clone)]
pub enum RenderMailboxError {
    #[error("render mailbox channel already closed")]
    ChannelClosed,
    #[error("unknown render tracking id")]
    UnknownTrackingId,
    #[error("render task aborted: {0}")]
    Aborted(String),
}

/// Results that can be exchanged between render jobs.
#[derive(Debug, Clone)]
pub enum RenderArtifact {
    Sections(Vec<PersistedPostSectionOwned>),
    Section(PersistedPostSectionOwned),
    SummaryHtml(String),
    Cancelled(RenderMailboxError),
}

/// Tracks posts that currently have an in-flight render container job.
#[derive(Default, Clone)]
pub struct InFlightRenders {
    posts: Arc<DashMap<Uuid, ()>>,
}

#[derive(Debug, Error)]
pub enum InFlightError {
    #[error("render already in progress for post {post_id}")]
    AlreadyRunning { post_id: Uuid },
}

impl InFlightRenders {
    pub fn new() -> Self {
        Self {
            posts: Arc::new(DashMap::new()),
        }
    }

    pub fn acquire(&self, post_id: Uuid) -> Result<RenderGuard, InFlightError> {
        use dashmap::mapref::entry::Entry;

        match self.posts.entry(post_id) {
            Entry::Vacant(vacant) => {
                vacant.insert(());
                Ok(RenderGuard {
                    post_id,
                    posts: Arc::clone(&self.posts),
                })
            }
            Entry::Occupied(_) => Err(InFlightError::AlreadyRunning { post_id }),
        }
    }
}

pub struct RenderGuard {
    post_id: Uuid,
    posts: Arc<DashMap<Uuid, ()>>,
}

impl Drop for RenderGuard {
    fn drop(&mut self) {
        self.posts.remove(&self.post_id);
    }
}
