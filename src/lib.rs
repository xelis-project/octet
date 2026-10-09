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

    /// Deserialize from bytes
    fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> Result<Self, Error> {
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
