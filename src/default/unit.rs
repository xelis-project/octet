use crate::{Error, Readable, Reader, Serializable, Writable, Writer};

impl Serializable for () {
    fn write<W: Writable>(&self, _writer: &mut Writer<W>) -> Result<(), Error> {
        Ok(())
    }

    fn read<R: Readable>(_reader: &mut Reader<R>) -> Result<Self, Error> {
        Ok(())
    }

    fn size(&self) -> usize {
        0
    }
}
