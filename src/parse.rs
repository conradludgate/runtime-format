use crate::ToFormatParser;

#[derive(Debug, Clone, Copy)]
/// An enum representing the parsed portion of a format string
#[non_exhaustive]
pub enum ParseSegment<'a> {
    /// A string literal to be included as is
    Literal(&'a str),
    /// A keyed value, that should be looked up using [`FormatKey`](crate::FormatKey)
    Key(&'a str),
}

impl Default for ParseSegment<'_> {
    fn default() -> Self {
        Self::Literal("")
    }
}

/// An [`Iterator`] of [`ParseSegment`]s. Returned by of [`str::to_parser`](ToFormatParser).
pub struct FromStr<'a> {
    pub(crate) s: &'a str,
    pub(crate) is_key: bool,
}

impl<'a> Iterator for FromStr<'a> {
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

impl<'a> ToFormatParser<'a> for str {
    type Parser = FromStr<'a>;

    fn to_parser(&'a self) -> Self::Parser {
        FromStr {
            s: self,
            is_key: false,
        }
    }

    fn unparsed(iter: Self::Parser) -> &'a str {
        iter.s
    }
}
