//! Cron job for expiring API keys that have passed their expiration date.

use std::sync::Arc;

use apalis::prelude::*;
use cron::Schedule;
use std::str::FromStr;

use crate::application::api_keys::ApiKeyService;

/// Marker struct for the cron-triggered expiration job.
/// Must implement `From<chrono::DateTime<chrono::Utc>>` for apalis-cron compatibility.
#[derive(Default, Debug, Clone)]
pub struct ExpireApiKeysJob;

impl From<chrono::DateTime<chrono::Utc>> for ExpireApiKeysJob {
    fn from(_: chrono::DateTime<chrono::Utc>) -> Self {
        Self
    }
}

/// Context for the expiration job worker.
#[derive(Clone)]
pub struct ExpireApiKeysContext {
    pub api_keys: Arc<ApiKeyService>,
}

/// Process the expiration job: update status for keys past their expires_at.
pub async fn process_expire_api_keys_job(
    _job: ExpireApiKeysJob,
    ctx: Data<ExpireApiKeysContext>,
) -> Result<(), apalis::prelude::Error> {
    match ctx.api_keys.expire_keys().await {
        Ok(count) if count > 0 => {
            tracing::info!(expired_count = count, "Expired API keys");
        }
        Err(err) => {
            tracing::warn!(error = %err, "Failed to expire API keys");
        }
        _ => {}
    }
    Ok(())
}

/// Create the cron schedule for API key expiration.
/// Runs every hour at minute 0: "0 * * * *"
pub fn expire_api_keys_schedule() -> Schedule {
    Schedule::from_str("0 0 * * * *").expect("Invalid cron expression for expire_api_keys")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_parses_correctly() {
        let schedule = expire_api_keys_schedule();
        // Should have upcoming times
        let upcoming: Vec<_> = schedule.upcoming(chrono::Utc).take(3).collect();
        assert_eq!(upcoming.len(), 3);
    }
}
