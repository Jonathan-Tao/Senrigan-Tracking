//! This module owns model-package identity, compatibility, and release labels.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{ContractError, NonNegativeF32, SchemaVersion, Validate};

/// Records the reviewed distribution status of a model package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelReleaseClass {
    /// The package may enter a public bundle.
    BundleApproved,
    /// The package is limited to permitted local research.
    LocalResearchOnly,
    /// The package terms have not been reviewed.
    Unreviewed,
}

/// Names one validated execution-provider and numeric-precision pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderPrecision {
    pub provider: String,
    pub precision: String,
}

/// Describes one model package that the loader checks before use.
///
/// Deserialization rejects unsupported schema versions and incomplete package
/// identity or compatibility data.
// Validate external data before use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ModelManifest {
    pub schema_version: SchemaVersion,
    pub model_id: String,
    pub version: String,
    /// Maps package-relative file names to recorded digest strings.
    pub file_hashes: BTreeMap<String, String>,
    /// Identifies the package notice or another source and license reference.
    pub source_and_license_reference: String,
    /// Identifies the preprocessing rules and named tensor contract.
    pub preprocessing_and_tensor_schema: String,
    pub canonical_adapter_version: String,
    pub supported_providers_and_precision: Vec<ProviderPrecision>,
    /// Stores the observed minimum device memory in bytes, if measured.
    pub minimum_memory_observed_bytes: Option<u64>,
    /// Maps each named measurement and unit to its tolerance.
    pub numerical_tolerances: BTreeMap<String, NonNegativeF32>,
    pub release_class: ModelReleaseClass,
}

impl Validate for ModelManifest {
    fn validate(&self) -> Result<(), ContractError> {
        // 1. Check package identity and provenance.
        for (field, value) in [
            ("model_id", self.model_id.as_str()),
            ("version", self.version.as_str()),
            (
                "source_and_license_reference",
                self.source_and_license_reference.as_str(),
            ),
            (
                "preprocessing_and_tensor_schema",
                self.preprocessing_and_tensor_schema.as_str(),
            ),
            (
                "canonical_adapter_version",
                self.canonical_adapter_version.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(ContractError::EmptyField { field });
            }
        }
        // 2. Check the package file inventory.
        if self.file_hashes.is_empty() {
            return Err(ContractError::EmptyField {
                field: "file_hashes",
            });
        }
        for (path, hash) in &self.file_hashes {
            if path.trim().is_empty() || hash.trim().is_empty() {
                return Err(ContractError::EmptyField {
                    field: "file_hashes entry",
                });
            }
        }
        // 3. Check runtime compatibility.
        if self.supported_providers_and_precision.is_empty() {
            return Err(ContractError::EmptyField {
                field: "supported_providers_and_precision",
            });
        }
        for placement in &self.supported_providers_and_precision {
            if placement.provider.trim().is_empty() || placement.precision.trim().is_empty() {
                return Err(ContractError::EmptyField {
                    field: "provider or precision",
                });
            }
        }
        Ok(())
    }
}

impl Serialize for ModelManifest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ModelManifest::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for ModelManifest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let manifest = ModelManifest::deserialize(deserializer)?;
        manifest.validate().map_err(de::Error::custom)?;
        Ok(manifest)
    }
}

/// Identifies the selected model package, provider, device, and precision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferencePlacement {
    pub provider: String,
    pub device_id: String,
    pub precision_mode: String,
    pub model_package_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> ModelManifest {
        ModelManifest {
            schema_version: SchemaVersion::current(),
            model_id: "fixture-body".into(),
            version: "1.0.0".into(),
            file_hashes: BTreeMap::from([("model.onnx".into(), "sha256:0123".into())]),
            source_and_license_reference: "NOTICE.txt".into(),
            preprocessing_and_tensor_schema: "adapter.md".into(),
            canonical_adapter_version: "fixture-v1".into(),
            supported_providers_and_precision: vec![ProviderPrecision {
                provider: "CUDA".into(),
                precision: "fp16".into(),
            }],
            minimum_memory_observed_bytes: None,
            numerical_tolerances: BTreeMap::from([(
                "joint_position_m".into(),
                NonNegativeF32::new(0.001).unwrap(),
            )]),
            release_class: ModelReleaseClass::Unreviewed,
        }
    }

    #[test]
    fn manifest_schema_round_trip() {
        let expected = manifest();
        expected.validate().unwrap();
        let encoded = serde_json::to_vec(&expected).unwrap();
        let decoded: ModelManifest = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn manifest_rejects_an_incomplete_package() {
        let mut value = manifest();
        value.file_hashes.clear();
        let encoded = serde_json::to_value(value).unwrap();
        assert!(serde_json::from_value::<ModelManifest>(encoded).is_err());
    }

    #[test]
    fn manifest_rejects_an_unknown_schema_version() {
        let mut value = serde_json::to_value(manifest()).unwrap();
        value["schema_version"] = serde_json::json!(999);
        assert!(serde_json::from_value::<ModelManifest>(value).is_err());
    }
}
