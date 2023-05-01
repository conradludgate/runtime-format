use core::fmt;

use crate::{parse::ParseSegment, FormatArgs, FormatError, FormatKey, ToFormatParser};

/// Preparsed formatting terms.
///
/// This is faster if you will be using the same format string again and again with
/// different inputs.
///
/// ```
/// use runtime_format::{FormatArgs, FormatKey, FormatKeyError, ParsedFmt};
/// use core::fmt;
/// # struct DateTime;
/// # impl DateTime { fn now() -> Self { Self } }
/// # impl DateTime { fn day(&self) -> i32 { 25 } fn short_month_name(&self) -> &'static str { "Jan" } fn year(&self) -> i32 { 2023 } }
/// # impl DateTime { fn hours(&self) -> i32 { 16 } fn minutes(&self) -> i32 { 27 } fn seconds(&self) -> i32 { 53 } }
/// impl FormatKey for DateTime {
///     fn fmt(&self, key: &str, f: &mut fmt::Formatter<'_>) -> Result<(), FormatKeyError> {
///          // ...
/// #        use core::fmt::Write;
/// #        match key {
/// #            "year"    => write!(f, "{}", self.year()).map_err(FormatKeyError::Fmt),
/// #            "month"   => write!(f, "{}", self.short_month_name()).map_err(FormatKeyError::Fmt),
/// #            "day"     => write!(f, "{}", self.day()).map_err(FormatKeyError::Fmt),
/// #            "hours"   => write!(f, "{}", self.hours()).map_err(FormatKeyError::Fmt),
/// #            "minutes" => write!(f, "{}", self.minutes()).map_err(FormatKeyError::Fmt),
/// #            "seconds" => write!(f, "{}", self.seconds()).map_err(FormatKeyError::Fmt),
/// #            _ => Err(FormatKeyError::UnknownKey),
/// #        }
///     }
/// }
///
/// let now = DateTime::now();
/// let fmt = ParsedFmt::new("{month} {day} {year} {hours}:{minutes}:{seconds}").unwrap();
/// let args = FormatArgs::new(&fmt, &now);
/// let expected = "Jan 25 2023 16:27:53";
/// assert_eq!(args.to_string(), expected);
pub struct ParsedFmt<'a> {
    segments: tinyvec::TinyVec<[ParseSegment<'a>; 8]>,
}

impl<'a> ToFormatParser<'a> for ParsedFmt<'a> {
    type Parser = std::iter::Copied<std::slice::Iter<'a, ParseSegment<'a>>>;

    fn to_parser(&'a self) -> Self::Parser {
        self.segments.iter().copied()
    }

    fn unparsed(_: Self::Parser) -> &'a str {
        ""
    }
}

impl fmt::Debug for ParsedFmt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledFormatter")
            .field("segments", &self.segments)
            .finish()
    }
}

impl<'a> ParsedFmt<'a> {
    /// Parse the given format string.
    ///
    /// # Errors
    /// If the string could not be parsed, or there is a key that is unacceptable.
    pub fn new(s: &'a str) -> Result<Self, FormatError<'a>> {
        let mut segments = s.to_parser();
        let this = Self {
            segments: segments.by_ref().collect(),
        };

        if !segments.s.is_empty() {
            Err(FormatError::Parse(segments.s))
        } else {
            Ok(this)
        }
    }

    /// Return the keys that will be used when formatting.
    ///
    /// ```
    /// # use runtime_format::ParsedFmt;
    /// let fmt = "Hello, {recipient}. Hope you are having a nice {time_descriptor}.";
    /// let parsed = ParsedFmt::new(fmt).unwrap();
    /// let keys: Vec<_> = parsed.keys().collect();
    /// assert_eq!(keys, ["recipient", "time_descriptor"]);
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &'_ str> {
        self.segments.iter().filter_map(|segment| match segment {
            ParseSegment::Literal(_) => None,
            ParseSegment::Key(key) => Some(*key),
        })
    }

    /// Combine this parsed format with the given values into a [`FormatArgs`]
    pub fn with_args<'fs, 'fk, F: FormatKey>(
        &'fs self,
        fmt: &'fk F,
    ) -> FormatArgs<'fs, 'fk, Self, F> {
        FormatArgs::new(self, fmt)
    }
}

impl<'a> TryFrom<&'a str> for ParsedFmt<'a> {
    type Error = FormatError<'a>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'a> FromIterator<ParseSegment<'a>> for ParsedFmt<'a> {
    fn from_iter<T: IntoIterator<Item = ParseSegment<'a>>>(iter: T) -> Self {
        Self {
            segments: FromIterator::from_iter(iter),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use crate::{FormatError, FormatKey, FormatKeyError};

    use super::ParsedFmt;

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
    fn compiled_happy_path() {
        let formatter =
            ParsedFmt::new("Hello, {recipient}. Hope you are having a nice {time_descriptor}.")
                .unwrap();
        let expected = "Hello, World. Hope you are having a nice morning.";
        assert_eq!(formatter.with_args(&Message).to_string(), expected);
    }

    #[test]
    fn compiled_failed_parsing() {
        let err =
            ParsedFmt::new("Hello, {recipient}. Hope you are having a nice {time_descriptor.")
                .unwrap_err();
        assert_eq!(err, FormatError::Parse("time_descriptor."));
    }

    #[test]
    fn compiled_keys() {
        let parsed =
            ParsedFmt::new("Hello, {recipient}. Hope you are having a nice {time_descriptr}.")
                .unwrap();
        let keys: Vec<_> = parsed.keys().collect();
        assert_eq!(keys, ["recipient", "time_descriptr"]);
    }
}
