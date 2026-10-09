use crate::SerializedBytes;
use crate::{Error, Readable, Reader, Serializable, Writable, Writer};
use ::bytes::Bytes;

impl Serializable for Bytes {
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| writer.extend_bytes(self.as_ref()))
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| reader.read_remaining_bytes().map(Bytes::from))
    }

    fn to_bytes<'a>(&'a self) -> Result<SerializedBytes<'a>, Error> {
        Ok(SerializedBytes::Borrowed(self.as_ref()))
    }

    fn size(&self) -> usize {
        self.len()
    }
}
