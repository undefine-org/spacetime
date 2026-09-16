use crate::lib::*;

// Used from generated code to buffer the contents of the Deserializer when
// deserializing untagged enums and internally tagged enums.
//
// Not public API. Use serde-value instead.
//
// Obsoleted by format-specific buffer types (https://github.com/serde-rs/serde/pull/2912).
#[doc(hidden)]
pub enum Content<'de> {
    Bool(bool),

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),

    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),

    F32(f32),
    F64(f64),

    Char(char),
    String(String),
    Str(&'de str),
    ByteBuf(Vec<u8>),
    Bytes(&'de [u8]),

    None,
    Some(Box<Content<'de>>),

    Unit,
    Newtype(Box<Content<'de>>),
    Seq(Vec<Content<'de>>),
    Map(Vec<(Content<'de>, Content<'de>)>),
}

// Backward-compat: old SWC crates (swc_config 3.0.0, swc_common 9.2.0, ast_node 3.0.0)
// reference <Content as Deserialize>::deserialize(), which was removed in the serde_core split.
// We restore it here since both Content and Deserialize live in serde_core.

use crate::de;
use super::size_hint;

struct ContentVisitor<'de> {
    value: PhantomData<Content<'de>>,
}

impl<'de> ContentVisitor<'de> {
    fn new() -> Self {
        ContentVisitor { value: PhantomData }
    }
}

impl<'de> de::Deserialize<'de> for Content<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(ContentVisitor::new())
    }
}

impl<'de> de::Visitor<'de> for ContentVisitor<'de> {
    type Value = Content<'de>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("any value")
    }

    fn visit_bool<F>(self, value: bool) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Bool(value))
    }

    fn visit_i8<F>(self, value: i8) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::I8(value))
    }

    fn visit_i16<F>(self, value: i16) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::I16(value))
    }

    fn visit_i32<F>(self, value: i32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::I32(value))
    }

    fn visit_i64<F>(self, value: i64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::I64(value))
    }

    fn visit_u8<F>(self, value: u8) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::U8(value))
    }

    fn visit_u16<F>(self, value: u16) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::U16(value))
    }

    fn visit_u32<F>(self, value: u32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::U32(value))
    }

    fn visit_u64<F>(self, value: u64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::U64(value))
    }

    fn visit_f32<F>(self, value: f32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::F32(value))
    }

    fn visit_f64<F>(self, value: f64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::F64(value))
    }

    fn visit_char<F>(self, value: char) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Char(value))
    }

    fn visit_str<F>(self, value: &str) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::String(value.into()))
    }

    fn visit_borrowed_str<F>(self, value: &'de str) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Str(value))
    }

    fn visit_string<F>(self, value: String) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::String(value))
    }

    fn visit_bytes<F>(self, value: &[u8]) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::ByteBuf(value.into()))
    }

    fn visit_borrowed_bytes<F>(self, value: &'de [u8]) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Bytes(value))
    }

    fn visit_byte_buf<F>(self, value: Vec<u8>) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::ByteBuf(value))
    }

    fn visit_unit<F>(self) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Unit)
    }

    fn visit_none<F>(self) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::None)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let v = de::Deserialize::deserialize(deserializer)?;
        Ok(Content::Some(Box::new(v)))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let v = de::Deserialize::deserialize(deserializer)?;
        Ok(Content::Newtype(Box::new(v)))
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::SeqAccess<'de>,
    {
        let mut vec =
            Vec::<Content>::with_capacity(size_hint::cautious::<Content>(visitor.size_hint()));
        while let Some(e) = visitor.next_element()? {
            vec.push(e);
        }
        Ok(Content::Seq(vec))
    }

    fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::MapAccess<'de>,
    {
        let mut vec =
            Vec::<(Content, Content)>::with_capacity(
                size_hint::cautious::<(Content, Content)>(visitor.size_hint()),
            );
        while let Some(kv) = visitor.next_entry()? {
            vec.push(kv);
        }
        Ok(Content::Map(vec))
    }

    fn visit_enum<V>(self, _visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::EnumAccess<'de>,
    {
        Err(de::Error::custom(
            "untagged and internally tagged enums do not support enum input",
        ))
    }
}
