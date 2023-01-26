//! Formatting, but processed at runtime.
//!
//! ```
//! use runtime_format::{FormatArgs, FormatKey, FormatKeyError};
//! use core::fmt;
//! # struct DateTime;
//! # impl DateTime { fn now() -> Self { Self } }
//! # impl DateTime { fn day(&self) -> i32 { 25 } fn short_month_name(&self) -> &'static str { "Jan" } fn year(&self) -> i32 { 2023 } }
//! # impl DateTime { fn hours(&self) -> i32 { 16 } fn minutes(&self) -> i32 { 27 } fn seconds(&self) -> i32 { 53 } }
//! impl FormatKey for DateTime {
//!     fn fmt(&self, key: &str, f: &mut fmt::Formatter<'_>) -> Result<(), FormatKeyError> {
//!         use core::fmt::Write;
//!         match key {
//!             "year"    => write!(f, "{}", self.year()).map_err(FormatKeyError::Fmt),
//!             "month"   => write!(f, "{}", self.short_month_name()).map_err(FormatKeyError::Fmt),
//!             "day"     => write!(f, "{}", self.day()).map_err(FormatKeyError::Fmt),
//!             "hours"   => write!(f, "{}", self.hours()).map_err(FormatKeyError::Fmt),
//!             "minutes" => write!(f, "{}", self.minutes()).map_err(FormatKeyError::Fmt),
//!             "seconds" => write!(f, "{}", self.seconds()).map_err(FormatKeyError::Fmt),
//!             _ => Err(FormatKeyError::UnknownKey),
//!         }
//!     }
//! }
//!
//! let now = DateTime::now();
//! let fmt = "{month} {day} {year} {hours}:{minutes}:{seconds}";
//! let args = FormatArgs::new(fmt, &now);
//! let expected = "Jan 25 2023 16:27:53";
//! assert_eq!(args.to_string(), expected);
//! ```
//!
//! See [`ParsedFmt`] if you need to repeatedly format a given string, but with
//! different args.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
mod alloc_impls;
#[cfg(feature = "std")]
pub use alloc_impls::*;

#[cfg(feature = "std")]
mod compiled;
#[cfg(feature = "std")]
pub use compiled::ParsedFmt;

mod parse;
pub use parse::{FromStr, ParseSegment};

use core::cell::Cell;
use core::fmt;

#[derive(Debug, Clone, PartialEq)]
/// Error produced when formatting
pub enum FormatKeyError {
    /// The formatter had an error
    Fmt(fmt::Error),
    /// The requested key is unknown
    UnknownKey,
}

#[cfg(feature = "std")]
impl std::error::Error for FormatKeyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FormatKeyError::Fmt(f) => Some(f),
            FormatKeyError::UnknownKey => None,
        }
    }
}

impl fmt::Display for FormatKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatKeyError::Fmt(_) => f.write_str("There was an error writing to the formatter"),
            FormatKeyError::UnknownKey => f.write_str("The requested key is unknown"),
        }
    }
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
/// Error returned when formatting or parsing.
pub enum FormatError<'a> {
    /// The key was invalid
    Key(&'a str),
    /// Could not parse the string
    Parse(&'a str),
}

#[cfg(feature = "std")]
impl std::error::Error for FormatError<'_> {}

impl fmt::Display for FormatError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Key(key) => write!(f, "The requested key {key:?} is unknown"),
            FormatError::Parse(rest) => write!(f, "Failed to parse {rest:?}"),
        }
    }
}

/// Performs formatting.
pub struct FormatArgs<'fs, 'fk, FS: ?Sized, FK: ?Sized> {
    format_segments: &'fs FS,
    format_keys: &'fk FK,
    error: Cell<Option<FormatError<'fs>>>,
}

impl<'fs, 'fk, FS: ?Sized, FK: ?Sized> FormatArgs<'fs, 'fk, FS, FK> {
    /// Create a new `FormatArgs` using the format specifier and the format keys
    pub fn new(format_specified: &'fs FS, format_keys: &'fk FK) -> Self {
        FormatArgs {
            format_segments: format_specified,
            format_keys,
            error: Cell::new(None),
        }
    }

    /// If there was an error when formatting, then that error is available here.
    pub fn status(&self) -> Result<(), FormatError<'fs>> {
        match self.error.take() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}

impl<'fs, 'fk, FS, FK> fmt::Display for FormatArgs<'fs, 'fk, FS, FK>
where
    FS: ?Sized + ToFormatParser<'fs>,
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

impl<'fs, 'fk, FS, FK> fmt::Debug for FormatArgs<'fs, 'fk, FS, FK>
where
    FS: ?Sized + ToFormatParser<'fs>,
    FK: ?Sized + FormatKey,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
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
