//! Cache consumer for executing consumption plans.
//!
//! Consumes events from the queue and executes invalidation/warming actions.

use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use metrics::histogram;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::application::pagination::PageRequest;
use crate::application::repos::{
    NavigationQueryFilter, NavigationRepo, PagesRepo, PostListScope, PostQueryFilter, PostsRepo,
    SettingsRepo, TagsRepo,
};
use crate::infra::db::PostgresRepositories;

use super::config::CacheConfig;
use super::events::EventQueue;
use super::keys::{CacheKey, EntityKey};
use super::planner::ConsumptionPlan;
use super::registry::CacheRegistry;
use super::store::{L0Store, L1Store};

mod invalidation;
#[cfg(test)]
mod tests;
mod warm;

const METRIC_CACHE_CONSUME_MS: &str = "soffio_cache_consume_ms";
const METRIC_CACHE_WARM_MS: &str = "soffio_cache_warm_ms";

/// Cache consumer that processes events and maintains cache consistency.
///
/// The consumer:
/// 1. Drains events from the queue
/// 2. Generates a consumption plan from the events
/// 3. Executes the plan (invalidate L0, invalidate L1, warm)
pub struct CacheConsumer {
    config: CacheConfig,
    l0: Arc<L0Store>,
    l1: Arc<L1Store>,
    registry: Arc<CacheRegistry>,
    queue: Arc<EventQueue>,
    repos: Option<Arc<PostgresRepositories>>,
    #[cfg(test)]
    warm_invocations: Arc<AtomicUsize>,
}

impl CacheConsumer {
    /// Create a new cache consumer with repository access for warming.
    pub fn new(
        config: CacheConfig,
        l0: Arc<L0Store>,
        l1: Arc<L1Store>,
        registry: Arc<CacheRegistry>,
        queue: Arc<EventQueue>,
        repos: Arc<PostgresRepositories>,
    ) -> Self {
        Self {
            config,
            l0,
            l1,
            registry,
            queue,
            repos: Some(repos),
            #[cfg(test)]
            warm_invocations: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create a cache consumer without repository access (warming disabled).
    ///
    /// This is primarily for testing purposes.
    #[cfg(test)]
    pub fn new_without_repos(
        config: CacheConfig,
        l0: Arc<L0Store>,
        l1: Arc<L1Store>,
        registry: Arc<CacheRegistry>,
        queue: Arc<EventQueue>,
    ) -> Self {
        Self {
            config,
            l0,
            l1,
            registry,
            queue,
            repos: None,
            warm_invocations: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Consume pending events and execute the plan.
    ///
    /// Returns true if any events were processed.
    #[instrument(skip(self))]
    pub async fn consume(&self) -> bool {
        self.consume_with_mode(true).await
    }

    /// Consume pending events and run only invalidation actions.
    ///
    /// Useful on latency-sensitive write paths where pre-warming is deferred.
    #[instrument(skip(self))]
    pub async fn consume_invalidate_only(&self) -> bool {
        self.consume_with_mode(false).await
    }

    /// Consume pending events and run both invalidation and warming actions.
    #[instrument(skip(self))]
    pub async fn consume_full(&self) -> bool {
        self.consume_with_mode(true).await
    }

    async fn consume_with_mode(&self, include_warm: bool) -> bool {
        let consume_started_at = Instant::now();
        let events = self.queue.drain(self.config.consume_batch_limit);
        if events.is_empty() {
            return false;
        }

        let event_count = events.len();
        let event_ids: Vec<Uuid> = events.iter().map(|e| e.id).collect();
        let plan = ConsumptionPlan::from_events(events);

        // Observable: log consumption start with plan details
        info!(
            event_count,
            event_ids = ?event_ids,
            plan = %plan,
            include_warm,
            "Cache consumption starting"
        );

        // Phase 1: Invalidate L0 (skip if disabled or no entities to invalidate)
        if self.config.enable_l0_cache && !plan.invalidate_entities.is_empty() {
            self.invalidate_l0(&plan);
        }

        // Phase 2: Invalidate L1 using registry (skip if no entities to invalidate)
        if !plan.invalidate_entities.is_empty() {
            self.invalidate_l1(&plan);
        }

        // Phase 3: Warm cache from repositories (skip if L0 disabled or no warm actions)
        if include_warm && self.config.enable_l0_cache && plan.has_warm_actions() {
            self.warm(&plan).await;
        }

        // Observable: log consumption complete
        info!(
            event_count,
            invalidated = plan.invalidate_entities.len(),
            "Cache consumption complete"
        );

        histogram!(
            METRIC_CACHE_CONSUME_MS,
            "mode" => if include_warm { "full" } else { "invalidate_only" }
        )
        .record(consume_started_at.elapsed().as_secs_f64() * 1000.0);

        true
    }

    /// Get reference to the event queue.
    pub fn queue(&self) -> &Arc<EventQueue> {
        &self.queue
    }

    /// Get reference to the L0 store.
    pub fn l0(&self) -> &Arc<L0Store> {
        &self.l0
    }

    /// Get reference to the L1 store.
    pub fn l1(&self) -> &Arc<L1Store> {
        &self.l1
    }

    /// Get reference to the registry.
    pub fn registry(&self) -> &Arc<CacheRegistry> {
        &self.registry
    }

    #[cfg(test)]
    fn warm_invocation_count(&self) -> usize {
        self.warm_invocations.load(Ordering::Relaxed)
    }
}
