//! This module owns finite canonical motion published by the estimator.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    ContractError, EstimateOrigin, JointId, MonotonicTime, NonNegativeF32, Point3, REQUIRED_JOINTS,
    S, SchemaVersion, Transform, UnitInterval, UnitQuaternion, Validate, Vector3,
    VisibilityEvidence, W,
};

/// Stores a canonical joint rotation relative to its parent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointRotation {
    pub joint_id: JointId,
    pub rotation: UnitQuaternion,
}

/// Stores a canonical joint position in site frame `W`, in meters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointPosition {
    pub joint_id: JointId,
    pub position_w: Point3<W>,
}

/// Stores a joint rest offset in source frame `S`, in meters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestOffset {
    pub joint_id: JointId,
    pub offset_s: Vector3<S>,
}

/// Stores the linear and angular velocity of one canonical joint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointVelocity {
    pub joint_id: JointId,
    /// Stores linear velocity in site frame `W`, in meters per second.
    pub linear_w: Vector3<W>,
    /// Stores angular velocity in source frame `S`, in radians per second.
    pub angular_s: Vector3<S>,
}

/// Stores position and rotation uncertainty for one canonical joint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointUncertainty {
    pub joint_id: JointId,
    /// Stores position uncertainty in meters.
    pub position: NonNegativeF32,
    /// Stores rotation uncertainty in radians.
    pub rotation: NonNegativeF32,
}

/// Retains image evidence for one canonical joint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JointVisibility {
    pub joint_id: JointId,
    pub evidence: VisibilityEvidence,
}

/// Retains the production path for one canonical joint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JointEstimateOrigin {
    pub joint_id: JointId,
    pub origin: EstimateOrigin,
}

/// Identifies a canonical body region that can propose contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContactRegion {
    LeftFoot,
    RightFoot,
    LeftHand,
    RightHand,
}

/// Stores estimator contact evidence for a canonical body region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContactProposal {
    pub source_region: ContactRegion,
    pub confidence: UnitInterval,
}

/// Reports the source of the performer scale in this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PerformerScaleStatus {
    /// The scale uses a population prior.
    PopulationPrior,
    /// Performer fit can still update the scale.
    Provisional,
    /// Performer fit has fixed the scale.
    Locked,
}

/// Carries body-model-independent source motion from the estimator.
///
/// Deserialization rejects invalid time order, topology, and numeric values.
// Validate external data before use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct CanonicalMotionFrame {
    pub schema_version: SchemaVersion,
    /// Records the accepted observation time on the runtime monotonic clock.
    pub source_time: MonotonicTime,
    /// Records the authoritative estimator state time.
    pub state_time: MonotonicTime,
    /// Records the scheduled publication time on the runtime monotonic clock.
    pub output_time: MonotonicTime,
    /// Maps source frame `S` into site frame `W`.
    pub t_w_s: Transform<W, S>,
    pub local_joint_rotations: Vec<JointRotation>,
    pub global_joint_positions_w: Vec<JointPosition>,
    pub rest_offsets_s: Vec<RestOffset>,
    pub linear_and_angular_velocities: Vec<JointVelocity>,
    pub joint_uncertainty: Vec<JointUncertainty>,
    pub visibility_evidence: Vec<JointVisibility>,
    pub estimate_origin: Vec<JointEstimateOrigin>,
    pub contact_proposals: Vec<ContactProposal>,
    /// Records the accepted global evidence for root correction.
    pub global_root_support: UnitInterval,
    pub performer_scale_status: PerformerScaleStatus,
}

impl Serialize for CanonicalMotionFrame {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        CanonicalMotionFrame::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for CanonicalMotionFrame {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let frame = CanonicalMotionFrame::deserialize(deserializer)?;
        frame.validate().map_err(de::Error::custom)?;
        Ok(frame)
    }
}

impl Validate for CanonicalMotionFrame {
    fn validate(&self) -> Result<(), ContractError> {
        // 1. Check causal time order.
        if self.source_time > self.state_time {
            return Err(ContractError::InvalidTimeOrder {
                earlier: "source_time",
                later: "state_time",
            });
        }
        if self.state_time > self.output_time {
            return Err(ContractError::InvalidTimeOrder {
                earlier: "state_time",
                later: "output_time",
            });
        }

        // 2. Set the published joint topology.
        let expected = validate_required_joints(
            "local_joint_rotations",
            self.local_joint_rotations
                .iter()
                .map(|value| value.joint_id),
        )?;
        // 3. Match each channel to that topology.
        validate_matching_joints(
            "global_joint_positions_w",
            self.global_joint_positions_w
                .iter()
                .map(|value| value.joint_id),
            &expected,
        )?;
        validate_matching_joints(
            "rest_offsets_s",
            self.rest_offsets_s.iter().map(|value| value.joint_id),
            &expected,
        )?;
        validate_matching_joints(
            "linear_and_angular_velocities",
            self.linear_and_angular_velocities
                .iter()
                .map(|value| value.joint_id),
            &expected,
        )?;
        validate_matching_joints(
            "joint_uncertainty",
            self.joint_uncertainty.iter().map(|value| value.joint_id),
            &expected,
        )?;
        validate_matching_joints(
            "visibility_evidence",
            self.visibility_evidence.iter().map(|value| value.joint_id),
            &expected,
        )?;
        validate_matching_joints(
            "estimate_origin",
            self.estimate_origin.iter().map(|value| value.joint_id),
            &expected,
        )?;
        Ok(())
    }
}

fn collect_joint_set(
    field: &'static str,
    joints: impl IntoIterator<Item = JointId>,
) -> Result<BTreeSet<JointId>, ContractError> {
    let mut found = BTreeSet::new();
    for joint in joints {
        if !found.insert(joint) {
            return Err(ContractError::DuplicateJoint {
                field,
                joint: joint.canonical_name(),
            });
        }
    }
    Ok(found)
}

fn validate_required_joints(
    field: &'static str,
    joints: impl IntoIterator<Item = JointId>,
) -> Result<BTreeSet<JointId>, ContractError> {
    let found = collect_joint_set(field, joints)?;
    for spec in REQUIRED_JOINTS {
        if !found.contains(&spec.joint) {
            return Err(ContractError::MissingJoint {
                field,
                joint: spec.joint.canonical_name(),
            });
        }
    }
    Ok(found)
}

fn validate_matching_joints(
    field: &'static str,
    joints: impl IntoIterator<Item = JointId>,
    expected: &BTreeSet<JointId>,
) -> Result<(), ContractError> {
    if collect_joint_set(field, joints)? != *expected {
        return Err(ContractError::JointSetMismatch { field });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> CanonicalMotionFrame {
        let joints = REQUIRED_JOINTS.map(|spec| spec.joint);
        CanonicalMotionFrame {
            schema_version: SchemaVersion::current(),
            source_time: MonotonicTime::from_nanos(10),
            state_time: MonotonicTime::from_nanos(20),
            output_time: MonotonicTime::from_nanos(30),
            t_w_s: Transform::new(
                UnitQuaternion::identity(),
                Vector3::try_new(0.0, 0.0, 0.0).unwrap(),
            ),
            local_joint_rotations: joints
                .map(|joint_id| JointRotation {
                    joint_id,
                    rotation: UnitQuaternion::identity(),
                })
                .to_vec(),
            global_joint_positions_w: joints
                .map(|joint_id| JointPosition {
                    joint_id,
                    position_w: Point3::try_new(0.0, 0.0, 0.0).unwrap(),
                })
                .to_vec(),
            rest_offsets_s: joints
                .map(|joint_id| RestOffset {
                    joint_id,
                    offset_s: Vector3::try_new(0.0, 0.0, 0.0).unwrap(),
                })
                .to_vec(),
            linear_and_angular_velocities: joints
                .map(|joint_id| JointVelocity {
                    joint_id,
                    linear_w: Vector3::try_new(0.0, 0.0, 0.0).unwrap(),
                    angular_s: Vector3::try_new(0.0, 0.0, 0.0).unwrap(),
                })
                .to_vec(),
            joint_uncertainty: joints
                .map(|joint_id| JointUncertainty {
                    joint_id,
                    position: NonNegativeF32::new(0.0).unwrap(),
                    rotation: NonNegativeF32::new(0.0).unwrap(),
                })
                .to_vec(),
            visibility_evidence: joints
                .map(|joint_id| JointVisibility {
                    joint_id,
                    evidence: VisibilityEvidence::Observed,
                })
                .to_vec(),
            estimate_origin: joints
                .map(|joint_id| JointEstimateOrigin {
                    joint_id,
                    origin: EstimateOrigin::Fresh,
                })
                .to_vec(),
            contact_proposals: Vec::new(),
            global_root_support: UnitInterval::new(1.0).unwrap(),
            performer_scale_status: PerformerScaleStatus::Locked,
        }
    }

    #[test]
    fn canonical_schema_round_trip() {
        let expected = fixture();
        expected.validate().unwrap();
        let encoded = serde_json::to_vec(&expected).unwrap();
        let decoded: CanonicalMotionFrame = serde_json::from_slice(&encoded).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn canonical_frame_rejects_missing_required_joint() {
        let mut frame = fixture();
        frame.local_joint_rotations.pop();
        assert!(matches!(
            frame.validate(),
            Err(ContractError::MissingJoint {
                field: "local_joint_rotations",
                ..
            })
        ));
        let value = serde_json::to_value(frame).unwrap();
        assert!(serde_json::from_value::<CanonicalMotionFrame>(value).is_err());
    }

    #[test]
    fn canonical_frame_rejects_mismatched_optional_joints() {
        let mut frame = fixture();
        frame.local_joint_rotations.push(JointRotation {
            joint_id: JointId::UpperChest,
            rotation: UnitQuaternion::identity(),
        });
        assert!(matches!(
            frame.validate(),
            Err(ContractError::JointSetMismatch { .. })
        ));
    }

    #[test]
    fn canonical_frame_rejects_invalid_time_order() {
        let mut frame = fixture();
        frame.state_time = MonotonicTime::from_nanos(31);
        assert!(frame.validate().is_err());
        let value = serde_json::to_value(frame).unwrap();
        assert!(serde_json::from_value::<CanonicalMotionFrame>(value).is_err());
    }

    #[test]
    fn canonical_deserialization_rejects_invalid_numeric_data() {
        let mut value = serde_json::to_value(fixture()).unwrap();
        value["t_w_s"]["rotation"] = serde_json::json!([0.0, 0.0, 0.0, 0.0]);
        assert!(serde_json::from_value::<CanonicalMotionFrame>(value).is_err());
    }
}
