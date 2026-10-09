use crate::VarUint;
use crate::{Error, Readable, Reader, Serializable, Writable, Writer};
use matriochka::ResultExt;

impl<T: Serializable> Serializable for Vec<T> {
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| {
            // Write length as VarInt
            VarUint(self.len() as u64).write(writer).context("length")?;

            // Write each element
            for (index, item) in self.iter().enumerate() {
                item.write(writer)
                    .with_context(|| format!("element[{index}]"))?;
            }

            Ok(())
        })
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| {
            // Read length as VarInt
            let len = VarUint::read(reader).context("length")?.value();

            // Pre-allocate vector
            let mut vec = Vec::with_capacity(len.min(1024) as usize); // Cap allocation for safety

            // Read each element
            for index in 0..len {
                vec.push(T::read(reader).with_context(|| format!("element[{index}]"))?);
            }

            Ok(vec)
        })
    }

    fn size(&self) -> usize {
        let len_size = VarUint::encoded_size(self.len());
        let items_size: usize = self.iter().map(|item| item.size()).sum();
        len_size + items_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_u8_serialization() {
        let vec = vec![1u8, 2, 3, 4, 5];
        let bytes = vec.to_bytes().unwrap();
        let decoded = Vec::<u8>::from_bytes(&bytes).unwrap();
        assert_eq!(vec, decoded);
    }

    #[test]
    fn test_vec_u32_serialization() {
        let vec = vec![100u32, 200, 300, 400];
        let bytes = vec.to_bytes().unwrap();
        let decoded = Vec::<u32>::from_bytes(&bytes).unwrap();
        assert_eq!(vec, decoded);
    }

    #[test]
    fn test_vec_empty() {
        let vec: Vec<u64> = vec![];
        let bytes = vec.to_bytes().unwrap();
        assert_eq!(bytes.len(), 1); // Just the length prefix (0)
        let decoded = Vec::<u64>::from_bytes(&bytes).unwrap();
        assert_eq!(vec, decoded);
    }

    #[test]
    fn test_vec_single_element() {
        let vec = vec![42u64];
        let bytes = vec.to_bytes().unwrap();
        let decoded = Vec::<u64>::from_bytes(&bytes).unwrap();
        assert_eq!(vec, decoded);
    }

    #[test]
    fn test_vec_large_collection() {
        let vec: Vec<u16> = (0..1000).collect();
        let bytes = vec.to_bytes().unwrap();
        let decoded = Vec::<u16>::from_bytes(&bytes).unwrap();
        assert_eq!(vec, decoded);
    }

    #[test]
    fn test_vec_nested() {
        let vec = vec![vec![1u32, 2, 3], vec![4, 5], vec![6, 7, 8, 9]];
        let bytes = vec.to_bytes().unwrap();
        let decoded = Vec::<Vec<u32>>::from_bytes(&bytes).unwrap();
        assert_eq!(vec, decoded);
    }

    #[test]
    fn test_vec_size_calculation() {
        let vec = vec![1u64, 2, 3, 4, 5];
        let size = vec.size();
        let bytes = vec.to_bytes().unwrap();
        assert_eq!(size, bytes.len());
    }

    #[test]
    fn test_vec_option_serialization() {
        let vec = vec![Some(1u32), None, Some(3), None, Some(5)];
        let bytes = vec.to_bytes().unwrap();
        let decoded = Vec::<Option<u32>>::from_bytes(&bytes).unwrap();
        assert_eq!(vec, decoded);
    }

    #[test]
    fn test_vec_variable_size_prefix() {
        // Test that small length uses 1 byte prefix
        let small_vec = vec![1u8; 10];
        let small_bytes = small_vec.to_bytes().unwrap();
        assert_eq!(small_bytes[0], 10); // Direct length encoding

        // Test that larger length uses multi-byte prefix
        let large_vec = vec![1u8; 300];
        let large_bytes = large_vec.to_bytes().unwrap();
        assert_eq!(large_bytes[0], 0xFD); // VarInt prefix for u16
    }
}
