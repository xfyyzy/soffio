use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

use crate::domain::types::SnapshotEntityType;

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("validation failed: {0}")]
    Validation(String),
}

pub trait Snapshotable {
    type Id: ToString + Clone;
    type Payload: Serialize + DeserializeOwned;

    const ENTITY_TYPE: SnapshotEntityType;

    /// Stable identifier for the entity being snapshotted.
    fn id(&self) -> &Self::Id;

    /// Produce a snapshot payload capturing the entity's current state.
    fn to_snapshot(&self) -> Result<Self::Payload, SnapshotError>;

    /// Validate a snapshot payload before applying.
    fn validate_snapshot(payload: &Self::Payload) -> Result<(), SnapshotError>;
}
