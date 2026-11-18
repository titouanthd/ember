//! `AssetId` (runtime) and `Uid` (persistent).

use crate::AssetError;
use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct AssetId(u64);

impl AssetId {
    pub fn invalid() -> Self {
        Self(0)
    }

    pub fn is_valid(self) -> bool {
        self.0 != 0
    }

    pub(crate) fn new(raw: u64) -> Self {
        Self(raw)
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, PartialOrd, Ord)]
pub struct Uid(String);

impl Uid {
    pub fn parse(s: &str) -> Result<Self, AssetError> {
        match s.strip_prefix("uid://") {
            Some(rest) if !rest.is_empty() && !rest.contains(char::is_whitespace) => {
                Ok(Self(s.to_string()))
            }
            _ => Err(AssetError::InvalidUid(s.to_string())),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Uid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
