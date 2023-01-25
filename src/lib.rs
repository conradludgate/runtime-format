//!
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
mod alloc_impls;
#[cfg(feature = "std")]
pub use alloc_impls::*;

use core::cell::Cell;
use core::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum FormatKeyError {
    Fmt(fmt::Error),
    UnknownKey,
}

impl From<fmt::Error> for FormatKeyError {
    fn from(value: fmt::Error) -> Self {
        FormatKeyError::Fmt(value)
    }
}

pub trait FormatKey {
    fn fmt(&self, key: &str, f: &mut fmt::Formatter<'_>) -> Result<(), FormatKeyError>;

    // you might have a hard coded list of strings at compile time. This is useful for
    // [`CompiledFormatter`] to be able to determine `UnknownKey` errors early
    fn is_acceptable_key(key: &str) -> bool {
        let _key = key;
        true
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FormatError<'a> {
    Key(&'a str),
    Parse(&'a str),
}

#[cfg(feature = "std")]
pub fn format<'a, F: FormatKey>(s: &'a str, f: &'a F) -> Result<String, FormatError<'a>> {
    use core::fmt::Write;

    let mut out = String::with_capacity(s.len() * 2);
    let fmt = Formatter::new(s, f);
    match write!(out, "{fmt}") {
        Ok(()) => Ok(out),
        Err(_) => match fmt.error.take() {
            Some(err) => Err(err),
            None => panic!("a formatting trait implementation returned an error"),
        },
    }
}

#[cfg(feature = "std")]
pub struct CompiledFormatter<'a, F> {
    segments: tinyvec::TinyVec<[ParseSegment<'a>; 8]>,
    _fmt: core::marker::PhantomData<&'a F>,
}

#[cfg(feature = "std")]
impl<F> fmt::Debug for CompiledFormatter<'_, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledFormatter")
            .field("segments", &self.segments)
            .finish()
    }
}

#[cfg(feature = "std")]
impl<'a, F: FormatKey> CompiledFormatter<'a, F> {
    pub fn new(s: &'a str) -> Result<Self, FormatError<'a>> {
        let mut segments = Parser { s, is_key: false };
        let this = Self {
            segments: segments.by_ref().collect(),
            _fmt: core::marker::PhantomData,
        };

        let mut unknown_keys = this
            .segments
            .iter()
            .filter_map(|segment| match segment {
                ParseSegment::Literal(_) => None,
                ParseSegment::Key(key) => Some(key),
            })
            .filter(|key| !F::is_acceptable_key(key));

        if !segments.s.is_empty() {
            Err(FormatError::Parse(segments.s))
        } else if let Some(key) = unknown_keys.next() {
            Err(FormatError::Key(key))
        } else {
            Ok(this)
        }
    }

    pub fn with_args<'b>(&'b self, fmt: &'b F) -> CompiledFmt<'b, F> {
        CompiledFmt {
            segments: &self.segments,
            fmt,
            error: Cell::new(None),
        }
    }
}

pub struct CompiledFmt<'a, F> {
    segments: &'a [ParseSegment<'a>],
    fmt: &'a F,
    error: Cell<Option<FormatError<'a>>>,
}

impl<F: FormatKey> fmt::Display for CompiledFmt<'_, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in self.segments {
            match segment {
                ParseSegment::Literal(s) => f.write_str(s)?,
                ParseSegment::Key(key) => match self.fmt.fmt(key, f) {
                    Ok(_) => {}
                    Err(FormatKeyError::Fmt(e)) => return Err(e),
                    Err(FormatKeyError::UnknownKey) => {
                        self.error.set(Some(FormatError::Key(key)));
                        return Err(fmt::Error);
                    }
                },
            }
        }
        Ok(())
    }
}

pub struct Formatter<'a, F> {
    s: &'a str,
    fmt: &'a F,
    error: Cell<Option<FormatError<'a>>>,
}

impl<'a, F> Formatter<'a, F> {
    pub fn new(s: &'a str, fmt: &'a F) -> Self {
        Formatter {
            s,
            fmt,
            error: Cell::new(None),
        }
    }
}

impl<F: FormatKey> fmt::Display for Formatter<'_, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut segments = Parser {
            s: self.s,
            is_key: false,
        };
        for segment in &mut segments {
            match segment {
                ParseSegment::Literal(s) => f.write_str(s)?,
                ParseSegment::Key(key) => match self.fmt.fmt(key, f) {
                    Ok(_) => {}
                    Err(FormatKeyError::Fmt(e)) => return Err(e),
                    Err(FormatKeyError::UnknownKey) => {
                        self.error.set(Some(FormatError::Key(key)));
                        return Err(fmt::Error);
                    }
                },
            }
        }
        if !segments.s.is_empty() {
            self.error.set(Some(FormatError::Parse(segments.s)));
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
enum ParseSegment<'a> {
    Literal(&'a str),
    Key(&'a str),
}
impl Default for ParseSegment<'_> {
    fn default() -> Self {
        Self::Literal("")
    }
}

struct Parser<'a> {
    s: &'a str,
    is_key: bool,
}

impl<'a> Iterator for Parser<'a> {
    type Item = ParseSegment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.s.is_empty() {
            None
        } else if self.is_key {
            match self.s.strip_prefix('{') {
                // escaped
                Some(rest) => match rest.split_once('{') {
                    None => {
                        self.is_key = false;
                        Some(ParseSegment::Literal(core::mem::take(&mut self.s)))
                    }
                    Some((prefix, rest)) => {
                        let x = &self.s[..prefix.len() + 1];
                        self.s = rest;
                        Some(ParseSegment::Literal(x))
                    }
                },
                None => match self.s.split_once('}') {
                    Some((key, rest)) => {
                        self.is_key = false;
                        self.s = rest;
                        Some(ParseSegment::Key(key))
                    }
                    None => None,
                },
            }
        } else {
            match self.s.split_once('{') {
                None => Some(ParseSegment::Literal(core::mem::take(&mut self.s))),
                Some((prefix, rest)) => {
                    self.is_key = true;
                    self.s = rest;
                    Some(ParseSegment::Literal(prefix))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::{self, Write};

    use crate::{FormatError, FormatKey, FormatKeyError, Formatter};

    struct WriteShim<'a> {
        w: &'a mut [u8],
        n: usize,
    }
    impl fmt::Write for WriteShim<'_> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let remaining = self.w.len() - self.n;
            if let Some(prefix) = s.as_bytes().get(..remaining) {
                self.w[self.n..].copy_from_slice(prefix);
                self.n = self.w.len();
                Err(fmt::Error)
            } else {
                let n = self.n + s.len();
                self.w[self.n..n].copy_from_slice(s.as_bytes());
                self.n = n;
                Ok(())
            }
        }
    }

    fn format<'a, F: FormatKey>(
        s: &'a str,
        fmt: &'a F,
        f: impl FnOnce(&[u8]),
    ) -> Result<(), FormatError<'a>> {
        let mut bytes = WriteShim {
            w: &mut [0; 1024],
            n: 0,
        };
        let fmt = Formatter::new(s, fmt);
        let _ = write!(bytes, "{}", fmt);
        if let Some(err) = fmt.error.take() {
            return Err(err);
        }

        f(&bytes.w[..bytes.n]);
        Ok(())
    }

    struct Message;
    impl FormatKey for Message {
        fn fmt(&self, key: &str, f: &mut fmt::Formatter<'_>) -> Result<(), FormatKeyError> {
            match key {
                "recipient" => f.write_str("World").map_err(FormatKeyError::Fmt),
                "time_descriptor" => f.write_str("morning").map_err(FormatKeyError::Fmt),
                _ => Err(FormatKeyError::UnknownKey),
            }
        }

        fn is_acceptable_key(key: &str) -> bool {
            matches!(key, "recipient" | "time_descriptor")
        }
    }

    #[test]
    fn happy_path() {
        let format_str = "Hello, {recipient}. Hope you are having a nice {time_descriptor}.";
        let expected = "Hello, World. Hope you are having a nice morning.";
        format(format_str, &Message, |output| {
            assert_eq!(output, expected.as_bytes())
        }).unwrap();
    }

    #[test]
    fn missing_key() {
        let format_str = "Hello, {recipient}. Hope you are having a nice {time_descriptr}.";
        assert_eq!(
            format(format_str, &Message, |_| {}),
            Err(FormatError::Key("time_descriptr"))
        );
    }

    #[test]
    fn failed_parsing() {
        let format_str = "Hello, {recipient}. Hope you are having a nice {time_descriptor.";
        assert_eq!(
            format(format_str, &Message, |_| {}),
            Err(FormatError::Parse("time_descriptor."))
        );
    }

    #[test]
    fn escape_brackets() {
        let format_str = "You can make custom formatting terms using {{foo}!";
        let expected = "You can make custom formatting terms using {foo}!";
        format(format_str, &Message, |output| {
            assert_eq!(output, expected.as_bytes())
        }).unwrap();
    }
}

#[cfg(all(test, feature = "std"))]
mod std_tests {
    use core::fmt;

    use crate::{CompiledFormatter, FormatError, FormatKey, FormatKeyError};

    struct Message;
    impl FormatKey for Message {
        fn fmt(&self, key: &str, f: &mut fmt::Formatter<'_>) -> Result<(), FormatKeyError> {
            match key {
                "recipient" => f.write_str("World").map_err(FormatKeyError::Fmt),
                "time_descriptor" => f.write_str("morning").map_err(FormatKeyError::Fmt),
                _ => Err(FormatKeyError::UnknownKey),
            }
        }

        fn is_acceptable_key(key: &str) -> bool {
            matches!(key, "recipient" | "time_descriptor")
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

    #[test]
    fn compiled_happy_path() {
        let formatter = CompiledFormatter::new(
            "Hello, {recipient}. Hope you are having a nice {time_descriptor}.",
        )
        .unwrap();
        let expected = "Hello, World. Hope you are having a nice morning.";
        assert_eq!(formatter.with_args(&Message).to_string(), expected);
    }

    #[test]
    fn compiled_failed_parsing() {
        let err = CompiledFormatter::<Message>::new(
            "Hello, {recipient}. Hope you are having a nice {time_descriptor.",
        )
        .unwrap_err();
        assert_eq!(err, FormatError::Parse("time_descriptor."));
    }

    #[test]
    fn compiled_unknown_key() {
        let err = CompiledFormatter::<Message>::new(
            "Hello, {recipient}. Hope you are having a nice {time_descriptr}.",
        )
        .unwrap_err();
        assert_eq!(err, FormatError::Key("time_descriptr"));
    }
}
