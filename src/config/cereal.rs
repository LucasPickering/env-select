//! Config serialization and deserialization

use crate::config::{Name, ProfileReference, ValueSource, ValueSourceInner};
use serde::{
    de::{self, value::MapAccessDeserializer, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::str::FromStr;

macro_rules! visit_primitive {
    ($func:ident, $type:ty) => {
        fn $func<E>(self, value: $type) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(ValueSource::from_literal(value))
        }
    };
}

// Custom deserialization for ValueSource, to support simple string OR map
impl<'de> Deserialize<'de> for ValueSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueSourceWrapperVisitor;

        impl<'de> Visitor<'de> for ValueSourceWrapperVisitor {
            type Value = ValueSource;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str("string, boolean, number, or map")
            }

            visit_primitive!(visit_bool, bool);
            visit_primitive!(visit_u64, u64);
            visit_primitive!(visit_u128, u128);
            visit_primitive!(visit_i64, i64);
            visit_primitive!(visit_i128, i128);
            visit_primitive!(visit_f64, f64);
            visit_primitive!(visit_str, &str);

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                Ok(ValueSource(<ValueSourceInner as Deserialize>::deserialize(
                    MapAccessDeserializer::new(map),
                )?))
            }
        }

        deserializer.deserialize_any(ValueSourceWrapperVisitor)
    }
}

// Deserialize Name using its FromStr
impl<'de> Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}

// Serialize ProfileReference using its Display
impl Serialize for ProfileReference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// Deserialize ProfileReference using its FromStr
impl<'de> Deserialize<'de> for ProfileReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}
