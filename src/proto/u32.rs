//! This module provides base primitives for encoding and decoding u32 values.
//!
//! It mostly centralizes the knowledge that the protocol uses little-endian
//! representation for u32 values.

/// Length of an encoded 32-bit integer in bytes.
pub const U32_BYTE_LEN: usize = 4;

/// Returns the byte representatio of the given integer value.
pub fn encode_u32(value: u32) -> [u8; U32_BYTE_LEN] {
  value.to_le_bytes()
}

/// Returns the integer value corresponding to the given bytes.
pub fn decode_u32(bytes: [u8; U32_BYTE_LEN]) -> u32 {
  u32::from_le_bytes(bytes)
}
