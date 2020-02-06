use std::fmt::Display;
use std::str::FromStr;

use serde::de::{Deserialize, Deserializer};

pub fn deser_parse<'de, E: Display, T: FromStr<Err = E> + Sized, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<T, D::Error> {
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

pub fn deser_parse_opt<'de, E: Display, T: FromStr<Err = E> + Sized, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    let s: Option<String> = Deserialize::deserialize(deserializer)?;
    s.map(|s| s.parse().map_err(serde::de::Error::custom))
        .transpose()
}
