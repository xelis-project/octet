use matriochka::{Error, ResultExt};
use std::any::type_name;

/// Trait for writing serialized data to a byte buffer or stream
pub trait Writable {
    /// Attach the serialized object's type to any error.
    /// Use this around custom `Serializable::write` implementations, and add
    /// field names with Matriochka's `context` at each nested write.
    fn write_with_context<T>(
        &mut self,
        write: impl FnOnce(&mut Self) -> Result<(), Error>,
    ) -> Result<(), Error>
    where
        Self: Sized,
    {
        write(self).with_context(|| format!("writing {}", type_name::<T>()))
    }

    /// Write a single byte to the output
    #[inline(always)]
    fn push(&mut self, byte: u8) -> Result<(), Error> {
        self.extend_bytes(&[byte])
    }

    /// Write all bytes or return an error. A failed write may have already
    /// written a prefix; the destination determines buffering and atomicity.
    fn extend_bytes(&mut self, bytes: &[u8]) -> Result<(), Error>;

    /// Pre-allocate space for additional bytes to optimize writes
    /// Returns true if pre-allocation was successful, false otherwise
    /// This is a hint to the underlying storage to optimize for the expected size
    fn pre_allocate(&mut self, _additional: usize) -> bool {
        false
    }
}

impl Writable for Vec<u8> {
    fn extend_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.extend_from_slice(bytes);
        Ok(())
    }

    fn pre_allocate(&mut self, additional: usize) -> bool {
        self.reserve(additional);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Serializable, SerializedBytes, VarUint, WritableBytes};
    use bytes::Bytes;
    use std::io::{self, Cursor, ErrorKind, Write};

    // Test adapter for exercising error propagation through a fallible sink.
    struct IoWriter<W>(W);

    impl<W: Write> Writable for IoWriter<W> {
        fn extend_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
            self.0.write_all(bytes).map_err(Error::new)
        }
    }

    #[test]
    fn serialization_writes_directly_to_a_fixed_buffer() {
        let value = Some(vec![253u64, 300u64]);
        let mut buffer = [0u8; 32];
        let mut writer = IoWriter(Cursor::new(buffer.as_mut_slice()));

        value.write(&mut writer).unwrap();

        let written = writer.0.position() as usize;
        assert_eq!(written, value.size());
        assert_eq!(
            &writer.0.get_ref()[..written],
            value.to_bytes().unwrap().as_ref()
        );
        assert!(!writer.pre_allocate(32));
    }

    struct PartialWriter {
        buffer: [u8; 32],
        written: usize,
        interrupt: bool,
    }

    impl Write for PartialWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.interrupt {
                self.interrupt = false;
                return Err(io::Error::from(ErrorKind::Interrupted));
            }

            let count = bytes.len().min(2).min(self.buffer.len() - self.written);
            self.buffer[self.written..self.written + count].copy_from_slice(&bytes[..count]);
            self.written += count;
            Ok(count)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn io_writer_handles_partial_and_interrupted_writes() {
        let value = 0x0102030405060708u64;
        let mut writer = IoWriter(PartialWriter {
            buffer: [0; 32],
            written: 0,
            interrupt: true,
        });

        value.write(&mut writer).unwrap();

        assert_eq!(writer.0.written, 8);
        assert_eq!(&writer.0.buffer[..8], &value.to_be_bytes());
    }

    fn assert_io_failure<T: Serializable>(value: T) {
        let mut buffer = [];
        let mut writer = IoWriter(Cursor::new(buffer.as_mut_slice()));

        let error = value.write(&mut writer).unwrap_err();

        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            ErrorKind::WriteZero
        );
    }

    #[test]
    fn serializers_propagate_sink_errors() {
        assert_io_failure(1u8);
        assert_io_failure(1u16);
        assert_io_failure(1u32);
        assert_io_failure(1u64);
        assert_io_failure(-1i64);
        assert_io_failure(true);
        assert_io_failure("hello".to_owned());
        assert_io_failure(&b"hello"[..]);
        assert_io_failure(Bytes::from_static(b"hello"));
        assert_io_failure(SerializedBytes::Borrowed(b"hello"));
        assert_io_failure(WritableBytes(b"hello"));
        assert_io_failure(Some(vec![1u64, 2]));

        for value in [1, 253, 65_536, 0x1_0000_0000] {
            assert_io_failure(VarUint(value));
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("sink rejected write")]
    struct SinkRejected;

    struct RejectSecondWrite {
        calls: usize,
    }

    impl Writable for RejectSecondWrite {
        fn extend_bytes(&mut self, _: &[u8]) -> Result<(), Error> {
            self.calls += 1;
            if self.calls == 2 {
                return Err(Error::new(SinkRejected).context("writing collection length"));
            }

            Ok(())
        }
    }

    #[test]
    fn composite_serialization_stops_at_the_first_failed_write() {
        let mut writer = RejectSecondWrite { calls: 0 };

        let error = Some(vec![1u64, 2]).write(&mut writer).unwrap_err();

        assert!(error.downcast_ref::<SinkRejected>().is_some());
        assert_eq!(
            format!("{error:#}"),
            "writing core::option::Option<alloc::vec::Vec<u64>>: Some: writing alloc::vec::Vec<u64>: length: writing octet::varuint::VarUint: prefix: writing collection length: sink rejected write"
        );
        assert_eq!(writer.calls, 2);
    }

    #[test]
    fn nested_collections_report_the_complete_encoding_path() {
        let value = vec![None, Some(vec![1u64, 2])];
        // Leave room for everything except the final byte of the second u64.
        let mut buffer = vec![0; value.size() - 1];
        let mut writer = IoWriter(Cursor::new(buffer.as_mut_slice()));
        let error = value.write(&mut writer).unwrap_err();

        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            ErrorKind::WriteZero
        );
        let diagnostic = format!("{error:#}");
        let mut remaining = diagnostic.as_str();
        for frame in [
            "writing alloc::vec::Vec<",
            "element[1]:",
            "writing core::option::Option<",
            "Some:",
            "writing alloc::vec::Vec<u64>",
            "element[1]:",
            "writing u64:",
        ] {
            let offset = remaining
                .find(frame)
                .unwrap_or_else(|| panic!("missing or out-of-order frame {frame:?}: {diagnostic}"));
            remaining = &remaining[offset + frame.len()..];
        }
        assert_eq!(writer.0.position() as usize, value.size() - 1);
    }

    #[test]
    fn custom_serializers_preserve_type_field_and_cause_context() {
        struct Record;

        impl Serializable for Record {
            fn write<W: Writable>(&self, writer: &mut W) -> Result<(), Error> {
                writer.write_with_context::<Self>(|writer| 42u64.write(writer).context("account id"))
            }

            fn read<R: crate::Readable>(_: &mut crate::Reader<R>) -> Result<Self, Error> {
                Err(crate::DecodeError::NotImplemented.into())
            }

            fn size(&self) -> usize {
                8
            }
        }

        struct Rejected;

        impl Writable for Rejected {
            fn extend_bytes(&mut self, _: &[u8]) -> Result<(), Error> {
                Err(Error::new(SinkRejected).context("custom sink"))
            }
        }

        let error = Record.write(&mut Rejected).unwrap_err();
        assert!(error.downcast_ref::<SinkRejected>().is_some());
        assert_eq!(
            format!("{error:#}"),
            format!(
                "writing {}: account id: writing u64: custom sink: sink rejected write",
                type_name::<Record>()
            )
        );
    }
}
