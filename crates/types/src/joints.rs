//! This module owns the body-model-independent canonical joint topology.
//!
//! Joint names follow Virtual Reality Model (VRM) 1 where versions differ.

use serde::{Deserialize, Serialize};

/// Identifies a joint in the body-model-independent canonical skeleton.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JointId {
    Hips,
    Spine,
    Chest,
    UpperChest,
    Neck,
    Head,
    LeftShoulder,
    LeftUpperArm,
    LeftLowerArm,
    LeftHand,
    LeftThumbMetacarpal,
    LeftThumbProximal,
    LeftThumbDistal,
    LeftIndexProximal,
    LeftIndexIntermediate,
    LeftIndexDistal,
    LeftMiddleProximal,
    LeftMiddleIntermediate,
    LeftMiddleDistal,
    LeftRingProximal,
    LeftRingIntermediate,
    LeftRingDistal,
    LeftLittleProximal,
    LeftLittleIntermediate,
    LeftLittleDistal,
    RightShoulder,
    RightUpperArm,
    RightLowerArm,
    RightHand,
    RightThumbMetacarpal,
    RightThumbProximal,
    RightThumbDistal,
    RightIndexProximal,
    RightIndexIntermediate,
    RightIndexDistal,
    RightMiddleProximal,
    RightMiddleIntermediate,
    RightMiddleDistal,
    RightRingProximal,
    RightRingIntermediate,
    RightRingDistal,
    RightLittleProximal,
    RightLittleIntermediate,
    RightLittleDistal,
    LeftUpperLeg,
    LeftLowerLeg,
    LeftFoot,
    LeftToes,
    RightUpperLeg,
    RightLowerLeg,
    RightFoot,
    RightToes,
    LeftEye,
    RightEye,
    Jaw,
}

impl JointId {
    /// Returns the canonical serialized joint name.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Hips => "hips",
            Self::Spine => "spine",
            Self::Chest => "chest",
            Self::UpperChest => "upperChest",
            Self::Neck => "neck",
            Self::Head => "head",
            Self::LeftShoulder => "leftShoulder",
            Self::LeftUpperArm => "leftUpperArm",
            Self::LeftLowerArm => "leftLowerArm",
            Self::LeftHand => "leftHand",
            Self::LeftThumbMetacarpal => "leftThumbMetacarpal",
            Self::LeftThumbProximal => "leftThumbProximal",
            Self::LeftThumbDistal => "leftThumbDistal",
            Self::LeftIndexProximal => "leftIndexProximal",
            Self::LeftIndexIntermediate => "leftIndexIntermediate",
            Self::LeftIndexDistal => "leftIndexDistal",
            Self::LeftMiddleProximal => "leftMiddleProximal",
            Self::LeftMiddleIntermediate => "leftMiddleIntermediate",
            Self::LeftMiddleDistal => "leftMiddleDistal",
            Self::LeftRingProximal => "leftRingProximal",
            Self::LeftRingIntermediate => "leftRingIntermediate",
            Self::LeftRingDistal => "leftRingDistal",
            Self::LeftLittleProximal => "leftLittleProximal",
            Self::LeftLittleIntermediate => "leftLittleIntermediate",
            Self::LeftLittleDistal => "leftLittleDistal",
            Self::RightShoulder => "rightShoulder",
            Self::RightUpperArm => "rightUpperArm",
            Self::RightLowerArm => "rightLowerArm",
            Self::RightHand => "rightHand",
            Self::RightThumbMetacarpal => "rightThumbMetacarpal",
            Self::RightThumbProximal => "rightThumbProximal",
            Self::RightThumbDistal => "rightThumbDistal",
            Self::RightIndexProximal => "rightIndexProximal",
            Self::RightIndexIntermediate => "rightIndexIntermediate",
            Self::RightIndexDistal => "rightIndexDistal",
            Self::RightMiddleProximal => "rightMiddleProximal",
            Self::RightMiddleIntermediate => "rightMiddleIntermediate",
            Self::RightMiddleDistal => "rightMiddleDistal",
            Self::RightRingProximal => "rightRingProximal",
            Self::RightRingIntermediate => "rightRingIntermediate",
            Self::RightRingDistal => "rightRingDistal",
            Self::RightLittleProximal => "rightLittleProximal",
            Self::RightLittleIntermediate => "rightLittleIntermediate",
            Self::RightLittleDistal => "rightLittleDistal",
            Self::LeftUpperLeg => "leftUpperLeg",
            Self::LeftLowerLeg => "leftLowerLeg",
            Self::LeftFoot => "leftFoot",
            Self::LeftToes => "leftToes",
            Self::RightUpperLeg => "rightUpperLeg",
            Self::RightLowerLeg => "rightLowerLeg",
            Self::RightFoot => "rightFoot",
            Self::RightToes => "rightToes",
            Self::LeftEye => "leftEye",
            Self::RightEye => "rightEye",
            Self::Jaw => "jaw",
        }
    }
}

/// Selects the parent of one canonical joint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentRule {
    /// The joint starts the canonical hierarchy.
    Root,
    /// The joint always uses the named parent.
    Fixed(JointId),
    /// The joint uses an optional parent when present.
    PreferOptional {
        optional: JointId,
        fallback: JointId,
    },
}

/// Defines one joint in the canonical topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalJointSpec {
    pub joint: JointId,
    pub parent: ParentRule,
    /// Whether every canonical motion frame must contain the joint.
    pub required: bool,
}

/// Lists the required joints from the canonical motion contract.
pub const REQUIRED_JOINTS: [CanonicalJointSpec; 17] = [
    CanonicalJointSpec {
        joint: JointId::Hips,
        parent: ParentRule::Root,
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::Spine,
        parent: ParentRule::Fixed(JointId::Hips),
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::Chest,
        parent: ParentRule::Fixed(JointId::Spine),
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::Neck,
        parent: ParentRule::PreferOptional {
            optional: JointId::UpperChest,
            fallback: JointId::Chest,
        },
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::Head,
        parent: ParentRule::Fixed(JointId::Neck),
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::LeftUpperArm,
        parent: ParentRule::PreferOptional {
            optional: JointId::LeftShoulder,
            fallback: JointId::Chest,
        },
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::LeftLowerArm,
        parent: ParentRule::Fixed(JointId::LeftUpperArm),
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::LeftHand,
        parent: ParentRule::Fixed(JointId::LeftLowerArm),
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::RightUpperArm,
        parent: ParentRule::PreferOptional {
            optional: JointId::RightShoulder,
            fallback: JointId::Chest,
        },
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::RightLowerArm,
        parent: ParentRule::Fixed(JointId::RightUpperArm),
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::RightHand,
        parent: ParentRule::Fixed(JointId::RightLowerArm),
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::LeftUpperLeg,
        parent: ParentRule::Fixed(JointId::Hips),
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::LeftLowerLeg,
        parent: ParentRule::Fixed(JointId::LeftUpperLeg),
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::LeftFoot,
        parent: ParentRule::Fixed(JointId::LeftLowerLeg),
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::RightUpperLeg,
        parent: ParentRule::Fixed(JointId::Hips),
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::RightLowerLeg,
        parent: ParentRule::Fixed(JointId::RightUpperLeg),
        required: true,
    },
    CanonicalJointSpec {
        joint: JointId::RightFoot,
        parent: ParentRule::Fixed(JointId::RightLowerLeg),
        required: true,
    },
];

const fn optional_joint(joint: JointId, parent: ParentRule) -> CanonicalJointSpec {
    CanonicalJointSpec {
        joint,
        parent,
        required: false,
    }
}

/// Lists optional canonical joints and their parent rules.
pub const OPTIONAL_JOINTS: [CanonicalJointSpec; 38] = [
    optional_joint(JointId::UpperChest, ParentRule::Fixed(JointId::Chest)),
    optional_joint(
        JointId::LeftShoulder,
        ParentRule::PreferOptional {
            optional: JointId::UpperChest,
            fallback: JointId::Chest,
        },
    ),
    optional_joint(
        JointId::RightShoulder,
        ParentRule::PreferOptional {
            optional: JointId::UpperChest,
            fallback: JointId::Chest,
        },
    ),
    optional_joint(JointId::LeftToes, ParentRule::Fixed(JointId::LeftFoot)),
    optional_joint(JointId::RightToes, ParentRule::Fixed(JointId::RightFoot)),
    optional_joint(JointId::LeftEye, ParentRule::Fixed(JointId::Head)),
    optional_joint(JointId::RightEye, ParentRule::Fixed(JointId::Head)),
    optional_joint(JointId::Jaw, ParentRule::Fixed(JointId::Head)),
    optional_joint(
        JointId::LeftThumbMetacarpal,
        ParentRule::Fixed(JointId::LeftHand),
    ),
    optional_joint(
        JointId::LeftThumbProximal,
        ParentRule::Fixed(JointId::LeftThumbMetacarpal),
    ),
    optional_joint(
        JointId::LeftThumbDistal,
        ParentRule::Fixed(JointId::LeftThumbProximal),
    ),
    optional_joint(
        JointId::LeftIndexProximal,
        ParentRule::Fixed(JointId::LeftHand),
    ),
    optional_joint(
        JointId::LeftIndexIntermediate,
        ParentRule::Fixed(JointId::LeftIndexProximal),
    ),
    optional_joint(
        JointId::LeftIndexDistal,
        ParentRule::Fixed(JointId::LeftIndexIntermediate),
    ),
    optional_joint(
        JointId::LeftMiddleProximal,
        ParentRule::Fixed(JointId::LeftHand),
    ),
    optional_joint(
        JointId::LeftMiddleIntermediate,
        ParentRule::Fixed(JointId::LeftMiddleProximal),
    ),
    optional_joint(
        JointId::LeftMiddleDistal,
        ParentRule::Fixed(JointId::LeftMiddleIntermediate),
    ),
    optional_joint(
        JointId::LeftRingProximal,
        ParentRule::Fixed(JointId::LeftHand),
    ),
    optional_joint(
        JointId::LeftRingIntermediate,
        ParentRule::Fixed(JointId::LeftRingProximal),
    ),
    optional_joint(
        JointId::LeftRingDistal,
        ParentRule::Fixed(JointId::LeftRingIntermediate),
    ),
    optional_joint(
        JointId::LeftLittleProximal,
        ParentRule::Fixed(JointId::LeftHand),
    ),
    optional_joint(
        JointId::LeftLittleIntermediate,
        ParentRule::Fixed(JointId::LeftLittleProximal),
    ),
    optional_joint(
        JointId::LeftLittleDistal,
        ParentRule::Fixed(JointId::LeftLittleIntermediate),
    ),
    optional_joint(
        JointId::RightThumbMetacarpal,
        ParentRule::Fixed(JointId::RightHand),
    ),
    optional_joint(
        JointId::RightThumbProximal,
        ParentRule::Fixed(JointId::RightThumbMetacarpal),
    ),
    optional_joint(
        JointId::RightThumbDistal,
        ParentRule::Fixed(JointId::RightThumbProximal),
    ),
    optional_joint(
        JointId::RightIndexProximal,
        ParentRule::Fixed(JointId::RightHand),
    ),
    optional_joint(
        JointId::RightIndexIntermediate,
        ParentRule::Fixed(JointId::RightIndexProximal),
    ),
    optional_joint(
        JointId::RightIndexDistal,
        ParentRule::Fixed(JointId::RightIndexIntermediate),
    ),
    optional_joint(
        JointId::RightMiddleProximal,
        ParentRule::Fixed(JointId::RightHand),
    ),
    optional_joint(
        JointId::RightMiddleIntermediate,
        ParentRule::Fixed(JointId::RightMiddleProximal),
    ),
    optional_joint(
        JointId::RightMiddleDistal,
        ParentRule::Fixed(JointId::RightMiddleIntermediate),
    ),
    optional_joint(
        JointId::RightRingProximal,
        ParentRule::Fixed(JointId::RightHand),
    ),
    optional_joint(
        JointId::RightRingIntermediate,
        ParentRule::Fixed(JointId::RightRingProximal),
    ),
    optional_joint(
        JointId::RightRingDistal,
        ParentRule::Fixed(JointId::RightRingIntermediate),
    ),
    optional_joint(
        JointId::RightLittleProximal,
        ParentRule::Fixed(JointId::RightHand),
    ),
    optional_joint(
        JointId::RightLittleIntermediate,
        ParentRule::Fixed(JointId::RightLittleProximal),
    ),
    optional_joint(
        JointId::RightLittleDistal,
        ParentRule::Fixed(JointId::RightLittleIntermediate),
    ),
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn canonical_table_contains_each_required_joint_once() {
        let joints = REQUIRED_JOINTS
            .iter()
            .map(|spec| spec.joint)
            .collect::<BTreeSet<_>>();
        assert_eq!(joints.len(), REQUIRED_JOINTS.len());
        assert!(REQUIRED_JOINTS.iter().all(|spec| spec.required));

        let parent_of = |joint| {
            REQUIRED_JOINTS
                .iter()
                .find(|spec| spec.joint == joint)
                .map(|spec| spec.parent)
        };
        assert_eq!(parent_of(JointId::Hips), Some(ParentRule::Root));
        assert_eq!(
            parent_of(JointId::Neck),
            Some(ParentRule::PreferOptional {
                optional: JointId::UpperChest,
                fallback: JointId::Chest,
            })
        );
    }

    #[test]
    fn optional_table_contains_each_optional_joint_once() {
        let joints = OPTIONAL_JOINTS
            .iter()
            .map(|spec| spec.joint)
            .collect::<BTreeSet<_>>();
        assert_eq!(joints.len(), OPTIONAL_JOINTS.len());
        assert!(OPTIONAL_JOINTS.iter().all(|spec| !spec.required));
        assert!(
            joints.is_disjoint(
                &REQUIRED_JOINTS
                    .iter()
                    .map(|spec| spec.joint)
                    .collect::<BTreeSet<_>>()
            )
        );
    }
}
