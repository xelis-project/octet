use octet::{DecodeError, Error, Serializable, Writable, Writer};

#[derive(Debug, PartialEq, Serializable)]
struct Record<T>
where
    T: PartialEq,
{
    id: u16,
    value: T,
}

#[derive(Debug, PartialEq, Serializable)]
struct Tuple<T, const N: usize>(T, [u8; N]);

#[derive(Debug, PartialEq, Serializable)]
struct Unit;

#[derive(Serializable)]
struct Borrowed<'a>(&'a u8);

#[derive(Debug, PartialEq, Serializable)]
enum Message<T> {
    Empty,
    Tuple(T, u16),
    Named { value: T },
}

fn round_trip<T: Serializable + PartialEq + std::fmt::Debug>(value: T, expected: &[u8]) {
    let bytes = value.to_bytes().unwrap();
    assert_eq!(bytes.as_ref(), expected);
    assert_eq!(value.size(), expected.len());
    assert_eq!(T::from_bytes(bytes).unwrap(), value);
}

#[test]
fn derives_encode_fields_and_tags_in_declaration_order() {
    assert_eq!(Borrowed(&5).to_bytes().unwrap().as_ref(), [5]);
    round_trip(
        Record {
            id: 0x1234,
            value: 5u8,
        },
        &[0x12, 0x34, 5],
    );
    round_trip(Tuple(5u8, [6, 7]), &[5, 6, 7]);
    round_trip(Unit, &[]);
    round_trip(Message::<u8>::Empty, &[0]);
    round_trip(Message::Tuple(5u8, 0x1234), &[1, 5, 0x12, 0x34]);
    round_trip(Message::Named { value: 5u8 }, &[2, 5]);
}

#[test]
fn decoding_errors_keep_field_and_variant_context() {
    let error = Record::<u16>::from_bytes([0, 1, 2]).unwrap_err();
    assert!(matches!(
        error.downcast_ref::<DecodeError>(),
        Some(DecodeError::OutOfBounds { .. })
    ));
    let diagnostic = format!("{error:#}");
    assert!(diagnostic.contains("Record"));
    assert!(diagnostic.contains("value"));
    let error = Message::<u16>::from_bytes([2, 1]).unwrap_err();
    let diagnostic = format!("{error:#}");
    assert!(diagnostic.contains("Named"));
    assert!(diagnostic.contains("value"));
    let error = Message::<u8>::from_bytes([3]).unwrap_err();
    assert!(matches!(
        error.downcast_ref::<DecodeError>(),
        Some(DecodeError::UnexpectedValue)
    ));
}

#[test]
fn writing_stops_at_failed_field_and_keeps_context() {
    struct Sink(Vec<u8>);
    impl Writable for Sink {
        fn extend_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
            if self.0.len() == 2 {
                return Err(DecodeError::NotImplemented.into());
            }
            self.0.extend_from_slice(bytes);
            Ok(())
        }
    }
    let mut writer = Writer::new(Sink(Vec::new()));
    let error = Record { id: 1, value: 2u8 }.write(&mut writer).unwrap_err();
    assert!(format!("{error:#}").contains("value"));
    assert_eq!(writer.into_inner().0, [0, 1]);
}
