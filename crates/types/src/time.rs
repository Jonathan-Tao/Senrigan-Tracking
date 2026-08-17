//! This module owns runtime monotonic-clock values and stream order.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ContractError;

/// Stores nanoseconds on the runtime monotonic clock.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct MonotonicTime(u64);

impl MonotonicTime {
    /// Creates a runtime clock value from nanoseconds.
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// Returns the runtime clock value in nanoseconds.
    pub const fn as_nanos(self) -> u64 {
        self.0
    }

    /// Returns the elapsed time since `earlier`.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::InvalidTimeOrder`] if `earlier` follows `self`.
    pub fn checked_duration_since(self, earlier: Self) -> Result<Duration, ContractError> {
        self.0
            .checked_sub(earlier.0)
            .map(Duration::from_nanos)
            .ok_or(ContractError::InvalidTimeOrder {
                earlier: "earlier timestamp",
                later: "later timestamp",
            })
    }
}

/// Checks sequence and source-time order for one stream.
///
/// Sequence numbers must increase. Source times may stay equal but cannot move
/// backward.
///
/// # Errors
///
/// Returns [`ContractError::InvalidSequence`] if the sequence does not increase.
/// Returns [`ContractError::InvalidTimeOrder`] if source time moves backward.
pub fn validate_stream_order(
    previous_sequence: u64,
    previous_time: MonotonicTime,
    next_sequence: u64,
    next_time: MonotonicTime,
) -> Result<(), ContractError> {
    if next_sequence <= previous_sequence {
        return Err(ContractError::InvalidSequence);
    }
    if next_time < previous_time {
        return Err(ContractError::InvalidTimeOrder {
            earlier: "previous source time",
            later: "next source time",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_monotonic_timestamps() {
        let first = MonotonicTime::from_nanos(10);
        let second = MonotonicTime::from_nanos(25);
        assert_eq!(second.checked_duration_since(first).unwrap().as_nanos(), 15);
        assert!(first.checked_duration_since(second).is_err());
    }

    #[test]
    fn rejects_reversed_stream_order() {
        let first = MonotonicTime::from_nanos(10);
        let second = MonotonicTime::from_nanos(20);
        assert!(validate_stream_order(1, first, 2, second).is_ok());
        assert!(validate_stream_order(2, first, 2, second).is_err());
        assert!(validate_stream_order(1, second, 2, first).is_err());
    }
}
