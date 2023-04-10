use std::error;
use std::fmt;
use std::path;

use crate::lexer;
use crate::lines_with_indices::StrLinesWithByteIndices;


#[derive(Debug,Clone)]
pub enum Error {
    /// lexing error regarding unbalanced parentheses with message and byte offset
    UnbalancedParentheses(String, usize),
    /// lexing error regarding syntax violation with message and byte offset
    InvalidSyntax(String, usize),
    /// parsing error where the lexer yields an invalid sequence of tokens
    /// with messages what we actual got and what we expected
    UnexpectedToken(lexer::Token, String),
    /// parsing error where the content suddenly finished
    UnexpectedEOF(String),
    /// lexing error which was resolved into a complete message
    /// including line number and line column. Consists of
    /// (filepath, message, line number, character index within line, byte offset within line).
    /// NOTE: must not be used directly by the lexer
    LexingError(path::PathBuf, String, usize, usize, usize),
    /// lexing error which was resolved into a complete message
    /// including line number and line column. Consists of (filepath, message, X)
    /// where X is (line number, character index within line, byte offset within line)
    /// twice for start and end.
    /// NOTE: must not be used directly by the lexer
    RangedLexingError(path::PathBuf, String, [(usize, usize, usize); 2]),
}

impl Error {
    /// Return (lineno, linecol, byte offset within line) for a given `byte_offset`
    /// within some text content `src`
    fn get_line_identifier_at_byte(byte_offset: usize, src: &str) -> (usize, usize, usize) {
        let mut prev_byte_offset = 0;
        let mut prev_line_index = 0;
        let mut prev_column_index = 0;

        for (line_index, (start_byte_offset, line)) in src.lines_indices().enumerate() {
            for (column_index, (column_byte_offset, _)) in line.char_indices().enumerate() {
                if prev_byte_offset <= byte_offset && byte_offset < start_byte_offset + column_byte_offset {
                    return (prev_line_index, prev_column_index, column_byte_offset);
                }

                prev_byte_offset = start_byte_offset + column_byte_offset;
                prev_line_index = line_index;
                prev_column_index = column_index;
            }
        }

        (prev_line_index, prev_column_index, prev_byte_offset)
    }

    pub fn format_with_source(&self, filepath: &path::Path, src: &str) -> Error {
        use Error::*;

        match self {
            UnbalancedParentheses(msg, byte_offset) |
            InvalidSyntax(msg, byte_offset) => {
                let (line_index, line_char_index, line_byte_index) = Self::get_line_identifier_at_byte(*byte_offset, src);
                let lineno = line_index + 1;  // humans prefer one-based indices, we get zero-based indices
                let linecol = line_char_index + 1;  // humans prefer one-based indices, we get zero-based indices

                LexingError(filepath.to_owned(), msg.to_owned(), lineno, linecol, line_byte_index)
            },
            UnexpectedEOF(msg) => {
                let lines_count = src.lines().count();
                LexingError(filepath.to_owned(), msg.to_owned(), lines_count, 0, src.len())
            },
            UnexpectedToken(got_token, expected) => {
                let byte_offsets = got_token.byte_offsets();
                let (start_index, start_char_index, start_byte_index) = Self::get_line_identifier_at_byte(byte_offsets.0, src);

                match byte_offsets.1 {
                    Some(end_byteoffset) => {
                        let (end_index, end_char_index, end_byte_index) = Self::get_line_identifier_at_byte(end_byteoffset, src);
                        RangedLexingError(
                            filepath.to_owned(),
                            format!("expected {}, but got token {:?}", expected, got_token.name()),
                            [(start_index, start_char_index, start_byte_index), (end_index, end_char_index, end_byte_index)]
                        )
                    },
                    None => {
                        let msg = format!("expected {}, but got token {:?}", expected, got_token.name());
                        LexingError(filepath.to_owned(), msg, start_index, start_char_index, start_byte_index)
                    },
                }


            },
            LexingError(..) => self.clone(),
            RangedLexingError(..) => self.clone(),
        }
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            UnbalancedParentheses(msg, byte) |
            InvalidSyntax(msg, byte) => write!(f, "{msg} at byte {byte}"),
            UnexpectedEOF(msg) => write!(f, "{msg}"),
            UnexpectedToken(got, expected) => write!(f, "expected {expected}, but got token {:?}", got),
            LexingError(filepath, message, line_index, column_index, column_byteoffset) =>
                write!(
                    f, "{message} in file {}, line {} at column {} (byte offset {} within line)",
                    filepath.display(), line_index + 1, column_index + 1, column_byteoffset
                ),
            RangedLexingError(filepath, message, range) =>
                write!(
                    f, "{message} in file {} from line {} at column {} until line {} at column {}",
                    filepath.display(), range[0].0 + 1, range[0].1 + 1, range[1].0, range[1].1
                ),
        }
    }
}


