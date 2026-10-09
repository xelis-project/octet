//! Binary serialization with fallible byte sources and sinks.
//!
//! Errors retain their concrete causes and Matriochka context through nested
//! objects, fields, and collection elements.

#![doc = include_str!("../README.md")]

use matriochka::ResultExt;

mod writer;
mod reader;
mod default;
mod writable;
mod varuint;
mod bytes;

use std::any::type_name;
use matriochka::Error;

pub use writer::{Writable, Writer};
pub use runtime_context::{self, Context};
pub use reader::{DecodeError, Readable, Reader, SliceSource};
pub use writable::WritableBytes;
pub use varuint::VarUint;
pub use bytes::SerializedBytes;

/// Trait for types that can be serialized and deserialized as binary data
pub trait Serializable: Sized {
    /// Serialize directly into a buffer or sink, propagating write errors.
    /// On failure the sink may contain a partially serialized value.
    /// Wrap custom implementations in `writer.write_with_context::<Self>(...)`
    /// and attach field names with Matriochka's `ResultExt::context`.
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error>;

    /// Serialize to bytes
    fn to_bytes<'a>(&'a self) -> Result<SerializedBytes<'a>, Error> {
        let mut writer = Writer::new(Vec::with_capacity(self.size()));
        self.write(&mut writer)
            .with_context(|| format!("serializing {}", type_name::<Self>()))?;
        Ok(SerializedBytes::Owned(writer.into_inner().into_boxed_slice()))
    }

    /// Serialize to a lowercase hexadecimal string without a prefix.
    fn to_hex(&self) -> Result<String, Error> {
        self.to_bytes().map(hex::encode)
    }

    /// Deserialize from hexadecimal, rejecting invalid hex and trailing bytes.
    /// Accepts uppercase and lowercase digits without a prefix or whitespace.
    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Error> {
        hex::decode(hex)
            .with_context(|| format!("decoding hex for {}", type_name::<Self>()))
            .and_then(Self::from_bytes)
    }

    /// Deserialize from bytes, rejecting trailing bytes after the value.
    fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> Result<Self, Error> {
        let mut reader = Reader::new(bytes.as_ref());
        let value = Self::read(&mut reader)
            .with_context(|| {
                format!(
                    "deserializing {} at byte {}",
                    type_name::<Self>(),
                    reader.total_read(),
                )
            })?;

            if reader.has_more() {
                return Err(DecodeError::TrailingBytes(reader.remaining()).into())
            }

            Ok(value)
    }

    /// Deserialize one value from bytes, allowing trailing bytes.
    fn from_bytes_non_strict<T: AsRef<[u8]>>(bytes: T) -> Result<Self, Error> {
        let mut reader = Reader::new(bytes.as_ref());
        Self::read(&mut reader).with_context(|| {
            format!(
                "deserializing {} at byte {}",
                type_name::<Self>(),
                reader.total_read(),
            )
        })
    }

    /// Read an instance of the type from a reader.
    /// Wrap custom implementations in `reader.read_with_context(...)` and attach
    /// field names to nested results with Matriochka's `ResultExt::context`.
    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error>;

    /// Estimate the size of the serialized entity without actually serializing it
    fn size(&self) -> usize;
}

impl<'a, T: Serializable> Serializable for &'a T {
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        (*self).write(writer)
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|_| Err(DecodeError::NotImplemented.into()))
    }

    fn size(&self) -> usize {
        (*self).size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_serialized_values() {
        assert_eq!(0xabcdu16.to_hex().unwrap(), "abcd");
        assert_eq!(u16::from_hex("ABcd").unwrap(), 0xabcd);
        let value = Some(vec![1u64, 2, 3]);
        assert_eq!(Option::<Vec<u64>>::from_hex(value.to_hex().unwrap()).unwrap(), value);
        assert_eq!(().to_hex().unwrap(), "");
        assert_eq!(<()>::from_hex("").unwrap(), ());
    }

    #[test]
    fn from_hex_rejects_trailing_and_truncated_bytes() {
        let error = u8::from_hex("2a63").unwrap_err();
        assert!(matches!(error.downcast_ref::<DecodeError>(), Some(DecodeError::TrailingBytes(1))));
        let error = u16::from_hex("2a").unwrap_err();
        assert!(matches!(error.downcast_ref::<DecodeError>(), Some(DecodeError::OutOfBounds { .. })));
    }

    #[test]
    fn from_hex_preserves_invalid_hex_errors() {
        for input in ["a", "gg", "0x2a", "2a "] {
            let error = u8::from_hex(input).unwrap_err();
            assert!(error.downcast_ref::<hex::FromHexError>().is_some());
            assert!(format!("{error:#}").contains("decoding hex for u8"));
        }
    }
}
