use std::fmt;

use serde::de::value::{Error, MapDeserializer, SeqDeserializer};
use serde::de::{
    self, Deserialize, DeserializeSeed, Deserializer, EnumAccess, IntoDeserializer, MapAccess,
    SeqAccess, Unexpected, VariantAccess, Visitor,
};
use serde::forward_to_deserialize_any;

#[derive(Debug)]
pub(crate) enum Raw {
    Null,
    Bool(bool),
    Unsigned(u64),
    Signed(i64),
    Float(f64),
    Text(String),
    List(Vec<Raw>),
    Object(Vec<(String, Raw)>),
}

impl Raw {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Raw::Null => "null",
            Raw::Bool(_) => "a boolean",
            Raw::Unsigned(_) | Raw::Signed(_) | Raw::Float(_) => "a number",
            Raw::Text(_) => "a string",
            Raw::List(_) => "a list",
            Raw::Object(_) => "an object",
        }
    }
}

struct RawVisitor;

impl<'de> Visitor<'de> for RawVisitor {
    type Value = Raw;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Raw, E> {
        Ok(Raw::Null)
    }

    fn visit_none<E: de::Error>(self) -> Result<Raw, E> {
        Ok(Raw::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Raw, D::Error> {
        deserializer.deserialize_any(RawVisitor)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Raw, E> {
        Ok(Raw::Bool(value))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Raw, E> {
        Ok(Raw::Unsigned(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Raw, E> {
        Ok(match u64::try_from(value) {
            Ok(unsigned) => Raw::Unsigned(unsigned),
            Err(_) => Raw::Signed(value),
        })
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Raw, E> {
        Ok(Raw::Float(value))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Raw, E> {
        Ok(Raw::Text(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Raw, E> {
        Ok(Raw::Text(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Raw, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element::<Raw>()? {
            items.push(item);
        }
        Ok(Raw::List(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Raw, A::Error> {
        let mut pairs = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            pairs.push((key, map.next_value::<Raw>()?));
        }
        Ok(Raw::Object(pairs))
    }
}

impl<'de> Deserialize<'de> for Raw {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Raw, D::Error> {
        deserializer.deserialize_any(RawVisitor)
    }
}

impl<'de> IntoDeserializer<'de, Error> for Raw {
    type Deserializer = Raw;

    fn into_deserializer(self) -> Raw {
        self
    }
}

impl<'de> Deserializer<'de> for Raw {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self {
            Raw::Null => visitor.visit_unit(),
            Raw::Bool(value) => visitor.visit_bool(value),
            Raw::Unsigned(value) => visitor.visit_u64(value),
            Raw::Signed(value) => visitor.visit_i64(value),
            Raw::Float(value) => visitor.visit_f64(value),
            Raw::Text(value) => visitor.visit_string(value),
            Raw::List(items) => {
                let mut seq = SeqDeserializer::new(items.into_iter());
                let value = visitor.visit_seq(&mut seq)?;
                seq.end()?;
                Ok(value)
            }
            Raw::Object(pairs) => {
                let mut map = MapDeserializer::new(pairs.into_iter());
                let value = visitor.visit_map(&mut map)?;
                map.end()?;
                Ok(value)
            }
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self {
            Raw::Null => visitor.visit_none(),
            other => visitor.visit_some(other),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self {
            Raw::Text(variant) => visitor.visit_enum(variant.into_deserializer()),
            Raw::Object(pairs) => {
                let mut pairs = pairs.into_iter();
                match (pairs.next(), pairs.next()) {
                    (Some((variant, value)), None) => {
                        visitor.visit_enum(Variant { variant, value })
                    }
                    _ => Err(de::Error::invalid_value(
                        Unexpected::Map,
                        &"an object with exactly one key",
                    )),
                }
            }
            other => Err(de::Error::invalid_type(
                Unexpected::Other(other.kind()),
                &"a string or an object with one key",
            )),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

struct Variant {
    variant: String,
    value: Raw,
}

impl<'de> EnumAccess<'de> for Variant {
    type Error = Error;
    type Variant = Raw;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Raw), Error> {
        let variant = seed.deserialize(self.variant.into_deserializer())?;
        Ok((variant, self.value))
    }
}

impl<'de> VariantAccess<'de> for Raw {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self {
            Raw::Null => Ok(()),
            other => Err(de::Error::invalid_type(
                Unexpected::Other(other.kind()),
                &"a unit variant",
            )),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(self)
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_map(visitor)
    }
}
