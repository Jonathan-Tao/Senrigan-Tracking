//! This crate owns shared Senrigan contracts.
//!
//! All persisted numeric values enter through validated types. Coordinate
//! transforms state the destination frame before the source frame.

pub mod capture;
pub mod coordinate;
pub mod joints;
pub mod manifest;
pub mod motion;
pub mod numeric;
pub mod observation;
pub mod schema;
pub mod status;
pub mod time;

pub use capture::*;
pub use coordinate::*;
pub use joints::*;
pub use manifest::*;
pub use motion::*;
pub use numeric::*;
pub use observation::*;
pub use schema::*;
pub use status::*;
pub use time::*;

use thiserror::Error;

/// Reports a contract validation failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ContractError {
    #[error("{field} must be finite")]
    NonFinite { field: &'static str },
    #[error("{field} is outside its valid range")]
    OutOfRange { field: &'static str },
    #[error("quaternion must be finite and have unit length")]
    InvalidQuaternion,
    #[error("timestamp order is invalid: {earlier} must not follow {later}")]
    InvalidTimeOrder {
        earlier: &'static str,
        later: &'static str,
    },
    #[error("frame sequence must increase")]
    InvalidSequence,
    #[error("schema version {found} is unsupported; expected {expected}")]
    UnsupportedSchemaVersion { found: u32, expected: u32 },
    #[error("required field {field} is empty")]
    EmptyField { field: &'static str },
    #[error("required canonical joint {joint} is missing from {field}")]
    MissingJoint {
        field: &'static str,
        joint: &'static str,
    },
    #[error("canonical joint {joint} occurs more than once in {field}")]
    DuplicateJoint {
        field: &'static str,
        joint: &'static str,
    },
    #[error("canonical joint set in {field} does not match local_joint_rotations")]
    JointSetMismatch { field: &'static str },
    #[error("frame image layout is invalid")]
    InvalidImageLayout,
}

/// Defines cross-field validation for a contract type.
pub trait Validate {
    /// Checks all invariants that field types cannot enforce.
    ///
    /// # Errors
    ///
    /// Returns the first contract violation.
    fn validate(&self) -> Result<(), ContractError>;
}
