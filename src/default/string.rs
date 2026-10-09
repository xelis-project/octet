use crate::VarUint;
use crate::{Error, Readable, Reader, Serializable, Writable};
use matriochka::ResultExt;

impl Serializable for String {
    fn write<W: Writable>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| {
            // Write length as VarInt, then raw UTF-8 bytes
            VarUint(self.len() as u64).write(writer).context("length")?;
            writer.extend_bytes(self.as_bytes()).context("UTF-8 bytes")
        })
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| {
            let len = VarUint::read(reader).context("length")?.0 as usize;
            let bytes = reader.read_vec(len).context("UTF-8 bytes")?;
            String::from_utf8(bytes).context("UTF-8 bytes")
        })
    }

    fn size(&self) -> usize {
        VarUint::encoded_size(self.len()) + self.len()
    }
}
