/// This object represents the state at the beginning of a line
/// within a Unicode string. The stored index points to the byte
/// index of the next start of a line or usize::MAX. The stored
/// string contains the complete string we index in.
/// via https://gist.github.com/tajpulo/a717e5f0dc2b36ebd8eae63786d1dd72
pub(crate) struct LinesWithByteIndices<'s>(&'s str, usize);

impl<'s> LinesWithByteIndices<'s> {
    pub(crate) fn from_str(s: &'s str) -> Self {
        Self(s, 0)
    }
}

/// Give me a substring and I will return the byte index of
/// the (next line terminator, next line start). If they don't
/// exist, we return usize::MAX in either case.
fn find_next_line_terminator(substr: &str) -> (usize, usize) {
    let mut iterator = substr.char_indices();

    if substr.is_empty() {
        return (usize::MAX, usize::MAX);
    }

    while let Some((byte_offset, chr)) = iterator.next() {
        match chr {
            '\u{000C}' | '\u{000B}' | '\u{2028}' | '\u{2029}' | // LB4
            '\u{000A}' | '\u{0085}' // LB5
            => {

                // these single characters already break the current line
                return (byte_offset, byte_offset + chr.len_utf8());
            },
            '\u{000D}' => {
                // we might have (U+000D) or (U+000D, U+000A) which are both terminators
                match iterator.next() {
                    Some((next_byte_offset, '\u{000A}')) => return (byte_offset, next_byte_offset + 1),
                    Some(_) => return (byte_offset, byte_offset + 1),
                    None => return (byte_offset, byte_offset + 1),
                }
            },
            _ => {},
        };
    }

    // we did not find a terminator, so claim the terminator and
    // the next line-start is past the end of the string
    (usize::MAX, usize::MAX)
}

impl<'s> Iterator for LinesWithByteIndices<'s> {
    type Item = (usize, &'s str); // (line start byte index, substring representing line)

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // We follow the “hard line breaks” definition of Unicode TR#14 here.
        // Specifically, we introduce a hard break after any of {000C, 000B, 2028, 2029}
        // because of rule LB4. Furthermore a hard break follows after any of
        // {000D, 000A, 0085} or the sequence 000A & 000D according to rule LB5.
        // https://www.unicode.org/reports/tr14/
        if self.1 == usize::MAX {
            return None;
        }

        let this_line_start = self.1; // index of first byte of this line

        // NOTE: self.0 must always be a valid byte offset to some Unicode scalar
        let substr = self.0.get(this_line_start..).unwrap();
        let (terminator_index, next_start_index) = find_next_line_terminator(substr);
        return if terminator_index == usize::MAX && next_start_index == usize::MAX {
            self.1 = usize::MAX;
            Some((this_line_start, substr))
        } else {
            self.1 = next_start_index.saturating_add(this_line_start);
            Some((this_line_start, self.0.get(this_line_start..this_line_start.saturating_add(terminator_index)).unwrap()))
        }
    }
}

pub(crate) trait StrLinesWithByteIndices {
    fn lines_indices<'s>(&'s self) -> LinesWithByteIndices<'s>;
}

impl<'s> StrLinesWithByteIndices for &'s str {
    fn lines_indices(&self) -> LinesWithByteIndices<'s> {
        LinesWithByteIndices::from_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_string() {
        let text = "Hello world!";
        let mut iter = text.lines_indices();
        assert_eq!(iter.next(), Some((0, "Hello world!")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn simple_newline_split() {
        let text = "Hello world\nfoo\nbar!";
        let mut iter = text.lines_indices();
        assert_eq!(iter.next(), Some((0, "Hello world")));
        assert_eq!(iter.next(), Some((12, "foo")));
        assert_eq!(iter.next(), Some((16, "bar!")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn simple_newline_split_with_trailing_line() {
        let text = "Hello world\nfoo\nbar!\n";
        let mut iter = text.lines_indices();
        assert_eq!(iter.next(), Some((0, "Hello world")));
        assert_eq!(iter.next(), Some((12, "foo")));
        assert_eq!(iter.next(), Some((16, "bar!")));
        assert_eq!(iter.next(), Some((21, "")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn split_with_various_line_terminators() {
        let text = "Knock\u{000D}\u{000A}knock\u{000A}…\u{000B}who's\u{000C}there\u{000D}?\u{2028}Knock\u{2029}Ness\u{0085}!";
        let mut iter = text.lines_indices();
        assert_eq!(iter.next(), Some((0, "Knock")));
        assert_eq!(iter.next(), Some((7, "knock")));
        assert_eq!(iter.next(), Some((13, "…")));
        assert_eq!(iter.next(), Some((17, "who's")));
        assert_eq!(iter.next(), Some((23, "there")));
        assert_eq!(iter.next(), Some((29, "?")));
        assert_eq!(iter.next(), Some((33, "Knock")));
        assert_eq!(iter.next(), Some((41, "Ness")));
        assert_eq!(iter.next(), Some((47, "!")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn many_empty_lines() {
        let text = "A\n\nB\n  \nC\r\n D \n";
        let mut iter = text.lines_indices();
        assert_eq!(iter.next(), Some((0, "A")));
        assert_eq!(iter.next(), Some((2, "")));
        assert_eq!(iter.next(), Some((3, "B")));
        assert_eq!(iter.next(), Some((5, "  ")));
        assert_eq!(iter.next(), Some((8, "C")));
        assert_eq!(iter.next(), Some((11, " D ")));
        assert_eq!(iter.next(), Some((15, "")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn invalid_terminator() {
        // the standardized sequence is (U+000D, U+000A), not the other way around
        let text = "Knock\u{000A}\u{000D}knock";
        let mut iter = text.lines_indices();
        assert_eq!(iter.next(), Some((0, "Knock")));
        assert_eq!(iter.next(), Some((6, "")));
        assert_eq!(iter.next(), Some((7, "knock")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn finish_with_carriage_return() {
        let text = "line 1\u{000A}line 2\u{000A}";
        let mut iter = text.lines_indices();
        assert_eq!(iter.next(), Some((0, "line 1")));
        assert_eq!(iter.next(), Some((7, "line 2")));
        assert_eq!(iter.next(), Some((14, "")));
        assert_eq!(iter.next(), None);
    }
}
