use std::collections::{BTreeMap, BTreeSet};

use matriochka::ResultExt;

use crate::{DecodeError, Error, Readable, Reader, Serializable, VarUint, Writable, Writer};

impl<K: Serializable + Ord, V: Serializable> Serializable for BTreeMap<K, V> {
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| {
            VarUint(self.len() as u64).write(writer).context("length")?;
            for (index, (key, value)) in self.iter().enumerate() {
                key.write(writer)
                    .with_context(|| format!("entry[{index}].key"))?;
                value
                    .write(writer)
                    .with_context(|| format!("entry[{index}].value"))?;
            }
            Ok(())
        })
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| {
            let len = VarUint::read(reader).context("length")?.value();
            let mut map = BTreeMap::new();
            for index in 0..len {
                let key = K::read(reader).with_context(|| format!("entry[{index}].key"))?;
                let value = V::read(reader).with_context(|| format!("entry[{index}].value"))?;
                if map.insert(key, value).is_some() {
                    return Err(Error::from(DecodeError::UnexpectedValue)
                        .context(format!("entry[{index}].key: duplicate key")));
                }
            }
            Ok(map)
        })
    }

    fn size(&self) -> usize {
        VarUint::encoded_size(self.len())
            + self
                .iter()
                .map(|(key, value)| key.size() + value.size())
                .sum::<usize>()
    }
}

impl<T: Serializable + Ord> Serializable for BTreeSet<T> {
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| {
            VarUint(self.len() as u64).write(writer).context("length")?;
            for (index, item) in self.iter().enumerate() {
                item.write(writer)
                    .with_context(|| format!("element[{index}]"))?;
            }
            Ok(())
        })
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| {
            let len = VarUint::read(reader).context("length")?.value();
            let mut set = BTreeSet::new();
            for index in 0..len {
                let item = T::read(reader).with_context(|| format!("element[{index}]"))?;
                if !set.insert(item) {
                    return Err(Error::from(DecodeError::UnexpectedValue)
                        .context(format!("element[{index}]: duplicate value")));
                }
            }
            Ok(set)
        })
    }

    fn size(&self) -> usize {
        VarUint::encoded_size(self.len()) + self.iter().map(Serializable::size).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btrees_encode_in_sorted_order() {
        let mut map = BTreeMap::new();
        map.insert(3u8, 0x1234u16);
        map.insert(1, 0xabcd);
        let bytes = map.to_bytes().unwrap();
        assert_eq!(bytes.as_ref(), [2, 1, 0xab, 0xcd, 3, 0x12, 0x34]);
        assert_eq!(map.size(), bytes.len());
        assert_eq!(BTreeMap::<u8, u16>::from_bytes(&bytes).unwrap(), map);

        let set = BTreeSet::from([3u16, 1, 2]);
        let bytes = set.to_bytes().unwrap();
        assert_eq!(bytes.as_ref(), [3, 0, 1, 0, 2, 0, 3]);
        assert_eq!(set.size(), bytes.len());
        assert_eq!(BTreeSet::<u16>::from_bytes(&bytes).unwrap(), set);
    }

    #[test]
    fn empty_and_large_btrees_round_trip() {
        let empty_map = BTreeMap::<String, Box<[u8; 2]>>::new();
        assert_eq!(empty_map.to_bytes().unwrap().as_ref(), [0]);
        assert_eq!(empty_map.size(), 1);
        assert_eq!(BTreeMap::from_bytes([0]).unwrap(), empty_map);
        let empty_set = BTreeSet::<String>::new();
        assert_eq!(empty_set.to_bytes().unwrap().as_ref(), [0]);
        assert_eq!(empty_set.size(), 1);
        assert_eq!(BTreeSet::from_bytes([0]).unwrap(), empty_set);

        let map: BTreeMap<u16, Box<[u8; 2]>> = (0u16..300)
            .map(|key| (key, Box::new(key.to_be_bytes())))
            .collect();
        let bytes = map.to_bytes().unwrap();
        assert_eq!(&bytes.as_ref()[..3], [0xfd, 1, 44]);
        assert_eq!(map.size(), bytes.len());
        assert_eq!(BTreeMap::from_bytes(&bytes).unwrap(), map);

        let set: BTreeSet<u16> = (0..300).collect();
        let bytes = set.to_bytes().unwrap();
        assert_eq!(&bytes.as_ref()[..3], [0xfd, 1, 44]);
        assert_eq!(set.size(), bytes.len());
        assert_eq!(BTreeSet::from_bytes(&bytes).unwrap(), set);
    }

    #[test]
    fn btrees_reject_duplicates_and_report_truncated_fields() {
        let error = BTreeMap::<u8, u8>::from_bytes([2, 1, 10, 1, 20]).unwrap_err();
        assert!(matches!(
            error.downcast_ref::<DecodeError>(),
            Some(DecodeError::UnexpectedValue)
        ));
        assert!(format!("{error:#}").contains("entry[1].key: duplicate key"));
        let error = BTreeSet::<u8>::from_bytes([2, 1, 1]).unwrap_err();
        assert!(matches!(
            error.downcast_ref::<DecodeError>(),
            Some(DecodeError::UnexpectedValue)
        ));
        assert!(format!("{error:#}").contains("element[1]: duplicate value"));

        for (bytes, field) in [(&[1][..], "entry[0].key"), (&[1, 2][..], "entry[0].value")] {
            let error = BTreeMap::<u8, u16>::from_bytes(bytes).unwrap_err();
            assert!(matches!(
                error.downcast_ref::<DecodeError>(),
                Some(DecodeError::OutOfBounds { .. })
            ));
            assert!(format!("{error:#}").contains(field));
        }
        let error = BTreeSet::<u16>::from_bytes([1, 0]).unwrap_err();
        assert!(format!("{error:#}").contains("element[0]"));
    }
}
