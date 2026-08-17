//! This module owns captured-image packets and camera metadata.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{ContractError, FiniteF32, MonotonicTime, Validate, validate_stream_order};

/// Identifies the origin of a packet's `source_time`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimestampQuality {
    /// The backend supplied source time on the runtime monotonic clock.
    Source,
    /// `source_time` uses the application receipt time.
    Receipt,
}

/// Identifies the image format in a [`FramePacket`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PixelFormat {
    Rgb8,
    Mjpg,
    Yuyv,
}

/// Describes the camera mode delivered with a frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraMode {
    pub pixel_format: PixelFormat,
    /// Stores the image width in pixels.
    pub width: u32,
    /// Stores the image height in pixels.
    pub height: u32,
    /// Stores the row stride in bytes for a packed image.
    pub stride: u32,
    /// Stores the nominal frame interval in nanoseconds.
    pub nominal_frame_interval_ns: u64,
}

/// Records backend-defined camera control values with a frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CameraControlsSnapshot {
    pub values: BTreeMap<String, FiniteF32>,
}

/// Contains one captured image and its runtime-clock metadata.
// Validate external data before use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct FramePacket {
    pub camera_id: String,
    /// Stores the increasing sequence number for this camera stream.
    pub sequence: u64,
    /// Stores the best available source time on the runtime monotonic clock.
    pub source_time: MonotonicTime,
    /// Stores the application receipt time on the runtime monotonic clock.
    pub receipt_time: MonotonicTime,
    pub timestamp_quality: TimestampQuality,
    pub mode: CameraMode,
    /// Contains the encoded or packed bytes described by `mode`.
    pub image_buffer: Vec<u8>,
    pub camera_controls: CameraControlsSnapshot,
}

impl Serialize for FramePacket {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        FramePacket::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for FramePacket {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let packet = FramePacket::deserialize(deserializer)?;
        packet.validate().map_err(de::Error::custom)?;
        Ok(packet)
    }
}

impl FramePacket {
    /// Checks this packet against the prior packet from one stream.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::InvalidSequence`] if the sequence does not
    /// increase. Returns [`ContractError::InvalidTimeOrder`] if source time
    /// moves backward.
    pub fn validate_after(&self, previous: &Self) -> Result<(), ContractError> {
        validate_stream_order(
            previous.sequence,
            previous.source_time,
            self.sequence,
            self.source_time,
        )
    }
}

impl Validate for FramePacket {
    fn validate(&self) -> Result<(), ContractError> {
        // 1. Check packet identity and time.
        if self.camera_id.trim().is_empty() {
            return Err(ContractError::EmptyField { field: "camera_id" });
        }
        if self.source_time > self.receipt_time {
            return Err(ContractError::InvalidTimeOrder {
                earlier: "source_time",
                later: "receipt_time",
            });
        }
        // 2. Check the delivered camera mode.
        if self.mode.width == 0
            || self.mode.height == 0
            || self.mode.nominal_frame_interval_ns == 0
            || self.image_buffer.is_empty()
        {
            return Err(ContractError::InvalidImageLayout);
        }

        // 3. Check the packed image layout.
        if let Some(bytes_per_pixel) = match self.mode.pixel_format {
            PixelFormat::Rgb8 => Some(3_u32),
            PixelFormat::Yuyv => Some(2_u32),
            PixelFormat::Mjpg => None,
        } {
            let minimum_stride = self.mode.width.checked_mul(bytes_per_pixel);
            let minimum_len = usize::try_from(self.mode.stride).ok().and_then(|stride| {
                usize::try_from(self.mode.height)
                    .ok()
                    .and_then(|height| stride.checked_mul(height))
            });
            if minimum_stride.is_none_or(|minimum| self.mode.stride < minimum)
                || minimum_len.is_none_or(|minimum| self.image_buffer.len() < minimum)
            {
                return Err(ContractError::InvalidImageLayout);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(pixel_format: PixelFormat, stride: u32, image_buffer: Vec<u8>) -> FramePacket {
        FramePacket {
            camera_id: "camera-0".into(),
            sequence: 1,
            source_time: MonotonicTime::from_nanos(10),
            receipt_time: MonotonicTime::from_nanos(20),
            timestamp_quality: TimestampQuality::Source,
            mode: CameraMode {
                pixel_format,
                width: 2,
                height: 2,
                stride,
                nominal_frame_interval_ns: 1,
            },
            image_buffer,
            camera_controls: CameraControlsSnapshot::default(),
        }
    }

    #[test]
    fn packet_schema_round_trip() {
        let expected = packet(PixelFormat::Rgb8, 6, vec![0; 12]);
        expected.validate().unwrap();
        let encoded = serde_json::to_vec(&expected).unwrap();
        let decoded: FramePacket = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn validates_compressed_and_raw_layouts_separately() {
        assert!(
            packet(PixelFormat::Mjpg, 0, vec![1, 2, 3])
                .validate()
                .is_ok()
        );
        assert!(packet(PixelFormat::Rgb8, 2, vec![0; 4]).validate().is_err());
        assert!(packet(PixelFormat::Yuyv, 4, vec![0; 8]).validate().is_ok());
    }

    #[test]
    fn rejects_packet_time_reversal() {
        let mut value = packet(PixelFormat::Mjpg, 0, vec![1]);
        value.source_time = MonotonicTime::from_nanos(21);
        assert!(value.validate().is_err());
        let encoded = serde_json::to_value(value).unwrap();
        assert!(serde_json::from_value::<FramePacket>(encoded).is_err());
    }
}
