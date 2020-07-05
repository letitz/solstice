//! Utility module for writing length-prefixed values, the length of which is
//! unknown before the value is encoded.

use std::convert::TryFrom;

use crate::proto::u32::{encode_u32, U32_BYTE_LEN};

/// Encodes a length prefix in a buffer.
pub struct Prefixer {
    /// The position of the length prefix in the target buffer.
    position: usize,
}

impl Prefixer {
    /// Reserves space for the length prefix at the end of the given buffer.
    ///
    /// Returns a prefixer for writing the length later.
    pub fn new(buffer: &mut Vec<u8>) -> Prefixer {
        // Remember where we were.
        let result = Prefixer {
            position: buffer.len(),
        };
        // Reserve enough bytes to write the prefix into later.
        buffer.extend_from_slice(&[0; U32_BYTE_LEN]);
        result
    }

    /// Writes the length prefix into the given buffer in the reserved space.
    ///
    /// The given buffer must be the same one passed to new(), and should not
    /// have been truncated since then.
    ///
    /// Panics if the buffer is not large enough to store the prefix.
    ///
    /// Returns `Err(length)` if `length`, the length of the suffix, is too
    /// large to store in the reserved space.
    pub fn finalize(self, buffer: &mut Vec<u8>) -> Result<(), usize> {
        // The position at which the value should have been encoded.
        let value_position = self.position + U32_BYTE_LEN;
        assert!(buffer.len() >= value_position);

        // Calculate the value's length, check it is not too large.
        let length = buffer.len() - value_position;
        let length_u32 = u32::try_from(length).map_err(|_| length)?;

        // Write the length prefix into the reserved space.
        let length_bytes = encode_u32(length_u32);
        buffer[self.position..value_position].copy_from_slice(&length_bytes);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Prefixer;

    use std::convert::TryInto;

    use crate::proto::u32::{decode_u32, U32_BYTE_LEN};

    #[test]
    fn new_reserves_space() {
        let mut buffer = vec![13];

        Prefixer::new(&mut buffer);

        assert_eq!(buffer.len(), U32_BYTE_LEN + 1);
        assert_eq!(buffer[0], 13);
    }

    #[test]
    fn finalize_empty() {
        let mut buffer = vec![13];

        Prefixer::new(&mut buffer).finalize(&mut buffer).unwrap();

        assert_eq!(buffer.len(), U32_BYTE_LEN + 1);
        let array: [u8; U32_BYTE_LEN] = buffer[1..].try_into().unwrap();
        assert_eq!(decode_u32(array), 0);
    }

    #[test]
    fn finalize_ok() {
        let mut buffer = vec![13];

        let prefixer = Prefixer::new(&mut buffer);

        buffer.extend_from_slice(&[0; 42]);

        prefixer.finalize(&mut buffer).unwrap();

        // 1 junk prefix byte, length prefix, 42 bytes of value.
        assert_eq!(buffer.len(), U32_BYTE_LEN + 43);
        let prefix = &buffer[1..U32_BYTE_LEN + 1];
        let array: [u8; U32_BYTE_LEN] = prefix.try_into().unwrap();
        assert_eq!(decode_u32(array), 42);
    }

    #[test]
    #[should_panic]
    fn finalize_truncated() {
        let mut buffer = vec![13];

        let prefixer = Prefixer::new(&mut buffer);

        buffer = vec![];

        // Explodes.
        let _ = prefixer.finalize(&mut buffer);
    }
}
