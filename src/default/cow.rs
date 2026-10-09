use crate::SerializedBytes;
use crate::{Error, Readable, Reader, Serializable, Writable, Writer};
use matriochka::ResultExt;
use std::borrow::Cow;

impl<'a, T: Serializable + Clone> Serializable for Cow<'a, T> {
    #[inline]
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| {
            let field = match self {
                Cow::Borrowed(_) => "borrowed value",
                Cow::Owned(_) => "owned value",
            };
            self.as_ref().write(writer).context(field)
        })
    }

    #[inline]
    fn to_bytes<'b>(&'b self) -> Result<SerializedBytes<'b>, Error> {
        self.as_ref().to_bytes()
    }

    #[inline]
    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| T::read(reader).map(Cow::Owned).context("owned value"))
    }

    #[inline]
    fn size(&self) -> usize {
        self.as_ref().size()
    }
}
