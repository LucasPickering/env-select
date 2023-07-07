//! Config serialization and deserialization

use crate::config::{ValueSource, ValueSourceInner};
use serde::{
    de::{self, value::MapAccessDeserializer, MapAccess, Visitor},
    Deserialize, Deserializer,
};

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
