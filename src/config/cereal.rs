//! Config serialization and deserialization

use crate::config::{Name, ProfileReference, ValueSource, ValueSourceInner};
use serde::{
    de::{self, value::MapAccessDeserializer, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::str::FromStr;

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
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ValueSource::from_literal(value))
            }

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
