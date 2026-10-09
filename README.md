# Octet

Binary serialization for Rust with fallible byte sources, sinks, and contextual errors.

Octet provides an explicit `Serializable` trait for defining how values are
encoded and decoded. Use it with in-memory buffers or custom I/O adapters, with
errors that retain their concrete causes and context through nested values.

- Serialize to bytes or write directly to a `Writable` sink.
- Decode from borrowed slices, owned buffers, or `std::io::Read` sources.
- Trace failures through types, fields, collection indices, and decoding offsets.
- Use built-in implementations for integers, strings, options, and vectors.

## Installation

Add Octet to your `Cargo.toml`:

```toml
[dependencies]
octet = "0.1.0"
```

## Quick start

```rust
use octet::Serializable;

fn main() -> Result<(), matriochka::Error> {
    let value = Some(vec![1u64, 2, 3]);
    let bytes = value.to_bytes()?;
    let decoded = Option::<Vec<u64>>::from_bytes(&bytes)?;

    assert_eq!(decoded, value);
    Ok(())
}
```

`to_bytes()` returns a `SerializedBytes` value, which supports borrowed, owned,
and shared byte storage. Use `as_ref()` to access a slice or `into_vec()` to
obtain a `Vec<u8>`.

## Custom types

Implement `write`, `read`, and `size` to define a type's binary layout. Read
fields in the same order they are written, and return the estimated serialized
size from `size`.

Add `matriochka = "0.1.0"` to your dependencies to use its error type and
context helpers directly.

```rust
use matriochka::{Error, ResultExt};
use octet::{Readable, Reader, Serializable, Writable};

#[derive(Debug, PartialEq)]
struct Record {
    id: u64,
    name: String,
}

impl Serializable for Record {
    fn write<W: Writable>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_with_context::<Self>(|writer| {
            self.id.write(writer).context("id")?;
            self.name.write(writer).context("name")
        })
    }

    fn read<R: Readable>(reader: &mut Reader<R>) -> Result<Self, Error> {
        reader.read_with_context(|reader| {
            Ok(Self {
                id: u64::read(reader).context("id")?,
                name: String::read(reader).context("name")?,
            })
        })
    }

    fn size(&self) -> usize {
        self.id.size() + self.name.size()
    }
}
```

## Sources and sinks

Use `Reader::new` for a borrowed byte slice or an owned `Vec<u8>`. Slice readers
also expose `remaining()`, `has_more()`, and `read_bytes_ref()` for borrowing
bytes without copying.

For streams, use `Reader::from_source`. Any `std::io::Read` implementation
implements `Readable` automatically:

```rust
use std::io::Cursor;
use octet::{Reader, Serializable};

let value = 42u64;
let mut reader = Reader::from_source(Cursor::new(value.to_bytes()?.into_vec()));

assert_eq!(u64::read(&mut reader)?, value);
# Ok::<(), matriochka::Error>(())
```

For output, `Vec<u8>` implements `Writable`. Implement `Writable::extend_bytes`
to support another destination; an adapter for `std::io::Write` can forward to
`write_all` and convert errors with `matriochka::Error::new`. A failed write may
leave a partially serialized value in the destination.

## Error handling

Errors use `matriochka::Error` to preserve their concrete causes. Custom types
can attach type context with `read_with_context` and `write_with_context`, then
attach field names with `ResultExt::context`, as shown above. Built-in
collections attach element indices automatically.

Format an error with `format!("{error:#}")` to display the full context chain.
Use `error.downcast_ref::<octet::DecodeError>()` to inspect typed decoding
failures, or downcast to the original source or sink error type.

## Binary format

| Type | Encoding |
| --- | --- |
| `u8` | One byte |
| `u16`, `u32`, `u64`, `i64` | Fixed-width, big-endian bytes |
| `bool` | One-byte tag: `0` for false, `1` for true |
| `VarUint` | Values up to 252 use one byte; larger values use a prefix (`0xFD`, `0xFE`, or `0xFF`) followed by a big-endian `u16`, `u32`, or `u64` |
| `String` | UTF-8 byte length as `VarUint`, followed by UTF-8 bytes |
| `Option<T>` | One-byte tag: `0` for `None`, `1` followed by the value for `Some` |
| `Vec<T>` | Element count as `VarUint`, followed by each element |
| `()` | No bytes |
| `&[u8]`, `bytes::Bytes`, `SerializedBytes` | Raw bytes without a length prefix |

`bytes::Bytes` and `SerializedBytes` decode until the source reaches EOF; use a
bounded source when embedding them in a larger message. Borrowed `&[u8]` values
support encoding only. `from_bytes()` decodes one value and does not reject
trailing bytes; use a slice reader and check `remaining()` when full consumption
is required.

## Optional features

The `serde` feature enables Serde `Serialize` and `Deserialize` implementations
for `VarUint`. Octet's binary encoding uses `Serializable` independently of
Serde.

## Development

Run the unit tests and documentation examples with all features enabled:

```sh
cargo test -p octet --all-features
```
