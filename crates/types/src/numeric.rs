//! This module owns finite and range-checked floating-point values.

use serde::{Deserialize, Deserializer, Serialize, de};

use crate::ContractError;

/// Stores a finite 32-bit floating-point value.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct FiniteF32(f32);

impl FiniteF32 {
    /// Creates a finite value.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::NonFinite`] if `value` is NaN or infinite.
    pub fn new(value: f32) -> Result<Self, ContractError> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(ContractError::NonFinite { field: "value" })
        }
    }

    pub const fn get(self) -> f32 {
        self.0
    }
}

impl TryFrom<f32> for FiniteF32 {
    type Error = ContractError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for FiniteF32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f32::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Stores a finite value that cannot be negative.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct NonNegativeF32(FiniteF32);

impl NonNegativeF32 {
    /// Creates a finite value greater than or equal to zero.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::NonFinite`] for NaN or infinity. Returns
    /// [`ContractError::OutOfRange`] for a negative value.
    pub fn new(value: f32) -> Result<Self, ContractError> {
        let value = FiniteF32::new(value)?;
        if value.get() >= 0.0 {
            Ok(Self(value))
        } else {
            Err(ContractError::OutOfRange { field: "value" })
        }
    }

    pub const fn get(self) -> f32 {
        self.0.get()
    }
}

impl<'de> Deserialize<'de> for NonNegativeF32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f32::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Stores a finite value in the inclusive range from zero through one.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct UnitInterval(FiniteF32);

impl UnitInterval {
    /// Creates a finite value in the inclusive unit interval.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::NonFinite`] for NaN or infinity. Returns
    /// [`ContractError::OutOfRange`] for a value outside the unit interval.
    pub fn new(value: f32) -> Result<Self, ContractError> {
        let value = FiniteF32::new(value)?;
        if (0.0..=1.0).contains(&value.get()) {
            Ok(Self(value))
        } else {
            Err(ContractError::OutOfRange { field: "value" })
        }
    }

    pub const fn get(self) -> f32 {
        self.0.get()
    }
}

impl<'de> Deserialize<'de> for UnitInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f32::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use serde::de::{IntoDeserializer, value::Error};

    use super::*;

    fn decode<T: for<'de> Deserialize<'de>>(value: f32) -> Result<T, Error> {
        T::deserialize(IntoDeserializer::<Error>::into_deserializer(value))
    }

    #[test]
    fn deserialization_rejects_out_of_range_values() {
        assert!(decode::<FiniteF32>(f32::NAN).is_err());
        assert!(decode::<FiniteF32>(f32::INFINITY).is_err());
        assert!(decode::<NonNegativeF32>(-0.1).is_err());
        assert!(decode::<UnitInterval>(1.1).is_err());
        assert!(decode::<UnitInterval>(1.0).is_ok());
    }
}
