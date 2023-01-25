//!
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
mod alloc_impls;
#[cfg(feature = "std")]
pub use alloc_impls::*;
use parse::ParseSegment;

#[cfg(feature = "std")]
pub mod compiled;

pub mod parse;

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

/// A trait like [`fmt::Display`] or [`fmt::Debug`] by with a keyed field.
///
/// It has a `fmt` method that accepts a [`fmt::Formatter`] argument. The important feature is the
/// `key` field which indicates what value should be written to the formatter.
///
/// ```
/// use runtime_format::{FormatArgs, FormatKey, FormatKeyError};
/// use core::fmt;
/// # struct DateTime;
/// # impl DateTime { fn now() -> Self { Self } }
/// # impl DateTime { fn day(&self) -> i32 { 25 } fn short_month_name(&self) -> &'static str { "Jan" } fn year(&self) -> i32 { 2023 } }
/// # impl DateTime { fn hours(&self) -> i32 { 16 } fn minutes(&self) -> i32 { 27 } fn seconds(&self) -> i32 { 53 } }
/// impl FormatKey for DateTime {
///     fn fmt(&self, key: &str, f: &mut fmt::Formatter<'_>) -> Result<(), FormatKeyError> {
///         use core::fmt::Write;
///         match key {
///             "year"    => write!(f, "{}", self.year()).map_err(FormatKeyError::Fmt),
///             "month"   => write!(f, "{}", self.short_month_name()).map_err(FormatKeyError::Fmt),
///             "day"     => write!(f, "{}", self.day()).map_err(FormatKeyError::Fmt),
///             "hours"   => write!(f, "{}", self.hours()).map_err(FormatKeyError::Fmt),
///             "minutes" => write!(f, "{}", self.minutes()).map_err(FormatKeyError::Fmt),
///             "seconds" => write!(f, "{}", self.seconds()).map_err(FormatKeyError::Fmt),
///             _ => Err(FormatKeyError::UnknownKey),
///         }
///     }
/// }
///
/// let now = DateTime::now();
/// let fmt = "{month} {day} {year} {hours}:{minutes}:{seconds}";
/// let args = FormatArgs::new(fmt, &now);
/// let expected = "Jan 25 2023 16:27:53";
/// assert_eq!(args.to_string(), expected);
/// ```
pub trait FormatKey {
    /// Write the value with the associated with the given `key` to the formatter.
    ///
    /// # Errors
    /// If the formatter returns an error, or if the key is unknown.
    fn fmt(&self, key: &str, f: &mut fmt::Formatter<'_>) -> Result<(), FormatKeyError>;

    // /// Returns false if the key is known at compile time to not be accepted.
    // ///
    // /// If the key might be accepted at runtime, this will return true.
    // fn is_acceptable_key(key: &str) -> bool {
    //     let _key = key;
    //     true
    // }
}

/// Turn a value into parsed formatting segments on the fly.
pub trait ToFormatParser<'a> {
    /// The Parser type that returns the [`ParseSegment`]s
    type Parser: Iterator<Item = ParseSegment<'a>>;

    /// Turn this value into the parser
    fn to_parser(&'a self) -> Self::Parser;
    /// Get the unparsed str from this parser.
    /// Used to determine if there was an error while parsing.
    fn unparsed(iter: Self::Parser) -> &'a str;
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FormatError<'a> {
    Key(&'a str),
    Parse(&'a str),
}

pub struct FormatArgs<'a, FS: ?Sized, FK: ?Sized> {
    format_segments: &'a FS,
    format_keys: &'a FK,
    error: Cell<Option<FormatError<'a>>>,
}

impl<'a, FS: ?Sized, FK: ?Sized> FormatArgs<'a, FS, FK> {
    pub fn new(format_segments: &'a FS, format_keys: &'a FK) -> Self {
        FormatArgs {
            format_segments,
            format_keys,
            error: Cell::new(None),
        }
    }

    pub fn status(&self) -> Result<(), FormatError<'a>> {
        match self.error.take() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}

impl<'a, FS, FK> fmt::Display for FormatArgs<'a, FS, FK>
where
    FS: ?Sized + ToFormatParser<'a>,
    FK: ?Sized + FormatKey,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut segments = self.format_segments.to_parser();
        for segment in &mut segments {
            match segment {
                ParseSegment::Literal(s) => f.write_str(s)?,
                ParseSegment::Key(key) => match self.format_keys.fmt(key, f) {
                    Ok(_) => {}
                    Err(FormatKeyError::Fmt(e)) => return Err(e),
                    Err(FormatKeyError::UnknownKey) => {
                        self.error.set(Some(FormatError::Key(key)));
                        return Err(fmt::Error);
                    }
                },
            }
        }
        let remaining = FS::unparsed(segments);
        if !remaining.is_empty() {
            self.error.set(Some(FormatError::Parse(remaining)));
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::{self, Write};

    use crate::{FormatArgs, FormatError, FormatKey, FormatKeyError};

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
        let fmt = FormatArgs::new(s, fmt);
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
    }

    #[test]
    fn happy_path() {
        let format_str = "Hello, {recipient}. Hope you are having a nice {time_descriptor}.";
        let expected = "Hello, World. Hope you are having a nice morning.";
        format(format_str, &Message, |output| {
            assert_eq!(output, expected.as_bytes())
        })
        .unwrap();
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
        })
        .unwrap();
    }
}
