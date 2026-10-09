use crate::{Error, Readable, Reader, Serializable, Writable};

impl Serializable for () {
    fn write<W: Writable>(&self, _writer: &mut W) -> Result<(), Error> {
        Ok(())
    }

    fn read<R: Readable>(_reader: &mut Reader<R>) -> Result<Self, Error> {
        Ok(())
    }

    fn size(&self) -> usize {
        0
    }
}
