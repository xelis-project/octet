use matriochka::ResultExt;

use crate::{DecodeError, Error, Readable, Reader, Serializable, SerializedBytes, Writable, Writer};

macro_rules! impl_serializable_fixed_width {
    ($($ty:ty => $size:expr),+) => {
        $(
            impl Serializable for $ty {
                fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
                    writer.write_with_context::<Self>(|writer| {
                        writer.extend_bytes(&self.to_be_bytes())
                    })
                }

                fn to_bytes<'a>(&'a self) -> Result<SerializedBytes<'a>, Error> {
                    Ok(SerializedBytes::Owned(Box::new(self.to_be_bytes())))
                }

                fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
                    reader.read_with_context(|reader| {
                        let mut bytes = [0; $size];
                        reader.read_exact(&mut bytes)?;
                        Ok(<$ty>::from_be_bytes(bytes))
                    })
                }

                fn size(&self) -> usize {
                    $size
                }
            }
        )+
    };
}

impl_serializable_fixed_width!(
    u16 => 2,
    u32 => 4,
    u64 => 8,
    u128 => 16,
    i16 => 2,
    i32 => 4,
    i64 => 8,
    i128 => 16,
    // Floats use their IEEE 754 bit patterns,
    // preserving NaN payloads and signed zero.
    f32 => 4,
    f64 => 8
);

impl Serializable for u8 {
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| writer.push(*self))
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| reader.next_byte())
    }

    fn size(&self) -> usize {
        1
    }
}

impl Serializable for bool {
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| (*self as u8).write(writer).context("tag"))
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| match u8::read(reader).context("tag")? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError::UnexpectedValue.into()),
        })
    }

    fn size(&self) -> usize {
        1
    }
}

impl Serializable for char {
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| (*self as u32).write(writer).context("scalar"))
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| {
            let scalar = u32::read(reader).context("scalar")?;
            char::from_u32(scalar)
                .ok_or(DecodeError::UnexpectedValue)
                .context("scalar")
        })
    }

    fn size(&self) -> usize {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_preserve_bits_and_big_endian_encoding() {
        for bits in [
            0u32,
            0x8000_0000,
            0x3f80_0000,
            0x7f80_0000,
            0xff80_0000,
            0x7fc0_1234,
            1,
        ] {
            let value = f32::from_bits(bits);
            let bytes = value.to_bytes().unwrap();
            assert_eq!(bytes.as_ref(), bits.to_be_bytes());
            assert_eq!(value.size(), bytes.len());
            assert_eq!(f32::from_bytes(&bytes).unwrap().to_bits(), bits);
            let mut written = Vec::new();
            value.write(&mut Writer::new(&mut written)).unwrap();
            assert_eq!(written, bytes.as_ref());
        }
        for bits in [
            0u64,
            0x8000_0000_0000_0000,
            0x3ff0_0000_0000_0000,
            0x7ff0_0000_0000_0000,
            0xfff0_0000_0000_0000,
            0x7ff8_0000_0000_1234,
            1,
        ] {
            let value = f64::from_bits(bits);
            let bytes = value.to_bytes().unwrap();
            assert_eq!(bytes.as_ref(), bits.to_be_bytes());
            assert_eq!(value.size(), bytes.len());
            assert_eq!(f64::from_bytes(&bytes).unwrap().to_bits(), bits);
            let mut written = Vec::new();
            value.write(&mut Writer::new(&mut written)).unwrap();
            assert_eq!(written, bytes.as_ref());
        }
        assert!(f32::from_bytes([0; 3]).is_err());
        assert!(f64::from_bytes([0; 7]).is_err());
    }

    #[test]
    fn chars_encode_unicode_scalars_and_reject_invalid_values() {
        for value in ['\0', 'A', 'é', '🦀', '\u{d7ff}', '\u{e000}', '\u{10ffff}'] {
            let bytes = value.to_bytes().unwrap();
            assert_eq!(bytes.as_ref(), (value as u32).to_be_bytes());
            assert_eq!(value.size(), bytes.len());
            assert_eq!(char::from_bytes(&bytes).unwrap(), value);
        }
        for scalar in [0xd800u32, 0xdfff, 0x110000, u32::MAX] {
            let error = char::from_bytes(scalar.to_be_bytes()).unwrap_err();
            assert!(matches!(
                error.downcast_ref::<DecodeError>(),
                Some(DecodeError::UnexpectedValue)
            ));
            assert!(format!("{error:#}").contains("scalar"));
        }
        assert!(char::from_bytes([0; 3]).is_err());
    }
}
