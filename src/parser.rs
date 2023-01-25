
#[derive(Debug)]
pub(crate) enum ParseSegment<'a> {
    Literal(&'a str),
    Key(&'a str),
}
impl Default for ParseSegment<'_> {
    fn default() -> Self {
        Self::Literal("")
    }
}

pub(crate) struct Parser<'a> {
    pub(crate) s: &'a str,
    pub(crate) is_key: bool,
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
