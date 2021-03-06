mod twitch;

pub use twitch::*;

use serde::{de, Deserialize, Deserializer};
use std::str::FromStr;

fn str_to_u64<'de, D>(d: D) -> std::result::Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(d)?;
    u64::from_str(s).map_err(de::Error::custom)
}

fn str_to_maybe_u64<'de, D>(d: D) -> std::result::Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(d)?;
    if s.is_empty() {
        Ok(None)
    } else {
        u64::from_str(s).map(Some).map_err(de::Error::custom)
    }
}
