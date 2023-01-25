use core::{
    borrow::Borrow,
    fmt,
    hash::{BuildHasher, Hash},
};
use std::collections::{BTreeMap, HashMap};

use crate::{FormatKey, FormatKeyError};

impl<K, V, S> FormatKey for HashMap<K, V, S>
where
    K: Borrow<str> + Eq + Hash,
    V: fmt::Display,
    S: BuildHasher,
{
    fn fmt(&self, key: &str, f: &mut fmt::Formatter<'_>) -> Result<(), FormatKeyError> {
        match self.get(key) {
            Some(v) => v.fmt(f).map_err(FormatKeyError::Fmt),
            None => Err(FormatKeyError::UnknownKey),
        }
    }
}

impl<K, V> FormatKey for BTreeMap<K, V>
where
    K: Borrow<str> + Ord,
    V: fmt::Display,
{
    fn fmt(&self, key: &str, f: &mut fmt::Formatter<'_>) -> Result<(), FormatKeyError> {
        match self.get(key) {
            Some(v) => v.fmt(f).map_err(FormatKeyError::Fmt),
            None => Err(FormatKeyError::UnknownKey),
        }
    }
}
