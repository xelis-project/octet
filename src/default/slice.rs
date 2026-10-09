use crate::DecodeError;
use crate::{Error, Readable, Reader, Serializable, Writable};

impl<'a> Serializable for &'a [u8] {
    fn write<W: Writable>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| writer.extend_bytes(self))
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|_| Err(DecodeError::NotImplemented.into()))
    }

    fn size(&self) -> usize {
        self.len()
    }
}

impl<const N: usize> Serializable for [u8; N] {
    fn write<W: Writable>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| writer.extend_bytes(self))
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| {
            let mut buffer = [0u8; N];
            reader.read_exact(&mut buffer)
                .map(|_| buffer)
        })
    }

    fn size(&self) -> usize {
        self.len()
    }
}
