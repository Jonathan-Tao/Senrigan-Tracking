//! This module owns spatial-model observations and evidence labels.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    C, ContractError, FiniteF32, JointId, MonotonicTime, NonNegativeF32, Point3, StageOutcome,
    UnitInterval, UnitQuaternion, Validate,
};

/// Describes the image evidence available for one joint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VisibilityEvidence {
    /// The image contains usable evidence for the joint.
    Observed,
    /// Another subject or object hides the joint.
    Occluded,
    /// The camera frame or crop cuts off the joint.
    Truncated,
    /// The adapter cannot classify the evidence.
    Unknown,
}

/// Describes how the engine produced one joint estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EstimateOrigin {
    /// A current accepted observation produced the estimate.
    Fresh,
    /// State prediction produced the estimate.
    Predicted,
    /// The engine retained the last valid estimate.
    Held,
    /// A declared fallback path produced the estimate.
    Fallback,
}

/// Stores an axis-aligned rectangle in source-image pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: FiniteF32,
    pub y: FiniteF32,
    pub width: NonNegativeF32,
    pub height: NonNegativeF32,
}

/// Stores the image transform applied before spatial-model inference.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CropTransform {
    /// Maps source-image pixels to model-input pixels with a row-major
    /// homogeneous transform.
    pub matrix: [[FiniteF32; 3]; 3],
}

/// Carries one camera-relative canonical joint observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointObservation {
    pub joint_id: JointId,
    /// Stores the joint position in camera frame `C`, in meters.
    pub position_c: Point3<C>,
    /// Stores the optional canonical local joint rotation.
    pub local_rotation: Option<UnitQuaternion>,
    /// Stores per-axis position variance in `C`, in square meters.
    pub position_covariance: [NonNegativeF32; 3],
    /// Stores rotation uncertainty in radians.
    pub rotation_uncertainty: NonNegativeF32,
    pub visibility_evidence: VisibilityEvidence,
    /// Stores distance to the nearest frame edge in source-image pixels.
    /// Positive values are inside the frame. Negative values are outside it.
    pub signed_frame_edge_distance: FiniteF32,
}

/// Carries one spatial-model result for one captured frame.
// Validate external data before use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct SpatialObservation {
    /// Identifies the source [`crate::FramePacket`] sequence.
    pub source_frame_sequence: u64,
    /// Records source frame time on the runtime monotonic clock.
    pub source_time: MonotonicTime,
    /// Records inference completion time on the runtime monotonic clock.
    pub completed_time: MonotonicTime,
    pub model_id: String,
    pub model_version: String,
    pub adapter_version: String,
    pub crop_transform: CropTransform,
    /// Stores the theoretical performer region before sensor-edge clamping.
    pub unclamped_performer_roi: Rect,
    /// Stores the performer region available to preprocessing.
    pub clamped_input_roi: Rect,
    /// Reports the fraction of the theoretical performer region in the input.
    pub visible_roi_fraction: UnitInterval,
    /// Reports the torso or pelvis evidence available for root correction.
    pub torso_root_support: UnitInterval,
    pub joints: Vec<JointObservation>,
    /// Stores optional adapter-defined shape and proportion values.
    pub shape_proportion_proposal: Option<Vec<FiniteF32>>,
}

impl Serialize for SpatialObservation {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SpatialObservation::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for SpatialObservation {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let observation = SpatialObservation::deserialize(deserializer)?;
        observation.validate().map_err(de::Error::custom)?;
        Ok(observation)
    }
}

/// Carries a typed result across the spatial-model boundary.
pub type SpatialObservationOutcome = StageOutcome<SpatialObservation>;

impl Validate for SpatialObservation {
    fn validate(&self) -> Result<(), ContractError> {
        if self.source_time > self.completed_time {
            return Err(ContractError::InvalidTimeOrder {
                earlier: "source_time",
                later: "completed_time",
            });
        }
        for (field, value) in [
            ("model_id", self.model_id.as_str()),
            ("model_version", self.model_version.as_str()),
            ("adapter_version", self.adapter_version.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ContractError::EmptyField { field });
            }
        }
        let mut joints = BTreeSet::new();
        for observation in &self.joints {
            if !joints.insert(observation.joint_id) {
                return Err(ContractError::DuplicateJoint {
                    field: "joints",
                    joint: observation.joint_id.canonical_name(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation() -> SpatialObservation {
        let zero = FiniteF32::new(0.0).unwrap();
        SpatialObservation {
            source_frame_sequence: 1,
            source_time: MonotonicTime::from_nanos(10),
            completed_time: MonotonicTime::from_nanos(20),
            model_id: "fixture".into(),
            model_version: "1".into(),
            adapter_version: "1".into(),
            crop_transform: CropTransform {
                matrix: [[zero; 3]; 3],
            },
            unclamped_performer_roi: Rect {
                x: zero,
                y: zero,
                width: NonNegativeF32::new(1.0).unwrap(),
                height: NonNegativeF32::new(1.0).unwrap(),
            },
            clamped_input_roi: Rect {
                x: zero,
                y: zero,
                width: NonNegativeF32::new(1.0).unwrap(),
                height: NonNegativeF32::new(1.0).unwrap(),
            },
            visible_roi_fraction: UnitInterval::new(1.0).unwrap(),
            torso_root_support: UnitInterval::new(1.0).unwrap(),
            joints: Vec::new(),
            shape_proportion_proposal: None,
        }
    }

    #[test]
    fn observation_schema_round_trip() {
        let expected = observation();
        expected.validate().unwrap();
        let encoded = serde_json::to_vec(&expected).unwrap();
        let decoded: SpatialObservation = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn observation_rejects_time_reversal() {
        let mut value = observation();
        value.completed_time = MonotonicTime::from_nanos(9);
        assert!(value.validate().is_err());
        let encoded = serde_json::to_value(value).unwrap();
        assert!(serde_json::from_value::<SpatialObservation>(encoded).is_err());
    }

    #[test]
    fn observation_rejects_a_duplicate_joint() {
        let joint = JointObservation {
            joint_id: JointId::Hips,
            position_c: Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            local_rotation: None,
            position_covariance: [NonNegativeF32::new(0.0).unwrap(); 3],
            rotation_uncertainty: NonNegativeF32::new(0.0).unwrap(),
            visibility_evidence: VisibilityEvidence::Observed,
            signed_frame_edge_distance: FiniteF32::new(0.0).unwrap(),
        };
        let mut value = observation();
        value.joints = vec![joint.clone(), joint];
        let encoded = serde_json::to_value(value).unwrap();
        assert!(serde_json::from_value::<SpatialObservation>(encoded).is_err());
    }
}
