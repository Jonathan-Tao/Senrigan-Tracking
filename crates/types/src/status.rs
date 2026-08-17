//! This module owns tracking lifecycle, status, and stage outcomes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    EstimateOrigin, InferencePlacement, JointId, NonNegativeF32, UnitInterval, VisibilityEvidence,
};

/// Reports the estimator lifecycle with each output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrackingLifecycle {
    /// Reliable global evidence supports normal correction.
    Tracking,
    /// Reliable evidence supports only part of the body.
    Partial,
    /// Prediction advances state without reliable global evidence.
    Predicting,
    /// Damped state retains the last valid motion.
    Holding,
    /// Output holds while no performer track is active.
    Lost,
    /// An isolated candidate is being confirmed or blended.
    Reacquiring,
}

/// Reports the current validity of the saved site calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CalibrationStatus {
    /// No site calibration is available.
    Unavailable,
    /// Calibration has not passed final validation.
    Provisional,
    /// Calibration matches the current camera and mode.
    Valid,
    /// Camera evidence no longer supports the saved calibration.
    Stale,
}

/// Carries status beside canonical motion on each output tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackingStatus {
    pub lifecycle: TrackingLifecycle,
    /// Reports the age of the last accepted observation in nanoseconds.
    pub last_observation_age_ns: u64,
    pub visibility: BTreeMap<JointId, VisibilityEvidence>,
    pub estimate_origins: BTreeMap<JointId, EstimateOrigin>,
    /// Reports the accepted evidence for global root correction.
    pub root_support: UnitInterval,
    /// Reports a dimensionless aggregate of estimator uncertainty.
    pub uncertainty_summary: NonNegativeF32,
    pub active_placement: Option<InferencePlacement>,
    /// Records the reason for the active fallback, if present.
    pub fallback_reason: Option<String>,
    /// Reports the confirmed fraction of the reacquisition process.
    pub reacquisition_progress: UnitInterval,
    pub calibration_status: CalibrationStatus,
}

/// Reports a typed success or failure from one runtime stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "outcome",
    content = "detail",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
pub enum StageOutcome<T> {
    /// The stage produced a valid value.
    Success(T),
    /// The stage received no usable observation.
    NoObservation { reason: String },
    /// The input arrived too late for the stage.
    StaleInput {
        /// Records the input age in nanoseconds.
        age_ns: u64,
    },
    /// The stage exceeded its deadline.
    DeadlineMissed {
        stage: String,
        /// Records the stage duration in nanoseconds.
        elapsed_ns: u64,
    },
    /// The stage produced data that failed validation.
    InvalidOutput { reason: String },
    /// The selected execution provider failed.
    ProviderFailure { provider: String, reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_schema_uses_contract_names() {
        let states = [
            TrackingLifecycle::Tracking,
            TrackingLifecycle::Partial,
            TrackingLifecycle::Predicting,
            TrackingLifecycle::Holding,
            TrackingLifecycle::Lost,
            TrackingLifecycle::Reacquiring,
        ];
        let encoded = serde_json::to_string(&states).unwrap();
        assert_eq!(
            encoded,
            r#"["TRACKING","PARTIAL","PREDICTING","HOLDING","LOST","REACQUIRING"]"#
        );
        let decoded: Vec<TrackingLifecycle> = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, states);
    }
}
