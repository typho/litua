//! Lexer for litua text documents

use std::collections::VecDeque;
use std::fmt;
use std::mem;
use std::ops;
use std::str;

use crate::errors;

// characters part of the litua text document syntax

/// U+007B  LEFT CURLY BRACKET
pub const OPEN_FUNCTION: char = '{';
/// U+007D  RIGHT CURLY BRACKET
pub const CLOSE_FUNCTION: char = '}';
/// U+005B  LEFT SQUARE BRACKET
pub const OPEN_ARG: char = '[';
/// U+005D  RIGHT SQUARE BRACKET
pub const CLOSE_ARG: char = ']';
/// U+003D  EQUALS SIGN
pub const ASSIGN: char = '=';
/// U+003C  LESS-THAN SIGN
pub const OPEN_RAW: char = '<';
/// U+003E  GREATER-THAN SIGN
pub const CLOSE_RAW: char = '>';

/// `Lexer` is an object holding a reference to the source code
/// of the text document to lex. Method `iter()` returns an
/// `LexingIterator` which allows to iterate over the tokens of
/// the lexed document.
#[derive(Clone,Debug,PartialEq)]
pub struct Lexer<'l> {
    /// reference to source code
    pub source: &'l str,
}

impl<'l> Lexer<'l> {
    pub fn new(src: &'l str) -> Self {
        Self { source: src }
    }

    pub fn iter(&'l self) -> LexingIterator {
        LexingIterator::new(self.source)
    }
}

#[derive(Clone,Debug,Hash,PartialEq)]
enum LexingScope {
    ContentInFunction,
    ArgumentValueInFunction,
    FunctionInContent,
    FunctionInArgumentValue,
}

/// The various states the lexer can be in during the
/// lexing phase. Reading prefixes mean “I just read the
/// first or more characters” whereas Found prefixes mean
/// “I just read the first character”. For details, please
/// refer to the state diagrams in the `design/` folder.
#[derive(Clone,Debug,PartialEq)]
pub enum LexingState {
    ReadingContent,
    ReadingContentText,
    ReadingArgumentValue,
    ReadingArgumentValueText,
    FoundCallOpening,
    StartRaw,
    ReadingRaw,
    EndRaw,
    ReadingCallName,
    FoundArgumentOpening,
    FoundArgumentClosing,
    Terminated,
}

impl fmt::Display for LexingState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexingState::ReadingContent => write!(f, "reading content"),
            LexingState::ReadingContentText => write!(f, "reading text inside content"),
            LexingState::ReadingArgumentValue => write!(f, "reading an argument value"),
            LexingState::ReadingArgumentValueText => write!(f, "reading text inside an argument value"),
            LexingState::FoundCallOpening => write!(f, "reading the start of a function call"),
            LexingState::StartRaw => write!(f, "starting a raw text"),
            LexingState::ReadingRaw => write!(f, "reading raw text"),
            LexingState::EndRaw => write!(f, "terminating raw text"),
            LexingState::ReadingCallName => write!(f, "reading the name of a function call"),
            LexingState::FoundArgumentOpening => write!(f, "reading a function argument"),
            LexingState::FoundArgumentClosing => write!(f, "finishing one function argument"),
            LexingState::Terminated => write!(f, "terminating"),
        }
    }
}

/// `LexingIteratior` is the object you receive when calling `.iter()` on the `Lexer` object.
#[derive(Debug)]
pub struct LexingIterator<'l> {
    /// state of this iterator
    pub state: LexingState,
    /// byte offset where the current token started
    token_start: usize,
    /// byte offset where the second-to-most-current token started.
    /// there are two scenarios where one 'token_start' does not suffice.
    /// (1) raw strings have a start of the '{<' and a start of the text.
    /// (2) any content string needs to store where it started and where
    ///     it currently is because EOF needs to be reported with the final position.
    /// Thus we introduce `token_wrapping_start` which stores the start
    /// position of the token wrapping the token referred to by `token_start`.
    token_wrapping_start: usize,
    /// raw-text ends with a repetition of “>” where the number matches
    /// the number of “<” of the beginning. Thus we store the number of
    /// characters here.
    raw_delimiter_length: u8,
    /// While parsing we discover a certain length and we will compare it
    /// with “raw_delimiter_length”
    raw_delimiter_read: u8,
    /// iterator over (UTF-8 byte offset, Unicode scalar)
    chars: str::CharIndices<'l>,
    /// `stack` stores the hierarchical level, we are in.
    /// Storing it is necessary, because the lexing rules are
    /// different after an argument value and a content value.
    /// Thus, we introduce the notion of “scopes” and store the
    /// level on a stack.
    stack: Vec<LexingScope>,
    /// `next_tokens` stores the next tokens to emit. The return value of
    /// `progress()` is one token, but sometimes several tokens are generated.
    /// In this case, the tokens are `push_back`ed and consecutively
    /// `pop_front`ed to process them.
    pub next_tokens: VecDeque<Token>,
    /// if an error occured, the error is returned once
    /// and the lexer switches to the infinite EOF state
    pub occured_error: Option<errors::Error>,
}

impl<'l> LexingIterator<'l> {
    /// Create a `LexingIterator` instance based on the source code `src`
    /// of the text document provided.
    pub fn new(src: &str) -> LexingIterator {
        LexingIterator {
            state: LexingState::ReadingContent,
            token_start: 0,
            token_wrapping_start: 0,
            raw_delimiter_length: 0,
            raw_delimiter_read: 0,
            chars: src.char_indices(),
            stack: Vec::new(),
            next_tokens: VecDeque::new(),
            occured_error: None,
        }
    }

    fn push_scope(&mut self, sc: LexingScope, byte_offset: usize) {
        self.token_start = byte_offset;
        self.stack.push(sc);
    }

    fn pop_scope(&mut self, byte_offset: usize) -> LexingScope {
        use LexingScope::*;

        let top = match self.stack.pop() {
            Some(t) => t,
            None => {
                self.state = LexingState::Terminated;
                self.occured_error = Some(errors::Error::UnbalancedParentheses(format!("there is some function end too many - function ended at {} but never started", self.token_start)));
                return LexingScope::ContentInFunction; // NOTE: arbitrary token
            }
        };

        match top {
            ArgumentValueInFunction => {
                self.state = LexingState::FoundArgumentClosing;
            },
            FunctionInContent => {
                self.state = LexingState::ReadingContent;
            },
            FunctionInArgumentValue => {
                self.state = LexingState::ReadingArgumentValue;
            },
            ContentInFunction => {
                self.next_tokens.push_back(Token::EndFunction(byte_offset));
                self.pop_scope(byte_offset);
            },
        };

        self.token_start = byte_offset;
        top
    }

    /// Continue reading the next Unicode scalar.
    /// Maybe the result is some (start_of_token, Ok(Token)) to emit
    /// or maybe the result is None, since the token consists of multiple scalars.
    pub(crate) fn progress(&mut self) -> Option<Token> {
        use LexingState::*;

        // emit pre-registered tokens from previous iteration
        let front = self.next_tokens.pop_front();
        if let Some(tok) = front {
            return Some(tok);
        }

        if self.state == Terminated {
            return None;
        }

        // read the next Unicode scalar
        let (byte_offset, chr) = match self.chars.next() {
            Some((bo, ch)) => (bo, ch),
            None => {
                if self.token_start != self.token_wrapping_start {
                    self.next_tokens.push_back(Token::Text(self.token_start..self.token_wrapping_start + 1));
                    self.token_start = self.token_wrapping_start;
                    return None;
                }
                self.state = Terminated;
                return Some(Token::EndOfFile(self.token_wrapping_start));
            },
        };

        // dbg!(&self.state);
        match self.state {
            ReadingContent => {
                match chr {
                    OPEN_FUNCTION => {
                        self.push_scope(LexingScope::FunctionInContent, byte_offset);
                        self.state = FoundCallOpening;
                    },
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::EndContent(byte_offset));
                        assert_eq!(self.pop_scope(byte_offset), LexingScope::ContentInFunction);
                    },
                    _ => {
                        self.state = ReadingContentText;
                        self.token_start = byte_offset;
                        self.token_wrapping_start = byte_offset;
                    },
                }
            },
            ReadingContentText => {
                match chr {
                    OPEN_FUNCTION => {
                        self.next_tokens.push_back(Token::Text(self.token_start..byte_offset));
                        self.push_scope(LexingScope::FunctionInContent, byte_offset);
                        self.state = FoundCallOpening;
                    },
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::Text(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::EndContent(byte_offset));
                        assert_eq!(self.pop_scope(byte_offset), LexingScope::ContentInFunction);
                    },
                    _ => {
                        self.token_wrapping_start = byte_offset;
                    },
                }
            },
            ReadingArgumentValue => {
                // NOTE: Technically, it would be more beautiful to introduce a separate
                //       state for the first character. Practically we only need to store
                //       the initial byte offset. Thus we set it to MAX before and to the
                //       current token upon the first iteration.
                if self.token_start == usize::MAX {
                    self.next_tokens.push_back(Token::BeginArgValue(byte_offset));
                }

                match chr {
                    OPEN_FUNCTION => {
                        self.push_scope(LexingScope::FunctionInArgumentValue, byte_offset);
                        self.state = FoundCallOpening;
                    },
                    CLOSE_ARG => {
                        self.next_tokens.push_back(Token::EndArgValue(byte_offset));
                        assert_eq!(self.pop_scope(byte_offset), LexingScope::ArgumentValueInFunction);
                    },
                    _ => {
                        self.state = ReadingArgumentValueText;
                        self.token_start = byte_offset;
                    },
                }
            },
            ReadingArgumentValueText => {
                match chr {
                    OPEN_FUNCTION => {
                        self.next_tokens.push_back(Token::Text(self.token_start..byte_offset));
                        self.push_scope(LexingScope::FunctionInArgumentValue, byte_offset);
                        self.state = FoundCallOpening;
                    },
                    CLOSE_ARG => {
                        self.next_tokens.push_back(Token::Text(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::EndArgValue(byte_offset));
                        assert_eq!(self.pop_scope(byte_offset), LexingScope::ArgumentValueInFunction);
                    },
                    _ => {},
                }
            },
            FoundCallOpening => {
                match chr {
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::BeginFunction(byte_offset));
                        self.state = Terminated;
                        self.occured_error = Some(errors::Error::SyntaxError(format!("call '{}' was immediately closed by '{}', but empty calls are not allowed", OPEN_FUNCTION, CLOSE_FUNCTION)));
                    },
                    OPEN_RAW => {
                        self.state = StartRaw;
                        self.token_start = byte_offset;
                        self.raw_delimiter_length = 1;
                    },
                    _ => {
                        self.next_tokens.push_back(Token::BeginFunction(byte_offset));
                        self.state = ReadingCallName;
                        self.token_start = byte_offset;
                    }
                }
            },
            StartRaw => {
                match chr {
                    OPEN_RAW => {
                        self.raw_delimiter_length += 1;
                        if self.raw_delimiter_length == 127 {
                            self.state = Terminated;
                            self.occured_error = Some(errors::Error::SyntaxError(format!("raw string delimiter must not exceed length 128")));
                        }
                    },
                    c if c.is_whitespace() => {
                        self.state = ReadingRaw;
                        self.raw_delimiter_read = 0;
                        self.next_tokens.push_back(Token::BeginRaw(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::Whitespace(byte_offset, c));
                        self.token_wrapping_start = usize::MAX;
                    },
                    c => {
                        self.state = Terminated;
                        self.occured_error = Some(errors::Error::SyntaxError(format!("unexpected character '{}' while reading raw string start", c)));
                    }
                }
            },
            ReadingRaw => {
                match chr {
                    CLOSE_RAW => {
                        self.raw_delimiter_read += 1;
                        if self.raw_delimiter_read == 1 {
                            self.token_start = byte_offset;
                        }
                        if self.raw_delimiter_read == self.raw_delimiter_length {
                            self.state = EndRaw;
                        }
                    },
                    _ => {
                        if self.token_wrapping_start == usize::MAX {
                            self.token_wrapping_start = byte_offset;
                        }
                    }
                }
            },
            EndRaw => {
                match chr {
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::Text(self.token_wrapping_start..self.token_start));
                        self.next_tokens.push_back(Token::EndRaw(self.token_start..byte_offset));
                        self.pop_scope(byte_offset);
                    },
                    _ => {
                        self.state = ReadingRaw;
                        self.raw_delimiter_read = 0;
                        self.token_wrapping_start = byte_offset;
                    }
                }
            },
            ReadingCallName => {
                match chr {
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::Call(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::EndFunction(byte_offset));
                        self.pop_scope(byte_offset);
                    },
                    c if c.is_whitespace() => {
                        self.next_tokens.push_back(Token::Call(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::Whitespace(byte_offset, c));
                        self.next_tokens.push_back(Token::BeginContent(byte_offset));
                        self.push_scope(LexingScope::ContentInFunction, byte_offset);
                        self.state = ReadingContent;
                    },
                    OPEN_ARG => {
                        self.next_tokens.push_back(Token::Call(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::BeginArgs);
                        self.token_start = usize::MAX; // invalidate value
                        self.state = FoundArgumentOpening;
                    },
                    _ => {},
                }
            },
            FoundArgumentOpening => {
                if self.token_start == usize::MAX {
                    self.token_start = byte_offset;
                }

                if chr == ASSIGN {
                    self.next_tokens.push_back(Token::ArgKey(self.token_start..byte_offset));
                    self.push_scope(LexingScope::ArgumentValueInFunction, byte_offset);
                    self.token_start = usize::MAX; // invalidate value
                    self.state = ReadingArgumentValue;
                }
            },
            FoundArgumentClosing => {
                match chr {
                    OPEN_ARG => {
                        self.state = FoundArgumentOpening;
                        self.token_start = usize::MAX;
                    },
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::EndArgs);
                        self.pop_scope(byte_offset);
                        self.next_tokens.push_back(Token::EndFunction(byte_offset));
                    },
                    c if c.is_whitespace() => {
                        self.next_tokens.push_back(Token::EndArgs);
                        self.next_tokens.push_back(Token::Whitespace(byte_offset, c));
                        self.next_tokens.push_back(Token::BeginContent(byte_offset));
                        self.push_scope(LexingScope::ContentInFunction, byte_offset);
                        self.state = ReadingContent;
                    },
                    _ => {
                        self.state = Terminated;
                        self.occured_error = Some(errors::Error::SyntaxError(format!("after ending arguments with '{}', I require a whitespace character to continue with content", CLOSE_ARG)));
                    }
                }
            },
            Terminated => {},
        }

        self.next_tokens.pop_front()
    }

    pub(crate) fn emit_occured_error(&mut self) -> Option<errors::Error> {
        mem::take(&mut self.occured_error)
    }
}

/// Tokens as interface between lexer and parser. The arguments of some
/// variant refer to a byte position within the source document where
/// this token happens (1-ary) or goes from-to (`ops::Range` instances).
/// `Whitespace` is an exception since it provides the whitespace character
/// directly.
/// 
/// The admissible sequences of `Token`s is not specified here. It is an
/// implicit contract between lexer and parser.
#[derive(Clone,Debug,PartialEq)]
pub enum Token {
    BeginFunction(usize),
    Call(ops::Range<usize>),
    Whitespace(usize, char),
    BeginArgs,
    ArgKey(ops::Range<usize>),
    BeginArgValue(usize),
    EndArgValue(usize),
    EndArgs,
    BeginContent(usize),
    EndContent(usize),
    EndFunction(usize),
    BeginRaw(ops::Range<usize>),
    EndRaw(ops::Range<usize>),
    Text(ops::Range<usize>),
    EndOfFile(usize),
}

impl Eq for Token {}

impl<'l> Iterator for LexingIterator<'l> {
    /// An item identifies when this token started (UTF-8 byte offset)
    /// and whether we get an error here (Err) or some token (Ok).
    type Item = Result<Token, errors::Error>;

    /// An iterator over tokens emitted by the lexer.
    /// It implements the rust's Iterator protocol, but additionally guarantees
    /// that a result value None will never be followed by a non-None result value.
    /// 
    /// Specifically the sequence of emitted tokens follows one of the following scenarios:
    /// 
    /// **Scenario 1** (success):
    /// 
    /// 1. An arbitrary sequence of ``Some(Ok(Token))`` elements where ``Token`` is not ``Token::EOF``
    /// 2. One value ``Some(Ok(Token::EOF))``
    /// 2. An infinite sequence of ``None`` elements
    /// 
    /// **Scenario 2** (failure):
    /// 
    /// 1. An arbitrary sequence of ``Some(Ok(Token))`` elements where ``Token`` is not ``Token::EOF``
    /// 2. Potentially one element ``Some(Ok(Token::EOF))``
    /// 3. One value ``Some(Err(errmsg))``
    /// 4. An infinite sequence of ``None`` elements
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.progress() {
                Some(Token::EndOfFile(pos)) => return Some(Ok(Token::EndOfFile(pos))),
                Some(token) => return Some(Ok(token)),
                None if self.state != LexingState::Terminated => continue,
                None => {
                    if let Some(error) = self.emit_occured_error() {
                        return Some(Err(error));
                    }

                    return None;
                },
            }
        }
    }
}
