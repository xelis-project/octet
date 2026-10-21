use crate::BytesView;
use crate::{Error, Readable, Reader, Serializable, Writable, Writer};
use ::bytes::Bytes;

impl Serializable for Bytes {
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| writer.extend_bytes(self.as_ref()))
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes)?;
            Ok(Bytes::from(bytes))
        })
    }

    fn to_bytes<'a>(&'a self) -> Result<BytesView<'a>, Error> {
        Ok(BytesView::Borrowed(self.as_ref()))
    }

    fn size(&self) -> usize {
        self.len()
    }
}
