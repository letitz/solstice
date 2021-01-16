//! Utility module for writing length-prefixed values, the length of which is
//! unknown before the value is encoded.

use std::convert::TryFrom;

use bytes::BytesMut;

use crate::proto::u32::{encode_u32, U32_BYTE_LEN};

/// Helper for writing length-prefixed values into buffers, without having to
/// know the length ahead of encoding time.
#[derive(Debug)]
pub struct Prefixer<'a> {
  /// The prefix buffer.
  ///
  /// The length of the suffix buffer is written to the end of this buffer
  /// when the prefixer is finalized.
  ///
  /// Contains any bytes with which this prefixer was constructed.
  prefix: &'a mut BytesMut,

  /// The suffix buffer.
  ///
  /// This is the buffer into which data is written before finalization.
  suffix: BytesMut,
}

impl Prefixer<'_> {
  /// Constructs a prefixer for easily appending a length prefixed value to
  /// the given buffer.
  pub fn new<'a>(buffer: &'a mut BytesMut) -> Prefixer<'a> {
    // Reserve some space fot the prefix, but don't write it yet.
    buffer.reserve(U32_BYTE_LEN);

    // Split off the suffix, into which bytes will be written.
    let suffix = buffer.split_off(buffer.len() + U32_BYTE_LEN);

    Prefixer {
      prefix: buffer,
      suffix: suffix,
    }
  }

  /// Returns a reference to the buffer into which data is written.
  pub fn suffix(&self) -> &BytesMut {
    &self.suffix
  }

  /// Returns a mutable reference to a buffer into which data can be written.
  pub fn suffix_mut(&mut self) -> &mut BytesMut {
    &mut self.suffix
  }

  /// Returns a buffer containing the original data passed at construction
  /// time, to which a length-prefixed value is appended. The value itself is
  /// the data written into the buffer returned by `get_mut()`.
  ///
  /// Returns `Ok(length)` if successful, in which case the length of the
  /// suffix is `length`.
  ///
  /// Returns `Err(self)` if the length of the suffix is too large to store as
  /// a prefix.
  pub fn finalize(self) -> Result<u32, Self> {
    // Check that the suffix's length is not too large.
    let length = self.suffix.len();
    let length_u32 = match u32::try_from(length) {
      Ok(value) => value,
      Err(_) => return Err(self),
    };

    // Write the prefix.
    self.prefix.extend_from_slice(&encode_u32(length_u32));

    // Join the prefix and suffix back again. Because `self.prefix` is
    // private, we are sure that this is O(1).
    self.prefix.unsplit(self.suffix);

    Ok(length_u32)
  }
}

#[cfg(test)]
mod tests {
  use super::Prefixer;

  use std::convert::TryInto;

  use bytes::{BufMut, BytesMut};

  use crate::proto::u32::{decode_u32, U32_BYTE_LEN};

  #[test]
  fn finalize_empty() {
    let mut buffer = BytesMut::new();
    buffer.put_u8(13);

    Prefixer::new(&mut buffer).finalize().unwrap();

    assert_eq!(buffer.len(), U32_BYTE_LEN + 1);
    let array: [u8; U32_BYTE_LEN] = buffer[1..].try_into().unwrap();
    assert_eq!(decode_u32(array), 0);
  }

  #[test]
  fn finalize_ok() {
    let mut buffer = BytesMut::new();
    buffer.put_u8(13);

    let mut prefixer = Prefixer::new(&mut buffer);

    prefixer.suffix_mut().extend_from_slice(&[0; 42]);

    prefixer.finalize().unwrap();

    // 1 junk prefix byte, length prefix, 42 bytes of value.
    assert_eq!(buffer.len(), U32_BYTE_LEN + 43);
    let prefix = &buffer[1..U32_BYTE_LEN + 1];
    let array: [u8; U32_BYTE_LEN] = prefix.try_into().unwrap();
    assert_eq!(decode_u32(array), 42);
  }
}
