use core::{cell::Cell, fmt};

use crate::{
    parser::{ParseSegment, Parser},
    FormatError, FormatKey, FormatKeyError,
};

pub struct Format<'a, F> {
    segments: tinyvec::TinyVec<[ParseSegment<'a>; 8]>,
    _fmt: core::marker::PhantomData<&'a F>,
}

impl<F> fmt::Debug for Format<'_, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledFormatter")
            .field("segments", &self.segments)
            .finish()
    }
}

impl<'a, F: FormatKey> Format<'a, F> {
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

#[cfg(test)]
mod tests {
    use core::fmt;

    use crate::{FormatError, FormatKey, FormatKeyError};

    use super::Format;

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
    fn compiled_happy_path() {
        let formatter =
            Format::new("Hello, {recipient}. Hope you are having a nice {time_descriptor}.")
                .unwrap();
        let expected = "Hello, World. Hope you are having a nice morning.";
        assert_eq!(formatter.with_args(&Message).to_string(), expected);
    }

    #[test]
    fn compiled_failed_parsing() {
        let err = Format::<Message>::new(
            "Hello, {recipient}. Hope you are having a nice {time_descriptor.",
        )
        .unwrap_err();
        assert_eq!(err, FormatError::Parse("time_descriptor."));
    }

    #[test]
    fn compiled_unknown_key() {
        let err = Format::<Message>::new(
            "Hello, {recipient}. Hope you are having a nice {time_descriptr}.",
        )
        .unwrap_err();
        assert_eq!(err, FormatError::Key("time_descriptr"));
    }
}
