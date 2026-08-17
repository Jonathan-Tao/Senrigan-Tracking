//! This module owns the accepted persisted-schema version.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::ContractError;

/// Identifies the schema version emitted and accepted by this build.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Stores a checked persisted-schema version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SchemaVersion(u32);

impl SchemaVersion {
    /// Returns the schema version emitted by this build.
    pub const fn current() -> Self {
        Self(CURRENT_SCHEMA_VERSION)
    }

    /// Checks a schema version read from persisted data.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::UnsupportedSchemaVersion`] if this build cannot
    /// read `value`.
    pub fn from_raw(value: u32) -> Result<Self, ContractError> {
        if value == CURRENT_SCHEMA_VERSION {
            Ok(Self(value))
        } else {
            Err(ContractError::UnsupportedSchemaVersion {
                found: value,
                expected: CURRENT_SCHEMA_VERSION,
            })
        }
    }

    /// Returns the checked schema version number.
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Default for SchemaVersion {
    fn default() -> Self {
        Self::current()
    }
}

impl Serialize for SchemaVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.0)
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        Self::from_raw(value).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_unsupported_version() {
        assert!(SchemaVersion::from_raw(CURRENT_SCHEMA_VERSION + 1).is_err());
        assert!(
            serde_json::from_str::<SchemaVersion>(&format!("{}", CURRENT_SCHEMA_VERSION + 1))
                .is_err()
        );
    }
}
