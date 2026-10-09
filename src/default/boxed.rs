use matriochka::ResultExt;

use crate::{Error, Readable, Reader, Serializable, SerializedBytes, Writable, Writer};

impl<T: Serializable> Serializable for Box<T> {
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| self.as_ref().write(writer).context("value"))
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| T::read(reader).map(Box::new).context("value"))
    }

    fn to_bytes<'a>(&'a self) -> Result<SerializedBytes<'a>, Error> {
        self.as_ref().to_bytes()
    }

    fn size(&self) -> usize {
        self.as_ref().size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DecodeError;

    #[test]
    fn boxes_preserve_inner_encoding_and_borrowed_bytes() {
        let value = Box::new(*b"hello");
        let bytes = value.to_bytes().unwrap();
        assert_eq!(bytes.as_ref(), value.as_ref().to_bytes().unwrap().as_ref());
        assert_eq!(value.size(), bytes.len());
        let mut written = Vec::new();
        value.write(&mut Writer::new(&mut written)).unwrap();
        assert_eq!(written, bytes.as_ref());
        assert_eq!(Box::<[u8; 5]>::from_bytes(bytes).unwrap(), value);

        let raw = Box::new(::bytes::Bytes::from_static(b"borrowed"));
        let bytes = raw.to_bytes().unwrap();
        assert!(matches!(bytes, SerializedBytes::Borrowed(_)));
        assert_eq!(bytes.as_ref().as_ptr(), raw.as_ref().as_ptr());
    }

    #[test]
    fn boxes_preserve_inner_errors_and_context() {
        let error = Box::<char>::from_bytes(0xd800u32.to_be_bytes()).unwrap_err();
        assert!(matches!(
            error.downcast_ref::<DecodeError>(),
            Some(DecodeError::UnexpectedValue)
        ));
        let diagnostic = format!("{error:#}");
        assert!(diagnostic.contains("value"));
        assert!(diagnostic.contains("scalar"));
    }
}
