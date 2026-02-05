use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

use tracing::warn;

pub(crate) fn rw_read<'a, T>(
    lock: &'a RwLock<T>,
    target: &'static str,
    op: &'static str,
) -> RwLockReadGuard<'a, T> {
    match lock.read() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!(
                op,
                target_module = target,
                lock_kind = "rwlock.read",
                result = "poisoned_recovered",
                hint = "state may be stale after panic in another thread",
                "Recovered from poisoned cache lock"
            );
            poisoned.into_inner()
        }
    }
}

pub(crate) fn rw_write<'a, T>(
    lock: &'a RwLock<T>,
    target: &'static str,
    op: &'static str,
) -> RwLockWriteGuard<'a, T> {
    match lock.write() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!(
                op,
                target_module = target,
                lock_kind = "rwlock.write",
                result = "poisoned_recovered",
                hint = "state may be stale after panic in another thread",
                "Recovered from poisoned cache lock"
            );
            poisoned.into_inner()
        }
    }
}

pub(crate) fn mutex_lock<'a, T>(
    lock: &'a Mutex<T>,
    target: &'static str,
    op: &'static str,
) -> MutexGuard<'a, T> {
    match lock.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!(
                op,
                target_module = target,
                lock_kind = "mutex.lock",
                result = "poisoned_recovered",
                hint = "state may be stale after panic in another thread",
                "Recovered from poisoned cache lock"
            );
            poisoned.into_inner()
        }
    }
}
