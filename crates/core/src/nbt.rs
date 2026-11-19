//! NBT (Named Binary Tag) data model and Java-compatible codec (SPEC.md 5.5).
//!
//! NBT is the on-disk interchange format for level data, chunks, registry
//! snapshots, and structures. This module owns both the data model
//! ([`Value`] / [`Compound`]) and the binary codec, so nothing outside
//! `core::nbt` sees a third-party NBT type.
//!
//! # Why an in-house codec (ADR 0006)
//!
//! - `simdnbt` requires nightly Rust (`portable_simd`, `allocator_api`); the
//!   workspace pins stable 1.88.
//! - `fastnbt::Value` stores compounds in a `HashMap`, so re-serialization
//!   order is not deterministic — forbidden by SPEC.md 18 for anything that
//!   feeds hashing, saves, or the wire.
//!
//! The format is small and fully specified, so owning it keeps byte-level
//! behavior explicit: big-endian payloads, ordered compounds, Java modified
//! UTF-8 names/strings, and a hard depth limit.
//!
//! # Untrusted input rule (SPEC.md 18)
//!
//! Every read returns [`Result`]; corrupt data is an error, never a panic and
//! never an unbounded allocation.

// The casts below are deliberate bit-level reinterpretation (NBT payloads are
// defined as raw big-endian bit patterns, e.g. `i32` bits of an `f32`, and
// signed/unsigned byte-array storage). Do not replace them with `From`/`try_from`.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]

use std::io::{Read, Write};

use thiserror::Error;

/// Maximum nesting depth accepted by the reader (guards the stack).
pub const MAX_DEPTH: u32 = 512;

/// Tag ids, as defined by the NBT specification.
mod tag {
    pub const END: u8 = 0;
    pub const BYTE: u8 = 1;
    pub const SHORT: u8 = 2;
    pub const INT: u8 = 3;
    pub const LONG: u8 = 4;
    pub const FLOAT: u8 = 5;
    pub const DOUBLE: u8 = 6;
    pub const BYTE_ARRAY: u8 = 7;
    pub const STRING: u8 = 8;
    pub const LIST: u8 = 9;
    pub const COMPOUND: u8 = 10;
    pub const INT_ARRAY: u8 = 11;
    pub const LONG_ARRAY: u8 = 12;
}

// ---------------------------------------------------------------------------
// Owned data model
// ---------------------------------------------------------------------------

/// An owned NBT value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Signed byte (`TAG_Byte`).
    Byte(i8),
    /// Signed short (`TAG_Short`).
    Short(i16),
    /// Signed int (`TAG_Int`).
    Int(i32),
    /// Signed long (`TAG_Long`).
    Long(i64),
    /// 32-bit float (`TAG_Float`).
    Float(f32),
    /// 64-bit float (`TAG_Double`).
    Double(f64),
    /// `TAG_Byte_Array`.
    ByteArray(Vec<i8>),
    /// UTF-8 string, decoded from Java modified UTF-8 (`TAG_String`).
    String(String),
    /// Homogeneous list (`TAG_List`).
    List(Vec<Value>),
    /// Compound (`TAG_Compound`).
    Compound(Compound),
    /// `TAG_Int_Array`.
    IntArray(Vec<i32>),
    /// `TAG_Long_Array`.
    LongArray(Vec<i64>),
}

impl Value {
    /// The NBT tag id for this value (`TAG_End` is never returned).
    pub fn tag_id(&self) -> u8 {
        match self {
            Self::Byte(_) => tag::BYTE,
            Self::Short(_) => tag::SHORT,
            Self::Int(_) => tag::INT,
            Self::Long(_) => tag::LONG,
            Self::Float(_) => tag::FLOAT,
            Self::Double(_) => tag::DOUBLE,
            Self::ByteArray(_) => tag::BYTE_ARRAY,
            Self::String(_) => tag::STRING,
            Self::List(_) => tag::LIST,
            Self::Compound(_) => tag::COMPOUND,
            Self::IntArray(_) => tag::INT_ARRAY,
            Self::LongArray(_) => tag::LONG_ARRAY,
        }
    }
}

/// An ordered NBT compound.
///
/// Entry order is preserved exactly (deterministic re-serialization); lookups
/// are a linear scan, which is fine for the tag counts we handle.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Compound {
    entries: Vec<(Box<str>, Value)>,
}

impl Compound {
    /// An empty compound.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a compound from entries (order preserved).
    pub fn from_entries(entries: Vec<(Box<str>, Value)>) -> Self {
        Self { entries }
    }

    /// Inserts or replaces a value, keeping position on replace.
    pub fn insert(&mut self, key: impl Into<Box<str>>, value: Value) {
        let key = key.into();
        if let Some(slot) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = value;
        } else {
            self.entries.push((key, value));
        }
    }

    /// Looks up a value.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries
            .iter()
            .find(|(k, _)| k.as_ref() == key)
            .map(|(_, v)| v)
    }

    /// Looks up an integer value, accepting `Byte`/`Short`/`Int`/`Long`.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.get(key)? {
            Value::Byte(v) => Some(i64::from(*v)),
            Value::Short(v) => Some(i64::from(*v)),
            Value::Int(v) => Some(i64::from(*v)),
            Value::Long(v) => Some(*v),
            _ => None,
        }
    }

    /// Looks up a string value.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Looks up a nested compound.
    pub fn get_compound(&self, key: &str) -> Option<&Compound> {
        match self.get(key)? {
            Value::Compound(c) => Some(c),
            _ => None,
        }
    }

    /// Looks up a list value.
    pub fn get_list(&self, key: &str) -> Option<&[Value]> {
        match self.get(key)? {
            Value::List(items) => Some(items),
            _ => None,
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the compound has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterates entries in stored order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries.iter().map(|(k, v)| (k.as_ref(), v))
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from NBT read/write.
#[derive(Debug, Error)]
pub enum NbtError {
    /// The payload ended mid-value.
    #[error("unexpected end of NBT payload")]
    UnexpectedEof,
    /// The stream failed at the I/O layer.
    #[error("I/O error reading NBT: {0}")]
    Io(#[from] std::io::Error),
    /// A tag id outside `0..=12` was encountered.
    #[error("unknown NBT tag id {0}")]
    UnknownTag(u8),
    /// The root tag was not `TAG_Compound`.
    #[error("NBT root tag must be TAG_Compound (10), got {0}")]
    RootNotCompound(u8),
    /// A string was not valid Java modified UTF-8.
    #[error("string is not valid Java modified UTF-8")]
    InvalidModifiedUtf8,
    /// A string exceeded the format's 65535-byte name/string limit.
    #[error("string is too long for NBT ({} bytes, limit 65535)", .0)]
    StringTooLong(usize),
    /// A list/array declared a negative length.
    #[error("negative NBT length {0}")]
    NegativeLength(i32),
    /// A list declared `TAG_End` as its element type but had elements, or a
    /// non-empty list's elements disagreed with its declared type.
    #[error("invalid NBT list element type {0}")]
    InvalidListType(u8),
    /// Nesting exceeded [`MAX_DEPTH`].
    #[error("NBT nesting deeper than {MAX_DEPTH}")]
    DepthLimit,
}

// ---------------------------------------------------------------------------
// Byte-level helpers
// ---------------------------------------------------------------------------

/// Bounds-checked reader over an in-memory NBT payload.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    depth: u32,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            depth: 0,
        }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], NbtError> {
        let end = self.pos.checked_add(count).ok_or(NbtError::UnexpectedEof)?;
        let slice = self
            .data
            .get(self.pos..end)
            .ok_or(NbtError::UnexpectedEof)?;
        self.pos = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, NbtError> {
        Ok(self.take(1)?[0])
    }

    fn read_i8(&mut self) -> Result<i8, NbtError> {
        Ok(self.read_u8()? as i8)
    }

    fn read_u16(&mut self) -> Result<u16, NbtError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn read_i32(&mut self) -> Result<i32, NbtError> {
        let b = self.take(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_i64(&mut self) -> Result<i64, NbtError> {
        let b = self.take(8)?;
        Ok(i64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn read_f32(&mut self) -> Result<f32, NbtError> {
        Ok(f32::from_bits(self.read_i32()? as u32))
    }

    fn read_f64(&mut self) -> Result<f64, NbtError> {
        Ok(f64::from_bits(self.read_i64()? as u64))
    }

    /// Reads a length-prefixed Java modified UTF-8 string.
    fn read_string(&mut self) -> Result<String, NbtError> {
        let len = self.read_u16()?;
        let bytes = self.take(usize::from(len))?;
        decode_mutf8(bytes)
    }

    /// Converts a declared element `count` into a capacity hint that never
    /// trusts the payload (a corrupt length must not allocate gigabytes).
    fn count_to_capacity(count: i32) -> Result<usize, NbtError> {
        if count < 0 {
            return Err(NbtError::NegativeLength(count));
        }
        Ok((count as usize).min(1024))
    }

    fn enter(&mut self) -> Result<(), NbtError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(NbtError::DepthLimit);
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    /// Reads a tag payload (name already consumed).
    fn read_payload(&mut self, id: u8) -> Result<Value, NbtError> {
        Ok(match id {
            tag::BYTE => Value::Byte(self.read_i8()?),
            tag::SHORT => Value::Short(self.read_u16()? as i16),
            tag::INT => Value::Int(self.read_i32()?),
            tag::LONG => Value::Long(self.read_i64()?),
            tag::FLOAT => Value::Float(self.read_f32()?),
            tag::DOUBLE => Value::Double(self.read_f64()?),
            tag::BYTE_ARRAY => {
                let len = self.read_i32()?;
                let cap = Self::count_to_capacity(len)?;
                let mut out = Vec::with_capacity(cap);
                for _ in 0..len {
                    out.push(self.read_i8()?);
                }
                Value::ByteArray(out)
            }
            tag::STRING => Value::String(self.read_string()?),
            tag::LIST => {
                let elem_id = self.read_u8()?;
                let len = self.read_i32()?;
                let cap = Self::count_to_capacity(len)?;
                if elem_id > tag::LONG_ARRAY {
                    return Err(NbtError::UnknownTag(elem_id));
                }
                if elem_id == tag::END && len != 0 {
                    return Err(NbtError::InvalidListType(elem_id));
                }
                self.enter()?;
                let mut items = Vec::with_capacity(cap);
                for _ in 0..len {
                    items.push(self.read_payload(elem_id)?);
                }
                self.leave();
                Value::List(items)
            }
            tag::COMPOUND => {
                self.enter()?;
                let mut compound = Compound::new();
                loop {
                    let inner_id = self.read_u8()?;
                    if inner_id == tag::END {
                        break;
                    }
                    if inner_id > tag::LONG_ARRAY {
                        return Err(NbtError::UnknownTag(inner_id));
                    }
                    let name = self.read_string()?;
                    let value = self.read_payload(inner_id)?;
                    compound.insert(name, value);
                }
                self.leave();
                Value::Compound(compound)
            }
            tag::INT_ARRAY => {
                let len = self.read_i32()?;
                let cap = Self::count_to_capacity(len)?;
                let mut out = Vec::with_capacity(cap);
                for _ in 0..len {
                    out.push(self.read_i32()?);
                }
                Value::IntArray(out)
            }
            tag::LONG_ARRAY => {
                let len = self.read_i32()?;
                let cap = Self::count_to_capacity(len)?;
                let mut out = Vec::with_capacity(cap);
                for _ in 0..len {
                    out.push(self.read_i64()?);
                }
                Value::LongArray(out)
            }
            other => return Err(NbtError::UnknownTag(other)),
        })
    }
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// Parses a Java-format NBT root compound from `bytes`.
///
/// Returns the root name (usually empty for `level.dat`, preserved verbatim)
/// and the root compound. Trailing bytes after the root tag are ignored, as
/// some writers pad region payloads.
pub fn read_bytes(bytes: &[u8]) -> Result<(Box<str>, Compound), NbtError> {
    let mut reader = Reader::new(bytes);
    let root_id = reader.read_u8()?;
    if root_id != tag::COMPOUND {
        return Err(NbtError::RootNotCompound(root_id));
    }
    let name = reader.read_string()?;
    let value = reader.read_payload(tag::COMPOUND)?;
    match value {
        Value::Compound(compound) => Ok((name.into_boxed_str(), compound)),
        _ => unreachable!("read_payload(TAG_Compound) returns a compound"),
    }
}

/// Reads a Java-format NBT root compound from `reader`.
///
/// The stream is consumed to end-of-input before parsing, which matches how
/// NBT is stored (one document per file/member).
pub fn read(reader: &mut dyn Read) -> Result<(Box<str>, Compound), NbtError> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    read_bytes(&bytes)
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

/// Writes `root_name`/`compound` as Java-format NBT bytes.
pub fn write(root_name: &str, compound: &Compound) -> Result<Vec<u8>, NbtError> {
    let mut out = Vec::new();
    write_to(&mut out, root_name, compound)?;
    Ok(out)
}

/// Writes `root_name`/`compound` as Java-format NBT to `writer`.
pub fn write_to(
    writer: &mut dyn Write,
    root_name: &str,
    compound: &Compound,
) -> Result<(), NbtError> {
    writer.write_all(&[tag::COMPOUND])?;
    write_string(writer, root_name)?;
    for (key, value) in compound.iter() {
        writer.write_all(&[value.tag_id()])?;
        write_string(writer, key)?;
        write_payload(writer, value)?;
    }
    writer.write_all(&[tag::END])?;
    Ok(())
}

fn write_payload(writer: &mut dyn Write, value: &Value) -> Result<(), NbtError> {
    match value {
        Value::Byte(v) => writer.write_all(&[*v as u8])?,
        Value::Short(v) => writer.write_all(&v.to_be_bytes())?,
        Value::Int(v) => writer.write_all(&v.to_be_bytes())?,
        Value::Long(v) => writer.write_all(&v.to_be_bytes())?,
        Value::Float(v) => writer.write_all(&v.to_bits().to_be_bytes())?,
        Value::Double(v) => writer.write_all(&v.to_bits().to_be_bytes())?,
        Value::ByteArray(items) => {
            write_length(writer, items.len(), |w| {
                for item in items {
                    w.write_all(&[*item as u8])?;
                }
                Ok(())
            })?;
        }
        Value::String(s) => write_string(writer, s)?,
        Value::List(items) => {
            let elem_id = match items.first() {
                Some(first) => first.tag_id(),
                None => tag::END,
            };
            writer.write_all(&[elem_id])?;
            write_length(writer, items.len(), |w| {
                for item in items {
                    if item.tag_id() != elem_id {
                        return Err(NbtError::InvalidListType(item.tag_id()));
                    }
                    write_payload(w, item)?;
                }
                Ok(())
            })?;
        }
        Value::Compound(c) => {
            for (key, inner) in c.iter() {
                writer.write_all(&[inner.tag_id()])?;
                write_string(writer, key)?;
                write_payload(writer, inner)?;
            }
            writer.write_all(&[tag::END])?;
        }
        Value::IntArray(items) => {
            write_length(writer, items.len(), |w| {
                for item in items {
                    w.write_all(&item.to_be_bytes())?;
                }
                Ok(())
            })?;
        }
        Value::LongArray(items) => {
            write_length(writer, items.len(), |w| {
                for item in items {
                    w.write_all(&item.to_be_bytes())?;
                }
                Ok(())
            })?;
        }
    }
    Ok(())
}

/// Writes an `i32` length prefix, rejecting lengths that overflow it.
fn write_length(
    writer: &mut dyn Write,
    len: usize,
    body: impl FnOnce(&mut dyn Write) -> Result<(), NbtError>,
) -> Result<(), NbtError> {
    let len = i32::try_from(len).map_err(|_| NbtError::StringTooLong(len))?;
    writer.write_all(&len.to_be_bytes())?;
    body(writer)
}

/// Writes a `u16`-length-prefixed Java modified UTF-8 string.
fn write_string(writer: &mut dyn Write, s: &str) -> Result<(), NbtError> {
    let encoded = encode_mutf8(s);
    let len = u16::try_from(encoded.len()).map_err(|_| NbtError::StringTooLong(encoded.len()))?;
    writer.write_all(&len.to_be_bytes())?;
    writer.write_all(&encoded)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Java modified UTF-8
// ---------------------------------------------------------------------------

/// Encodes a Rust string as Java modified UTF-8.
///
/// `U+0000` becomes `C0 80`; supplementary characters are written as their
/// two surrogate halves (3 bytes each), exactly as `DataOutputStream.writeUTF`
/// does. This makes output byte-identical to Java's for every input that can
/// be represented in NBT.
fn encode_mutf8(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() + 2);
    for unit in s.encode_utf16() {
        match unit {
            0x0001..=0x007F => out.push(unit as u8),
            0x0000 | 0x0080..=0x07FF => {
                out.push(0xC0 | (unit >> 6) as u8);
                out.push(0x80 | (unit & 0x3F) as u8);
            }
            _ => {
                out.push(0xE0 | (unit >> 12) as u8);
                out.push(0x80 | ((unit >> 6) & 0x3F) as u8);
                out.push(0x80 | (unit & 0x3F) as u8);
            }
        }
    }
    out
}

/// Decodes Java modified UTF-8 into a Rust string.
///
/// Unpaired UTF-16 surrogates cannot be represented in a Rust `String` and are
/// rejected rather than silently replaced.
fn decode_mutf8(bytes: &[u8]) -> Result<String, NbtError> {
    let mut units: Vec<u16> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let first = bytes[index];
        if first & 0x80 == 0 {
            units.push(u16::from(first));
            index += 1;
        } else if first & 0xE0 == 0xC0 {
            let second = *bytes.get(index + 1).ok_or(NbtError::UnexpectedEof)?;
            if second & 0xC0 != 0x80 {
                return Err(NbtError::InvalidModifiedUtf8);
            }
            units.push((u16::from(first & 0x1F) << 6) | u16::from(second & 0x3F));
            index += 2;
        } else if first & 0xF0 == 0xE0 {
            let second = *bytes.get(index + 1).ok_or(NbtError::UnexpectedEof)?;
            let third = *bytes.get(index + 2).ok_or(NbtError::UnexpectedEof)?;
            if second & 0xC0 != 0x80 || third & 0xC0 != 0x80 {
                return Err(NbtError::InvalidModifiedUtf8);
            }
            units.push(
                (u16::from(first & 0x0F) << 12)
                    | (u16::from(second & 0x3F) << 6)
                    | u16::from(third & 0x3F),
            );
            index += 3;
        } else {
            return Err(NbtError::InvalidModifiedUtf8);
        }
    }
    String::from_utf16(&units).map_err(|_| NbtError::InvalidModifiedUtf8)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Java-format bytes for `"" { key: "value" }` — hand-computed golden:
    /// TAG_Compound root (0x0A) with empty name, then TAG_String "key" =
    /// "value", then TAG_End.
    const GOLDEN_BYTES: &[u8] = &[
        0x0A, 0x00, 0x00, // root compound, empty name
        0x08, 0x00, 0x03, b'k', b'e', b'y', // TAG_String "key"
        0x00, 0x05, b'v', b'a', b'l', b'u', b'e', // "value"
        0x00, // TAG_End
    ];

    #[test]
    fn write_matches_java_format_golden_bytes() {
        let mut root = Compound::new();
        root.insert("key", Value::String("value".to_string()));
        assert_eq!(write("", &root).expect("writable"), GOLDEN_BYTES);
    }

    #[test]
    fn read_parses_java_format_golden_bytes() {
        let (root_name, root) = read(&mut std::io::Cursor::new(GOLDEN_BYTES)).expect("valid");
        assert_eq!(root_name.as_ref(), "");
        assert_eq!(root.get("key"), Some(&Value::String("value".to_string())));
        assert_eq!(root.len(), 1);
    }

    #[test]
    fn round_trips_every_tag_type() {
        let mut nested = Compound::new();
        nested.insert("inner", Value::Byte(7));

        let mut root = Compound::new();
        root.insert("byte", Value::Byte(i8::MIN));
        root.insert("short", Value::Short(i16::MAX));
        root.insert("int", Value::Int(i32::MIN));
        root.insert("long", Value::Long(i64::MIN));
        root.insert("float", Value::Float(0.1));
        root.insert("double", Value::Double(-1.0 / 3.0));
        root.insert("bytes", Value::ByteArray(vec![-128, 0, 127]));
        root.insert("string", Value::String("héllo — wörld ✔".to_string()));
        root.insert(
            "list",
            Value::List(vec![
                Value::Compound(nested),
                Value::Compound(Compound::new()),
            ]),
        );
        root.insert("empty_list", Value::List(Vec::new()));
        root.insert("empty_compound", Value::Compound(Compound::new()));
        root.insert("ints", Value::IntArray(vec![1, -2, 3]));
        root.insert("longs", Value::LongArray(vec![i64::MAX, 0, i64::MIN]));

        let bytes = write("root", &root).expect("writable");
        let (name, reparsed) = read_bytes(&bytes).expect("valid");
        assert_eq!(name.as_ref(), "root");
        assert_eq!(reparsed, root);
    }

    #[test]
    fn round_trips_string_edge_cases() {
        // NUL must take the two-byte modified-UTF-8 form; an astral character
        // must be two 3-byte surrogates; both must survive a round trip.
        let cases = ["", "a\0b", "𝄞 clef", "\u{FFFF}", "日本語"];
        for case in cases {
            let mut root = Compound::new();
            root.insert("s", Value::String(case.to_string()));
            let bytes = write("", &root).expect("writable");
            let (_, reparsed) = read_bytes(&bytes).expect("valid");
            assert_eq!(reparsed.get_str("s"), Some(case));
        }
    }

    #[test]
    fn nul_byte_uses_java_encoding() {
        let mut root = Compound::new();
        root.insert("s", Value::String("\0".to_string()));
        let bytes = write("", &root).expect("writable");
        assert!(bytes.windows(2).any(|w| w == [0xC0, 0x80]));
        // No raw NUL inside the string payload (the trailing TAG_End is the
        // last byte and is a legitimate 0x00).
        let payload_start = 1 + 2 + 1 + 3 + 2; // header + name + u16 length
        assert!(!bytes[payload_start..bytes.len() - 1].contains(&0x00));
    }

    #[test]
    fn preserves_entry_order_and_replace_in_place() {
        let mut c = Compound::new();
        c.insert("a", Value::Int(1));
        c.insert("b", Value::Int(2));
        c.insert("a", Value::Int(3)); // replace, keeps first position
        let order: Vec<&str> = c.iter().map(|(k, _)| k).collect();
        assert_eq!(order, vec!["a", "b"]);
        assert_eq!(c.get("a"), Some(&Value::Int(3)));
    }

    #[test]
    fn corrupt_input_is_an_error_not_a_panic() {
        // Truncated stream.
        assert!(read_bytes(&GOLDEN_BYTES[..8]).is_err());
        // Bad root tag id.
        let mut bad = GOLDEN_BYTES.to_vec();
        bad[0] = 0x42;
        assert!(read_bytes(&bad).is_err());
        // Empty input.
        assert!(read_bytes(&[]).is_err());
        // Unknown inner tag id.
        let mut bad = GOLDEN_BYTES.to_vec();
        bad[3] = 0x7F;
        assert!(read_bytes(&bad).is_err());
        // Trailing garbage after a complete root tag is tolerated.
        let mut padded = GOLDEN_BYTES.to_vec();
        padded.extend_from_slice(&[0xDE, 0xAD]);
        assert!(read_bytes(&padded).is_ok());
    }

    #[test]
    fn absurd_lengths_do_not_allocate() {
        // TAG_Compound root, empty name, TAG_Int_Array claiming 2^31-1 items.
        let bytes = [
            0x0A, 0x00, 0x00, 0x0B, 0x00, 0x01, b'a', 0x7F, 0xFF, 0xFF, 0xFF,
        ];
        assert!(read_bytes(&bytes).is_err());
    }

    #[test]
    fn depth_limit_is_enforced() {
        // Build MAX_DEPTH + 2 nested compounds of one element each.
        let bytes_len = MAX_DEPTH as usize + 3;
        let mut bytes = vec![0x0A, 0x00, 0x00];
        for _ in 0..(MAX_DEPTH + 2) {
            bytes.extend_from_slice(&[0x0A, 0x00, 0x01, b'a']);
        }
        bytes.extend(std::iter::repeat_n(0x00u8, bytes_len));
        assert!(matches!(read_bytes(&bytes), Err(NbtError::DepthLimit)));
    }

    #[test]
    fn rejects_heterogeneous_list_on_write() {
        let mut root = Compound::new();
        root.insert("mixed", Value::List(vec![Value::Int(1), Value::Long(2)]));
        assert!(matches!(
            write("", &root),
            Err(NbtError::InvalidListType(_))
        ));
    }

    // --- property tests -----------------------------------------------------

    fn arb_string() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            // `char` cannot hold a surrogate, so any generated char is fine.
            prop::char::range('\u{1}', '\u{10FFFF}'),
            0..4,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    fn arb_list(depth: u32) -> impl Strategy<Value = Value> {
        // NBT lists are homogeneous: generate one element strategy per list.
        prop_oneof![
            proptest::collection::vec(any::<i32>(), 0..3)
                .prop_map(|v| Value::List(v.into_iter().map(Value::Int).collect())),
            proptest::collection::vec(any::<i64>(), 0..3)
                .prop_map(|v| Value::List(v.into_iter().map(Value::Long).collect())),
            proptest::collection::vec(arb_string(), 0..3)
                .prop_map(|v| Value::List(v.into_iter().map(Value::String).collect())),
            arb_value(depth - 1)
                .prop_flat_map(|value| proptest::collection::vec(Just(value), 0..3))
                .prop_map(Value::List),
        ]
    }

    fn arb_value(depth: u32) -> BoxedStrategy<Value> {
        let leaf = prop_oneof![
            any::<i8>().prop_map(Value::Byte),
            any::<i16>().prop_map(Value::Short),
            any::<i32>().prop_map(Value::Int),
            any::<i64>().prop_map(Value::Long),
            (-1.0e30f32..1.0e30f32).prop_map(Value::Float),
            (-1.0e300f64..1.0e300f64).prop_map(Value::Double),
            proptest::collection::vec(any::<i8>(), 0..4).prop_map(Value::ByteArray),
            arb_string().prop_map(Value::String),
            proptest::collection::vec(any::<i32>(), 0..4).prop_map(Value::IntArray),
            proptest::collection::vec(any::<i64>(), 0..4).prop_map(Value::LongArray),
        ]
        .boxed();
        if depth == 0 {
            return leaf;
        }
        let nested = prop_oneof![
            arb_list(depth).boxed(),
            proptest::collection::vec((arb_string(), arb_value(depth - 1)), 0..3)
                .prop_map(|entries| {
                    let mut compound = Compound::new();
                    for (key, value) in entries {
                        compound.insert(key, value);
                    }
                    Value::Compound(compound)
                })
                .boxed(),
        ]
        .boxed();
        prop_oneof![leaf, nested].boxed()
    }

    proptest! {
        #[test]
        fn write_read_is_identity(root in proptest::collection::vec((arb_string(), arb_value(2)), 0..4)) {
            // Duplicate keys collapse on insert, exactly as a Java
            // `HashMap`-backed compound does; compare after deduping.
            let mut compound = Compound::new();
            for (key, value) in root {
                compound.insert(key, value);
            }
            let bytes = write("level", &compound).expect("writable");
            let (name, reparsed) = read_bytes(&bytes).expect("valid");
            prop_assert_eq!(name.as_ref(), "level");
            prop_assert_eq!(reparsed, compound);
        }

        #[test]
        fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
            let _ = read_bytes(&bytes);
        }
    }
}
