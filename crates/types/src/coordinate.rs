//! This module owns coordinate-frame markers and finite rigid transforms.
//!
//! Point coordinates and transform translations use meters. Quaternions use
//! `xyzw` component order.

use std::{fmt, marker::PhantomData};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::ContractError;

/// `C` marks the camera optical frame with OpenCV axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C {}
/// `W` marks the calibrated site frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W {}
/// `S` marks the canonical source-skeleton frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S {}
/// `A` marks the imported avatar rest frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A {}

/// Names the positive direction of one coordinate axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisDirection {
    Right,
    Left,
    Up,
    Down,
    Forward,
    Backward,
}

/// Describes the positive axes and handedness of a coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameConvention {
    pub x: AxisDirection,
    pub y: AxisDirection,
    pub z: AxisDirection,
    pub right_handed: bool,
}

/// Defines OpenCV camera coordinates as +X right, +Y down, and +Z forward.
pub const OPENCV_CAMERA_FRAME: FrameConvention = FrameConvention {
    x: AxisDirection::Right,
    y: AxisDirection::Down,
    z: AxisDirection::Forward,
    right_handed: true,
};

/// Defines site coordinates as +X left, +Y up, and +Z site forward.
///
/// Right-handedness with +Y up and +Z forward forces +X to point left.
pub const SITE_FRAME: FrameConvention = FrameConvention {
    x: AxisDirection::Left,
    y: AxisDirection::Up,
    z: AxisDirection::Forward,
    right_handed: true,
};

/// Defines avatar adapter coordinates as +X right, +Y up, and +Z backward.
pub const GLTF_AVATAR_FRAME: FrameConvention = FrameConvention {
    x: AxisDirection::Right,
    y: AxisDirection::Up,
    z: AxisDirection::Backward,
    right_handed: true,
};

/// Stores a finite vector on the axes of frame `F`.
///
/// The field that owns the vector defines its unit.
#[derive(PartialEq)]
pub struct Vector3<F> {
    xyz: [f32; 3],
    frame: PhantomData<F>,
}

impl<F> Copy for Vector3<F> {}

impl<F> Clone for Vector3<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> Vector3<F> {
    /// Creates a vector on the axes of frame `F`.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::NonFinite`] if a component is not finite.
    pub fn try_new(x: f32, y: f32, z: f32) -> Result<Self, ContractError> {
        if [x, y, z].into_iter().all(f32::is_finite) {
            Ok(Self {
                xyz: [x, y, z],
                frame: PhantomData,
            })
        } else {
            Err(ContractError::NonFinite { field: "vector" })
        }
    }

    /// Returns the `[x, y, z]` components.
    pub const fn to_array(self) -> [f32; 3] {
        self.xyz
    }

    fn add(self, rhs: Self) -> Self {
        Self {
            xyz: [
                self.xyz[0] + rhs.xyz[0],
                self.xyz[1] + rhs.xyz[1],
                self.xyz[2] + rhs.xyz[2],
            ],
            frame: PhantomData,
        }
    }

    fn neg(self) -> Self {
        Self {
            xyz: [-self.xyz[0], -self.xyz[1], -self.xyz[2]],
            frame: PhantomData,
        }
    }
}

impl<F> fmt::Debug for Vector3<F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("Vector3").field(&self.xyz).finish()
    }
}

impl<F> Serialize for Vector3<F> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        self.xyz.serialize(serializer)
    }
}

impl<'de, F> Deserialize<'de> for Vector3<F> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let [x, y, z] = <[f32; 3]>::deserialize(deserializer)?;
        Self::try_new(x, y, z).map_err(de::Error::custom)
    }
}

/// Stores a finite point in frame `F`, in meters.
#[derive(PartialEq)]
pub struct Point3<F> {
    xyz: [f32; 3],
    frame: PhantomData<F>,
}

impl<F> Copy for Point3<F> {}

impl<F> Clone for Point3<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> Point3<F> {
    /// Creates a point in frame `F`, in meters.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::NonFinite`] if a component is not finite.
    pub fn try_new(x: f32, y: f32, z: f32) -> Result<Self, ContractError> {
        if [x, y, z].into_iter().all(f32::is_finite) {
            Ok(Self {
                xyz: [x, y, z],
                frame: PhantomData,
            })
        } else {
            Err(ContractError::NonFinite { field: "point" })
        }
    }

    /// Returns the `[x, y, z]` components in meters.
    pub const fn to_array(self) -> [f32; 3] {
        self.xyz
    }
}

impl<F> fmt::Debug for Point3<F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("Point3").field(&self.xyz).finish()
    }
}

impl<F> Serialize for Point3<F> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        self.xyz.serialize(serializer)
    }
}

impl<'de, F> Deserialize<'de> for Point3<F> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let [x, y, z] = <[f32; 3]>::deserialize(deserializer)?;
        Self::try_new(x, y, z).map_err(de::Error::custom)
    }
}

/// Stores a finite unit quaternion in internal `xyzw` component order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitQuaternion {
    xyzw: [f32; 4],
}

impl UnitQuaternion {
    const NORM_TOLERANCE: f32 = 1.0e-5;

    /// Returns the identity rotation.
    pub fn identity() -> Self {
        Self {
            xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Creates a finite unit quaternion from `xyzw` components.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::InvalidQuaternion`] if a component is not
    /// finite or the quaternion does not have unit length.
    pub fn try_from_xyzw(xyzw: [f32; 4]) -> Result<Self, ContractError> {
        if !xyzw.into_iter().all(f32::is_finite) {
            return Err(ContractError::InvalidQuaternion);
        }
        let norm_squared = xyzw.into_iter().map(|value| value * value).sum::<f32>();
        if (norm_squared - 1.0).abs() > Self::NORM_TOLERANCE {
            return Err(ContractError::InvalidQuaternion);
        }
        Ok(Self { xyzw })
    }

    /// Returns the quaternion in internal `xyzw` component order.
    pub const fn to_xyzw(self) -> [f32; 4] {
        self.xyzw
    }

    /// Returns the inverse rotation.
    pub fn inverse(self) -> Self {
        let [x, y, z, w] = self.xyzw;
        Self {
            xyzw: [-x, -y, -z, w],
        }
    }

    /// Applies `rhs`, then `self`, with the Hamilton product.
    pub fn hamilton(self, rhs: Self) -> Self {
        let [x1, y1, z1, w1] = self.xyzw;
        let [x2, y2, z2, w2] = rhs.xyzw;
        let raw = [
            w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
            w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
            w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
            w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
        ];
        Self::normalize_internal(raw)
    }

    fn rotate<F, G>(self, vector: Vector3<F>) -> Vector3<G> {
        let [x, y, z, w] = self.xyzw;
        let [vx, vy, vz] = vector.xyz;
        let tx = 2.0 * (y * vz - z * vy);
        let ty = 2.0 * (z * vx - x * vz);
        let tz = 2.0 * (x * vy - y * vx);
        Vector3 {
            xyz: [
                vx + w * tx + (y * tz - z * ty),
                vy + w * ty + (z * tx - x * tz),
                vz + w * tz + (x * ty - y * tx),
            ],
            frame: PhantomData,
        }
    }

    fn normalize_internal(raw: [f32; 4]) -> Self {
        let inverse_norm = raw
            .into_iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt()
            .recip();
        Self {
            xyzw: raw.map(|value| value * inverse_norm),
        }
    }
}

impl Serialize for UnitQuaternion {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        self.xyzw.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for UnitQuaternion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let xyzw = <[f32; 4]>::deserialize(deserializer)?;
        Self::try_from_xyzw(xyzw).map_err(de::Error::custom)
    }
}

/// Maps values from frame `From` into frame `To`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Transform<To, From> {
    /// Rotates values from `From` axes to `To` axes.
    pub rotation: UnitQuaternion,
    /// Places the `From` origin in `To`, in meters.
    pub translation: Vector3<To>,
    // Omit frame markers from wire data.
    #[serde(skip)]
    from: PhantomData<From>,
}

impl<To, From> Transform<To, From> {
    /// Creates `T_To_From` from its rotation and translation.
    pub fn new(rotation: UnitQuaternion, translation: Vector3<To>) -> Self {
        Self {
            rotation,
            translation,
            from: PhantomData,
        }
    }

    /// Maps a point from `From` into `To`.
    pub fn transform_point(&self, point: Point3<From>) -> Point3<To> {
        let source_vector = Vector3::<From> {
            xyz: point.xyz,
            frame: PhantomData,
        };
        let rotated: Vector3<To> = self.rotation.rotate(source_vector);
        Point3 {
            xyz: rotated.add(self.translation).xyz,
            frame: PhantomData,
        }
    }

    /// Compose `T_To_From` with `T_From_Source` to produce `T_To_Source`.
    pub fn compose<Source>(&self, rhs: &Transform<From, Source>) -> Transform<To, Source> {
        let rhs_translation_in_to: Vector3<To> = self.rotation.rotate(rhs.translation);
        Transform::new(
            self.rotation.hamilton(rhs.rotation),
            rhs_translation_in_to.add(self.translation),
        )
    }

    /// Returns `T_From_To`.
    pub fn inverse(&self) -> Transform<From, To> {
        let rotation = self.rotation.inverse();
        let translation: Vector3<From> = rotation.rotate(self.translation.neg());
        Transform::new(rotation, translation)
    }
}

impl<F> Transform<F, F> {
    /// Returns the identity transform in frame `F`.
    pub fn identity() -> Self {
        Self::new(
            UnitQuaternion::identity(),
            Vector3 {
                xyz: [0.0, 0.0, 0.0],
                frame: PhantomData,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_1_SQRT_2;

    use super::*;

    fn assert_point_close<F>(actual: Point3<F>, expected: [f32; 3]) {
        for (actual, expected) in actual.to_array().into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1.0e-5, "{actual} != {expected}");
        }
    }

    #[test]
    fn composes_destination_from_source_transforms() {
        let t_w_c = Transform::<W, C>::new(
            UnitQuaternion::identity(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        );
        let t_c_s = Transform::<C, S>::new(
            UnitQuaternion::identity(),
            Vector3::try_new(0.0, 2.0, 0.0).unwrap(),
        );
        let t_w_s = t_w_c.compose(&t_c_s);
        let point_s = Point3::<S>::try_new(0.0, 0.0, 3.0).unwrap();
        assert_point_close(t_w_s.transform_point(point_s), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn inverse_restores_a_point() {
        let rotation =
            UnitQuaternion::try_from_xyzw([0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2]).unwrap();
        let transform = Transform::<W, C>::new(rotation, Vector3::try_new(2.0, -1.0, 0.5).unwrap());
        let point = Point3::<C>::try_new(1.0, 3.0, -2.0).unwrap();
        let restored = transform
            .inverse()
            .transform_point(transform.transform_point(point));
        assert_point_close(restored, point.to_array());
    }

    #[test]
    fn uses_xyzw_quaternions_and_hamilton_rotation() {
        let rotation =
            UnitQuaternion::try_from_xyzw([0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2]).unwrap();
        let t_w_c = Transform::<W, C>::new(rotation, Vector3::try_new(0.0, 0.0, 0.0).unwrap());
        let point_c = Point3::<C>::try_new(1.0, 0.0, 0.0).unwrap();
        assert_point_close(t_w_c.transform_point(point_c), [0.0, 1.0, 0.0]);
    }

    #[test]
    fn rejects_quaternions_that_are_not_unit_length() {
        assert!((f32::NAN - 1.0).abs().is_nan(), "the norm test is vacuous");
        assert!(UnitQuaternion::try_from_xyzw([f32::NAN, 0.0, 0.0, 1.0]).is_err());
        assert!(UnitQuaternion::try_from_xyzw([0.0, 0.0, 0.0, 0.0]).is_err());
        assert!(UnitQuaternion::try_from_xyzw([0.0, 0.0, 0.0, 2.0]).is_err());
    }

    #[test]
    fn deserialization_rejects_invalid_quaternions() {
        let result = serde_json::from_str::<UnitQuaternion>("[0.0,0.0,0.0,2.0]");
        assert!(result.is_err());
    }
}
