use core::{
    borrow::Borrow,
    fmt,
    hash::{BuildHasher, Hash},
};
use std::collections::{BTreeMap, HashMap};

use crate::{FormatKey, FormatKeyError, FormatArgs, FormatError};

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

pub fn format<'a, F: FormatKey>(s: &'a str, f: &'a F) -> Result<String, FormatError<'a>> {
    use core::fmt::Write;

    let mut out = String::with_capacity(s.len() * 2);
    let fmt = FormatArgs::new(s, f);
    match write!(out, "{fmt}") {
        Ok(()) => Ok(out),
        Err(_) => {
            fmt.status()?;
            panic!("a formatting trait implementation returned an error");
        }
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use crate::{FormatError, FormatKey, FormatKeyError};

    struct Message;
    impl FormatKey for Message {
        fn fmt(&self, key: &str, f: &mut fmt::Formatter<'_>) -> Result<(), FormatKeyError> {
            match key {
                "recipient" => f.write_str("World").map_err(FormatKeyError::Fmt),
                "time_descriptor" => f.write_str("morning").map_err(FormatKeyError::Fmt),
                _ => Err(FormatKeyError::UnknownKey),
            }
        }
    }

    #[test]
    fn happy_path() {
        let format_str = "Hello, {recipient}. Hope you are having a nice {time_descriptor}.";
        let expected = "Hello, World. Hope you are having a nice morning.";
        assert_eq!(crate::format(format_str, &Message).unwrap(), expected);
    }

    #[test]
    fn missing_key() {
        let format_str = "Hello, {recipient}. Hope you are having a nice {time_descriptr}.";
        assert_eq!(
            crate::format(format_str, &Message),
            Err(FormatError::Key("time_descriptr"))
        );
    }

    #[test]
    fn failed_parsing() {
        let format_str = "Hello, {recipient}. Hope you are having a nice {time_descriptor.";
        assert_eq!(
            crate::format(format_str, &Message),
            Err(FormatError::Parse("time_descriptor."))
        );
    }

    #[test]
    fn escape_brackets() {
        let format_str = "You can make custom formatting terms using {{foo}!";
        let expected = "You can make custom formatting terms using {foo}!";
        assert_eq!(crate::format(format_str, &Message).unwrap(), expected);
    }
}
