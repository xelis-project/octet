use std::{borrow::Cow, collections::BTreeMap, io::Cursor};

use matriochka::Error;
use octet::{Context, Readable, Reader, Serializable, Writable, Writer};

#[derive(Default)]
struct State {
    writes: usize,
    reads: usize,
}
octet::runtime_context::tid!(State);

#[derive(Clone, Debug, PartialEq)]
struct Counted(u8);

impl Serializable for Counted {
    fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
        writer.context_mut().get_mut::<State>().unwrap().writes += 1;
        self.0.write(writer)
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.context_mut().get_mut::<State>().unwrap().reads += 1;
        u8::read(reader).map(Self)
    }

    fn size(&self) -> usize {
        1
    }
}

type Nested = BTreeMap<u8, Vec<Option<Box<Counted>>>>;

fn nested() -> Nested {
    BTreeMap::from([
        (1, vec![Some(Box::new(Counted(7))), None]),
        (2, vec![Some(Box::new(Counted(8)))]),
    ])
}

#[test]
fn nested_values_share_mutable_borrowed_context() {
    let mut state = State::default();
    {
        let mut context = Context::new();
        context.insert_mut(&mut state);
        let value = nested();
        let borrowed = Cow::Borrowed(&value);
        let mut writer = Writer::with_context(Vec::new(), context);
        borrowed.write(&mut writer).unwrap();
        assert_eq!(writer.context().unwrap().get::<State>().unwrap().writes, 2);
        let (bytes, context) = writer.into_parts();
        let mut reader = Reader::with_context(bytes.as_slice(), context);
        assert_eq!(Nested::read(&mut reader).unwrap(), value);
        assert_eq!(reader.context().unwrap().get::<State>().unwrap().reads, 2);
        assert_eq!(bytes, [2, 1, 2, 1, 7, 0, 2, 1, 1, 8]);
    }
    assert_eq!(state.writes, 2);
    assert_eq!(state.reads, 2);
}

#[test]
fn streaming_readers_and_custom_sinks_retain_context() {
    struct Sink(Vec<u8>);
    impl Writable for Sink {
        fn extend_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
            self.0.extend_from_slice(bytes);
            Ok(())
        }
    }

    let mut writer = Writer::new(Sink(Vec::new()));
    writer.context_mut().insert(State::default());
    nested().write(&mut writer).unwrap();
    let (sink, context) = writer.into_parts();
    let mut reader = Reader::with_context(Cursor::new(sink.0), context);
    assert_eq!(Nested::read(&mut reader).unwrap(), nested());
    let (_, context) = reader.into_parts();
    let context = context.unwrap();
    assert_eq!(context.get::<State>().unwrap().writes, 2);
    assert_eq!(context.get::<State>().unwrap().reads, 2);
}

#[test]
fn readers_and_writers_preserve_mutations_when_values_fail() {
    struct Failing;
    impl Serializable for Failing {
        fn write<W: Writable>(&self, writer: &mut Writer<W>) -> Result<(), Error> {
            writer.context_mut().get_mut::<State>().unwrap().writes += 1;
            Err(octet::DecodeError::UnexpectedValue.into())
        }
        fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
            reader.context_mut().get_mut::<State>().unwrap().reads += 1;
            Err(octet::DecodeError::UnexpectedValue.into())
        }
        fn size(&self) -> usize {
            0
        }
    }
    let mut context = Context::new();
    context.insert(State::default());
    let mut writer = Writer::with_context(Vec::new(), context);
    assert!(Some(Failing).write(&mut writer).is_err());
    assert_eq!(writer.context().unwrap().get::<State>().unwrap().writes, 1);
    let (bytes, context) = writer.into_parts();
    let mut reader = Reader::with_context(bytes.as_slice(), context);
    assert!(Option::<Failing>::read(&mut reader).is_err());
    assert_eq!(reader.context().unwrap().get::<State>().unwrap().reads, 1);
}

#[test]
fn default_context_stays_absent_until_mutable_access() {
    let mut writer = Writer::new(Vec::new());
    assert!(writer.context().is_none());
    Some(vec![1u64, 2]).write(&mut writer).unwrap();
    assert!(writer.context().is_none());
    let (bytes, context) = writer.into_parts();
    assert!(context.is_none());

    let mut reader = Reader::new(bytes.as_slice());
    assert_eq!(
        Option::<Vec<u64>>::read(&mut reader).unwrap(),
        Some(vec![1, 2])
    );
    assert!(reader.context().is_none());
    assert!(reader.into_parts().1.is_none());

    let mut writer = Writer::new(Vec::new());
    writer.context_mut().insert(State::default());
    Counted(7).write(&mut writer).unwrap();
    assert_eq!(writer.context().unwrap().get::<State>().unwrap().writes, 1);
    let (bytes, context) = writer.into_parts();
    let mut reader = Reader::with_context(bytes.as_slice(), context);
    assert_eq!(Counted::read(&mut reader).unwrap(), Counted(7));
    assert_eq!(reader.context().unwrap().get::<State>().unwrap().reads, 1);

    let mut reader = Reader::from_source(Cursor::new([42]));
    assert!(reader.context().is_none());
    reader.context_mut().insert(State::default());
    assert_eq!(Counted::read(&mut reader).unwrap(), Counted(42));
    assert_eq!(reader.context().unwrap().get::<State>().unwrap().reads, 1);
}
