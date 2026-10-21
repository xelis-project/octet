use std::{borrow::Borrow, cmp::Ordering, hash::{Hash, Hasher}, ops::Deref};

use bytes::Bytes;

use crate::{Readable, Reader, Error, Serializable, Writable, Writer};

/// A byte view backed by borrowed, owned, or shared storage.
#[derive(Debug, Clone)]
pub enum BytesView<'a> {
    Borrowed(&'a [u8]),
    Owned(Box<[u8]>),
    Shared(Bytes),
}

impl<'a> BytesView<'a> {
    /// Convert the data into a Vec<u8>, consuming self
    #[inline]
    pub fn into_vec(self) -> Vec<u8> {
        match self {
            BytesView::Borrowed(bytes) => bytes.to_vec(),
            BytesView::Owned(bytes) => bytes.into_vec(),
            BytesView::Shared(bytes) => bytes.to_vec(),
        }
    }

    /// Convert the data into a Vec<u8> without consuming self
    #[inline]
    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            BytesView::Borrowed(bytes) => bytes.to_vec(),
            BytesView::Owned(bytes) => bytes.to_vec(),
            BytesView::Shared(bytes) => bytes.to_vec(),
        }
    }

    /// Convert the data into a Box<[u8]> without consuming self
    #[inline]
    pub fn to_boxed_slice(&self) -> Box<[u8]> {
        match self {
            BytesView::Borrowed(bytes) => (*bytes).into(),
            BytesView::Owned(bytes) => bytes.clone(),
            BytesView::Shared(bytes) => bytes.as_ref().into(),
        }
    }

    /// Convert the data into a Box<[u8]>, consuming self
    #[inline]
    pub fn into_boxed_slice(self) -> Box<[u8]> {
        match self {
            BytesView::Borrowed(bytes) => bytes.into(),
            BytesView::Owned(bytes) => bytes,
            BytesView::Shared(bytes) => bytes.as_ref().into(),
        }
    }

    /// Convert into a Bytes, consuming self
    #[inline]
    pub fn into_bytes(self) -> Bytes {
        match self {
            BytesView::Borrowed(bytes) => Bytes::copy_from_slice(bytes),
            BytesView::Owned(bytes) => Bytes::copy_from_slice(&bytes),
            BytesView::Shared(bytes) => bytes,
        }
    }

    /// Convert into a Bytes without consuming self
    #[inline]
    pub fn to_bytes(&self) -> Bytes {
        match self {
            BytesView::Borrowed(bytes) => Bytes::copy_from_slice(bytes),
            BytesView::Owned(bytes) => Bytes::copy_from_slice(&bytes),
            BytesView::Shared(bytes) => bytes.clone(),
        }
    }
}

impl<'a> AsRef<[u8]> for BytesView<'a> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        match self {
            BytesView::Borrowed(bytes) => bytes,
            BytesView::Owned(bytes) => bytes,
            BytesView::Shared(bytes) => bytes.as_ref(),
        }
    }
}

impl<'a> Deref for BytesView<'a> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a> From<Vec<u8>> for BytesView<'a> {
    #[inline]
    fn from(vec: Vec<u8>) -> Self {
        BytesView::Owned(vec.into_boxed_slice())
    }
}

impl<'a> From<Bytes> for BytesView<'a> {
    #[inline]
    fn from(bytes: Bytes) -> Self {
        BytesView::Shared(bytes)
    }
}

impl<'a> From<&'a [u8]> for BytesView<'a> {
    #[inline]
    fn from(bytes: &'a [u8]) -> Self {
        BytesView::Borrowed(bytes)
    }
}

impl<'a> Borrow<[u8]> for BytesView<'a> {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self.as_ref()
    }
}

impl Hash for BytesView<'_> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl PartialEq for BytesView<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl Eq for BytesView<'_> {}

impl PartialOrd for BytesView<'_> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl Ord for BytesView<'_> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<'a> Into<Bytes> for BytesView<'a> {
    #[inline]
    fn into(self) -> Bytes {
        self.into_bytes()
    }
}

impl<'a> Into<Vec<u8>> for BytesView<'a> {
    #[inline]
    fn into(self) -> Vec<u8> {
        self.into_vec()
    }
}

impl<'a> Into<Box<[u8]>> for BytesView<'a> {
    #[inline]
    fn into(self) -> Box<[u8]> {
        self.into_boxed_slice()
    }
}

impl<'a> Serializable for BytesView<'a> {
    #[inline]
    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes)?;
            Ok(BytesView::Owned(bytes.into()))
        })
    }

    #[inline]
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| writer.extend_bytes(self.as_ref()))
    }

    #[inline]
    fn to_bytes<'b>(&'b self) -> Result<BytesView<'b>, Error> {
        Ok(BytesView::Borrowed(self.as_ref()))
    }

    #[inline]
    fn size(&self) -> usize {
        self.as_ref().len()
    }
}