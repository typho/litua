use std::error;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    UnbalancedParentheses(String),
    InvalidSyntax(String),
    UnexpectedToken(String, String),
    UnexpectedEOF(String),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            UnbalancedParentheses(msg) |
            UnexpectedEOF(msg) |
            InvalidSyntax(msg) => write!(f, "{msg}"),
            UnexpectedToken(got, expected) => write!(f, "expected {expected}, but got {got}"),
        }
    }
}
