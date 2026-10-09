use crate::{DecodeError, SerializedBytes, Readable, Reader, Error, Serializable, Writable, Writer};

pub struct WritableBytes<T: AsRef<[u8]>>(pub T);

impl<T: AsRef<[u8]>> Serializable for WritableBytes<T> {
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| writer.extend_bytes(self.0.as_ref()))
    }

    fn to_bytes<'a>(&'a self) -> Result<SerializedBytes<'a>, Error> {
        Ok(SerializedBytes::Borrowed(self.0.as_ref()))
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|_| Err(DecodeError::NotImplemented.into()))
    }

    fn size(&self) -> usize {
        self.0.as_ref().len()
    }
}
