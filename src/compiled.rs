use core::fmt;

use crate::{parse::ParseSegment, FormatArgs, FormatError, FormatKey, ToFormatParser};

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

    pub fn keys(&self) -> impl Iterator<Item = &'_ str> {
        self.segments.iter().filter_map(|segment| match segment {
            ParseSegment::Literal(_) => None,
            ParseSegment::Key(key) => Some(*key),
        })
    }

    /// Combine this parsed format with the given values into a [`FormatArgs`]
    pub fn with_args<'b, F: FormatKey>(&'b self, fmt: &'b F) -> FormatArgs<'b, Self, F> {
        FormatArgs::new(self, fmt)
    }
}

impl<'a> TryFrom<&'a str> for ParsedFmt<'a> {
    type Error = FormatError<'a>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::new(value)
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
