//! Cache event system.
//!
//! Defines cache events and an in-memory queue for event-driven invalidation.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use time::OffsetDateTime;
use tracing::info;
use uuid::Uuid;

use super::lock::mutex_lock;

const SOURCE: &str = "cache::events";

/// Monotonic epoch for ordering events.
///
/// Each event gets a unique, monotonically increasing epoch number.
/// Used to determine which event is "latest" when merging multiple events
/// for the same entity.
pub type Epoch = u64;

/// Cache event with idempotency and ordering support.
#[derive(Debug, Clone)]
pub struct CacheEvent {
    /// Unique identifier for idempotency (UUIDv4).
    pub id: Uuid,
    /// Monotonic epoch for ordering within this process.
    pub epoch: Epoch,
    /// The type of cache event.
    pub kind: EventKind,
    /// When the event was created.
    pub timestamp: OffsetDateTime,
}

impl CacheEvent {
    /// Create a new cache event with the given kind and epoch.
    pub fn new(kind: EventKind, epoch: Epoch) -> Self {
        Self {
            id: Uuid::new_v4(),
            epoch,
            kind,
            timestamp: OffsetDateTime::now_utc(),
        }
    }
}

/// Types of cache events that trigger invalidation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    // Singletons
    /// Site settings were updated.
    SiteSettingsUpdated,
    /// Navigation menu was updated.
    NavigationUpdated,

    // Content
    /// A post was created or updated.
    PostUpserted { post_id: Uuid, slug: String },
    /// A post was deleted.
    PostDeleted { post_id: Uuid, slug: String },
    /// A page was created or updated.
    PageUpserted { page_id: Uuid, slug: String },
    /// A page was deleted.
    PageDeleted { page_id: Uuid, slug: String },

    // Security
    /// An API key was created or updated.
    ApiKeyUpserted { prefix: String },
    /// An API key was revoked.
    ApiKeyRevoked { prefix: String },

    // Startup
    /// Warm the cache on application startup.
    WarmupOnStartup,
}

/// In-memory event queue for cache invalidation.
///
/// Events are published by write operations and consumed by the cache consumer.
/// The queue uses a mutex for simplicity since contention is expected to be low.
pub struct EventQueue {
    queue: Mutex<VecDeque<CacheEvent>>,
    epoch_counter: AtomicU64,
}

impl EventQueue {
    /// Create a new empty event queue.
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            epoch_counter: AtomicU64::new(0),
        }
    }

    /// Get the next epoch number.
    pub fn next_epoch(&self) -> Epoch {
        self.epoch_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Publish an event to the queue.
    ///
    /// The event is logged for observability.
    pub fn publish(&self, kind: EventKind) {
        let epoch = self.next_epoch();
        let event = CacheEvent::new(kind.clone(), epoch);

        // Observable: log event enqueue
        info!(
            event_id = %event.id,
            event_epoch = event.epoch,
            event_kind = ?kind,
            "Cache event enqueued"
        );

        mutex_lock(&self.queue, SOURCE, "publish").push_back(event);
    }

    /// Drain up to `limit` events from the queue.
    ///
    /// Returns the events in FIFO order.
    pub fn drain(&self, limit: usize) -> Vec<CacheEvent> {
        let mut queue = mutex_lock(&self.queue, SOURCE, "drain");
        let count = limit.min(queue.len());
        queue.drain(..count).collect()
    }

    /// Get the current queue length.
    pub fn len(&self) -> usize {
        mutex_lock(&self.queue, SOURCE, "len").len()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all events from the queue.
    pub fn clear(&self) {
        mutex_lock(&self.queue, SOURCE, "clear").clear();
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;

    #[test]
    fn event_creation() {
        let kind = EventKind::SiteSettingsUpdated;
        let event = CacheEvent::new(kind.clone(), 42);

        assert_eq!(event.epoch, 42);
        assert_eq!(event.kind, kind);
        assert!(!event.id.is_nil());
    }

    #[test]
    fn epoch_monotonicity() {
        let queue = EventQueue::new();

        let e1 = queue.next_epoch();
        let e2 = queue.next_epoch();
        let e3 = queue.next_epoch();

        assert!(e1 < e2);
        assert!(e2 < e3);
    }

    #[test]
    fn publish_and_drain() {
        let queue = EventQueue::new();

        queue.publish(EventKind::SiteSettingsUpdated);
        queue.publish(EventKind::NavigationUpdated);
        queue.publish(EventKind::PostUpserted {
            post_id: Uuid::nil(),
            slug: "test".to_string(),
        });

        assert_eq!(queue.len(), 3);

        let events = queue.drain(2);
        assert_eq!(events.len(), 2);
        assert_eq!(queue.len(), 1);

        // Check order (FIFO)
        assert_eq!(events[0].kind, EventKind::SiteSettingsUpdated);
        assert_eq!(events[1].kind, EventKind::NavigationUpdated);
    }

    #[test]
    fn drain_more_than_available() {
        let queue = EventQueue::new();

        queue.publish(EventKind::SiteSettingsUpdated);

        let events = queue.drain(100);
        assert_eq!(events.len(), 1);
        assert!(queue.is_empty());
    }

    #[test]
    fn clear_queue() {
        let queue = EventQueue::new();

        queue.publish(EventKind::SiteSettingsUpdated);
        queue.publish(EventKind::NavigationUpdated);
        assert!(!queue.is_empty());

        queue.clear();
        assert!(queue.is_empty());
    }

    #[test]
    fn event_kind_equality() {
        let kind1 = EventKind::PostUpserted {
            post_id: Uuid::nil(),
            slug: "test".to_string(),
        };
        let kind2 = EventKind::PostUpserted {
            post_id: Uuid::nil(),
            slug: "test".to_string(),
        };
        let kind3 = EventKind::PostUpserted {
            post_id: Uuid::nil(),
            slug: "other".to_string(),
        };

        assert_eq!(kind1, kind2);
        assert_ne!(kind1, kind3);
    }

    #[test]
    fn event_queue_recovers_from_poisoned_lock() {
        let queue = EventQueue::new();

        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = queue.queue.lock().expect("queue lock should be acquired");
            panic!("poison queue lock");
        }));

        queue.publish(EventKind::SiteSettingsUpdated);
        assert_eq!(queue.len(), 1);
    }
}
