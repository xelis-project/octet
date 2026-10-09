use crate::DecodeError;
use crate::{Error, Readable, Reader, Serializable, Writable};
use matriochka::ResultExt;

impl<T: Serializable> Serializable for Option<T> {
    fn write<W: Writable>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| match self {
            Some(value) => {
                1u8.write(writer).context("tag")?;
                value.write(writer).context("Some")
            }
            None => 0u8.write(writer).context("tag"),
        })
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| match u8::read(reader).context("tag")? {
            0 => Ok(None),
            1 => T::read(reader).map(Some).context("Some"),
            _ => Err(DecodeError::UnexpectedValue.into()),
        })
    }

    fn size(&self) -> usize {
        1 + self.as_ref().map_or(0, |value| value.size())
    }
}
