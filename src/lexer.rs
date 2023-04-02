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

// TODO: with each LexingScope, we should store the start of the function.
//       This enables better tracking which function we are currently in
#[derive(Clone,Debug,Hash,PartialEq)]
enum LexingScope {
    ArgumentValue,
    Content,
    Function,
    RawString,
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
    /// State of this iterator
    pub state: LexingState,
    /// Number of bytes to be read by this lexer.
    /// Only used to handle EOF properly.
    source_byte_length: usize,
    /// Byte offset where the current token started.
    /// e.g. while lexing 'f' in ``{left}helloX``, `token_start` points to '{'.
    /// e.g. while lexing 'X' in ``{left}helloX``, `token_start` points to 'h'.
    /// e.g. while lexing 'X' in ``{item helloX``, `token_start` points to 'h'.
    /// e.g. while lexing 'X' in ``{<<< helloX``, `token_start` points to 'h'.
    token_start: usize,
    /// Byte offset where the last function started [the user usually wants to know which function is affected].
    /// e.g. while lexing 'f' in ``{left}helloX``, `token_function_start` points to '{'.
    /// e.g. while lexing 'X' in ``{left}helloX``, `token_function_start` points to usize::MAX.
    /// e.g. while lexing 'X' in ``{item helloX``, `token_function_start` points to '{'.
    /// e.g. while lexing 'X' in ``{<<< helloX``, `token_function_start` points to '{' [even though its a rawstring].
    token_function_start: usize,
    /// Byte offset where the raw string content starts.
    /// e.g. while lexing 'X' in ``{<<< helloX``, `token_rawcontent_start` points to 'h'.
    token_rawcontent_start: usize,
    /// raw strings end with a repetition of “>” where the number matches
    /// the number of “<” of the beginning. Thus we store the number of
    /// characters here.
    raw_delimiter_length: u8,
    /// While parsing raw string content we discover '>' and count this number
    /// of '>' until we reach “raw_delimiter_length”
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

// NOTE: we could replace those “== usize::MAX” checks and the raw_delimiter_read
//       hack with one integer “start_new_token_at_next_byte_offset: bool”.
//       Cleaner code, more memory required.
impl<'l> LexingIterator<'l> {
    /// Create a `LexingIterator` instance based on the source code `src`
    /// of the text document provided.
    pub fn new(src: &str) -> LexingIterator {
        LexingIterator {
            state: LexingState::ReadingContent,
            source_byte_length: src.len(),
            token_start: 0,
            token_function_start: 0,
            token_rawcontent_start: 0,
            raw_delimiter_length: 0,
            raw_delimiter_read: 0,
            chars: src.char_indices(),
            stack: vec![LexingScope::Content],
            next_tokens: VecDeque::new(),
            occured_error: None,
        }
    }

    fn push_scope(&mut self, sc: LexingScope, byte_offset: usize) {
        self.token_start = byte_offset;
        self.stack.push(sc);
    }

    fn pop_scope(&mut self, byte_offset: usize) {
        use LexingScope::*;

        let old_top = match self.stack.pop() {
            Some(t) => t,
            None => {
                self.state = LexingState::Terminated;
                self.occured_error = Some(errors::Error::UnbalancedParentheses(format!("some scope ended at byte {} but it never started", byte_offset)));
                return;
            }
        };

        let new_top = match self.stack.last() {
            Some(t) => t,
            None => {
                self.state = LexingState::Terminated;
                self.occured_error = Some(errors::Error::UnbalancedParentheses(format!("scope {:?} ended at byte {} but it never started", old_top, byte_offset)));
                return;
            }
        };

        match (&old_top, new_top) {
            (ArgumentValue, Function) => {
                self.state = LexingState::FoundArgumentClosing;
            },
            (Content, Function) => {
                self.next_tokens.push_back(Token::EndFunction(byte_offset));
                self.pop_scope(byte_offset);
            },
            (Function, ArgumentValue) => {
                self.state = LexingState::ReadingArgumentValue;
            },
            (Function, Content) => {
                self.state = LexingState::ReadingContent;
            },
            (RawString, Content) => {
                self.state = LexingState::ReadingContent;
                // NOTE: I am abusing “raw_delimiter_read” here.
                //       I need to distinguish whether we come from RawString (1) or just start new Content (0).
                self.raw_delimiter_read = 1;
            },
            (RawString, ArgumentValue) => {
                self.state = LexingState::ReadingArgumentValueText;
                // NOTE: I am abusing “raw_delimiter_read” here.
                //       I need to distinguish whether we come from RawString (1) or just start new Content (0).
                self.raw_delimiter_read = 1;
            },
            (_, _) => {
                // NOTE: only certain scopes can be stacked one-onto-another.
                //       the given state indicates a programming error and thus we panic.
                panic!("internal error: lexing scope state invalid: stack […, {:?}, {:?}]", &new_top, &old_top)
            },
        };

        self.token_start = usize::MAX;
    }

    /// Continue reading the next Unicode scalar.
    /// Maybe the result is some (start_of_token, Ok(Token)) to emit
    /// or maybe the result is None, since the token consists of multiple scalars.
    pub(crate) fn progress(&mut self) -> Option<Token> {
        use LexingState::*;

        // emit pre-registered tokens from previous iteration
        if let Some(tok) = self.next_tokens.pop_front() {
            return Some(tok);
        }

        if self.state == Terminated {
            return None;
        }

        // read the next Unicode scalar
        let (byte_offset, chr) = match self.chars.next() {
            Some((bo, ch)) => (bo, ch),
            None => {
                if self.token_start != self.source_byte_length {
                    self.next_tokens.push_back(Token::Text(self.token_start..self.source_byte_length));
                    self.token_start = self.source_byte_length;
                    return None;
                }
                self.state = Terminated;
                return Some(Token::EndOfFile(self.source_byte_length));
            },
        };
        //eprintln!("state {:?} raw_delimiter_read = {} and now char '{}'", self.state, self.raw_delimiter_read, chr);

        match self.state {
            ReadingContent => {
                if self.token_start == usize::MAX {
                    // NOTE: I am abusing “raw_delimiter_read” here.
                    //       I need to distinguish whether we came from RawString (1) or just start new Content (0).
                    if self.raw_delimiter_read != 1 {
                        self.next_tokens.push_back(Token::BeginContent(byte_offset));
                    }
                    self.raw_delimiter_read = 0;
                    self.token_start = byte_offset;
                }

                match chr {
                    OPEN_FUNCTION => {
                        self.token_start = byte_offset;
                        self.token_function_start = byte_offset;
                        self.state = FoundCallOpening;
                    },
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::EndContent(byte_offset));
                        self.token_start = byte_offset;
                        self.token_function_start = usize::MAX;
                        self.pop_scope(byte_offset);
                    },
                    _ => {
                        self.state = ReadingContentText;
                    },
                }
            },
            ReadingContentText => {
                match chr {
                    OPEN_FUNCTION => {
                        self.next_tokens.push_back(Token::Text(self.token_start..byte_offset));
                        self.token_start = byte_offset;
                        self.token_function_start = byte_offset;
                        self.state = FoundCallOpening;
                    },
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::Text(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::EndContent(byte_offset));
                        self.token_start = byte_offset;
                        self.token_function_start = usize::MAX;
                        self.pop_scope(byte_offset);
                    },
                    _ => {},
                }
            },
            ReadingArgumentValue => {
                if self.token_start == usize::MAX {
                    self.next_tokens.push_back(Token::BeginArgValue(byte_offset));
                    self.token_start = byte_offset;
                }

                match chr {
                    OPEN_FUNCTION => {
                        self.token_start = byte_offset;
                        self.token_function_start = byte_offset;
                        self.state = FoundCallOpening;
                    },
                    CLOSE_ARG => {
                        self.next_tokens.push_back(Token::EndArgValue(byte_offset));
                        self.token_start = byte_offset;
                        self.pop_scope(byte_offset);
                    },
                    _ => {
                        self.state = ReadingArgumentValueText;
                    },
                }
            },
            ReadingArgumentValueText => {
                // NOTE: I am abusing “raw_delimiter_read” here. Resetting value.
                self.raw_delimiter_read = 0;

                match chr {
                    OPEN_FUNCTION => {
                        if self.token_start != usize::MAX && self.token_start != byte_offset {
                            self.next_tokens.push_back(Token::Text(self.token_start..byte_offset));
                        }
                        self.token_start = byte_offset;
                        self.token_function_start = byte_offset;
                        self.state = FoundCallOpening;
                    },
                    CLOSE_ARG => {
                        if self.token_start != usize::MAX && self.token_start != byte_offset {
                            self.next_tokens.push_back(Token::Text(self.token_start..byte_offset));
                        }
                        self.next_tokens.push_back(Token::EndArgValue(byte_offset));
                        self.token_start = byte_offset;
                        self.pop_scope(byte_offset);
                    },
                    _ => {
                        if self.token_start == usize::MAX {
                            self.token_start = byte_offset;
                        }
                    },
                }
            },
            FoundCallOpening => {
                // NOTE: it is a little bit awkward that “{{item}” is a legal call of “{item”
                match chr {
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::BeginFunction(self.token_start));
                        let msg = format!("call '{OPEN_FUNCTION}' was immediately closed by '{CLOSE_FUNCTION}', but empty calls are not allowed");
                        self.occured_error = Some(errors::Error::InvalidSyntax(msg));
                        self.state = Terminated;
                    },
                    OPEN_RAW => {
                        self.token_start = byte_offset;
                        self.raw_delimiter_length = 1;
                        self.state = StartRaw;
                    },
                    _ => {
                        self.push_scope(LexingScope::Function, self.token_start);
                        self.next_tokens.push_back(Token::BeginFunction(self.token_start));
                        self.token_start = byte_offset;
                        self.state = ReadingCallName;
                    },
                }
            },
            StartRaw => {
                match chr {
                    OPEN_RAW => {
                        self.raw_delimiter_length += 1;
                        if self.raw_delimiter_length == 127 {
                            self.occured_error = Some(errors::Error::InvalidSyntax("raw string delimiter must not exceed length 126".to_string()));
                            self.state = Terminated;
                        }
                    },
                    c if c.is_whitespace() => {
                        self.raw_delimiter_read = 0;
                        self.next_tokens.push_back(Token::BeginRaw(self.token_function_start + OPEN_FUNCTION.len_utf8()..byte_offset));
                        self.next_tokens.push_back(Token::Whitespace(byte_offset, c));
                        self.push_scope(LexingScope::RawString, byte_offset);
                        self.token_start = usize::MAX;
                        self.token_rawcontent_start = usize::MAX;
                        self.state = ReadingRaw;
                    },
                    c => {
                        let msg = format!("unexpected character '{c}' while reading raw string start");
                        self.occured_error = Some(errors::Error::InvalidSyntax(msg));
                        self.state = Terminated;
                    },
                }
            },
            ReadingRaw => {
                if self.token_start == usize::MAX {
                    self.token_rawcontent_start = byte_offset;
                    self.token_start = byte_offset;
                }
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
                        self.raw_delimiter_read = 0;
                    }
                }
            },
            EndRaw => {
                match chr {
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::Text(self.token_rawcontent_start..self.token_start));
                        self.next_tokens.push_back(Token::EndRaw(self.token_start..byte_offset));
                        self.token_function_start = usize::MAX;
                        self.token_rawcontent_start = usize::MAX;
                        self.pop_scope(byte_offset);
                    },
                    _ => {
                        let msg = format!("unexpected character '{chr}' - only '}}' after a '>' sequence terminates a raw string");
                        self.occured_error = Some(errors::Error::InvalidSyntax(msg));
                        self.state = Terminated;
                    }
                }
            },
            ReadingCallName => {
                match chr {
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::Call(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::EndFunction(byte_offset));
                        self.token_start = usize::MAX;
                        self.token_function_start = usize::MAX;
                        self.pop_scope(byte_offset);
                    },
                    c if c.is_whitespace() => {
                        self.next_tokens.push_back(Token::Call(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::Whitespace(byte_offset, c));
                        self.push_scope(LexingScope::Content, byte_offset);
                        self.token_start = usize::MAX;
                        self.state = ReadingContent;
                    },
                    OPEN_ARG => {
                        self.next_tokens.push_back(Token::Call(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::BeginArgs);
                        self.token_start = usize::MAX;
                        self.state = FoundArgumentOpening;
                    },
                    _ => {},
                }
            },
            FoundArgumentOpening => {
                match chr {
                    ASSIGN => {
                        self.next_tokens.push_back(Token::ArgKey(self.token_start..byte_offset));
                        self.push_scope(LexingScope::ArgumentValue, byte_offset);
                        self.token_start = usize::MAX;
                        self.state = ReadingArgumentValue;
                    },
                    _ if self.token_start == usize::MAX => {
                        self.token_start = byte_offset;
                    },
                    _ => {},
                }
            },
            FoundArgumentClosing => {
                match chr {
                    OPEN_ARG => {
                        self.token_start = usize::MAX;
                        self.state = FoundArgumentOpening;
                    },
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::EndArgs);
                        self.pop_scope(byte_offset);
                        self.token_start = byte_offset;
                        self.token_function_start = usize::MAX;
                        self.next_tokens.push_back(Token::EndFunction(byte_offset));
                    },
                    c if c.is_whitespace() => {
                        self.next_tokens.push_back(Token::EndArgs);
                        self.next_tokens.push_back(Token::Whitespace(byte_offset, c));
                        self.push_scope(LexingScope::Content, byte_offset);
                        self.token_start = usize::MAX;
                        self.token_rawcontent_start = usize::MAX;
                        self.state = ReadingContent;
                    },
                    _ => {
                        self.state = Terminated;
                        let msg = format!("after ending arguments with '{CLOSE_ARG}', I require a whitespace character to continue with content");
                        self.occured_error = Some(errors::Error::InvalidSyntax(msg));
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_only_text() -> Result<(), errors::Error> {
        let input = "hello";
        let lex = Lexer::new(input);
        let mut iter = lex.iter();
        assert_eq!(iter.next().unwrap()?, Token::Text(0..5));
        Ok(())
    }

    #[test]
    fn lex_only_call() -> Result<(), errors::Error> {
        let input = "{item}";
        let lex = Lexer::new(input);
        let mut iter = lex.iter();
        assert_eq!(iter.next().unwrap()?, Token::BeginFunction(0));
        assert_eq!(iter.next().unwrap()?, Token::Call(1..5));
        assert_eq!(iter.next().unwrap()?, Token::EndFunction(5));
        Ok(())
    }

    #[test]
    fn lex_call_with_arg() -> Result<(), errors::Error> {
        let input = "{item[arg1=3]}";
        let lex = Lexer::new(input);
        let mut iter = lex.iter();
        assert_eq!(iter.next().unwrap()?, Token::BeginFunction(0));
        assert_eq!(iter.next().unwrap()?, Token::Call(1..5));
        assert_eq!(iter.next().unwrap()?, Token::BeginArgs);
        assert_eq!(iter.next().unwrap()?, Token::ArgKey(6..10));
        assert_eq!(iter.next().unwrap()?, Token::BeginArgValue(11));
        assert_eq!(iter.next().unwrap()?, Token::Text(11..12));
        assert_eq!(iter.next().unwrap()?, Token::EndArgValue(12));
        assert_eq!(iter.next().unwrap()?, Token::EndArgs);
        assert_eq!(iter.next().unwrap()?, Token::EndFunction(13));
        Ok(())
    }

    #[test]
    fn lex_call_with_args() -> Result<(), errors::Error> {
        let input = "{element[arg1=3][arg2=42] hello world}";
        let lex = Lexer::new(input);
        let mut iter = lex.iter();
        assert_eq!(iter.next().unwrap()?, Token::BeginFunction(0));
        assert_eq!(iter.next().unwrap()?, Token::Call(1..8));
        assert_eq!(iter.next().unwrap()?, Token::BeginArgs);
        assert_eq!(iter.next().unwrap()?, Token::ArgKey(9..13));
        assert_eq!(iter.next().unwrap()?, Token::BeginArgValue(14));
        assert_eq!(iter.next().unwrap()?, Token::Text(14..15));
        assert_eq!(iter.next().unwrap()?, Token::EndArgValue(15));
        assert_eq!(iter.next().unwrap()?, Token::ArgKey(17..21));
        assert_eq!(iter.next().unwrap()?, Token::BeginArgValue(22));
        assert_eq!(iter.next().unwrap()?, Token::Text(22..24));
        assert_eq!(iter.next().unwrap()?, Token::EndArgValue(24));
        assert_eq!(iter.next().unwrap()?, Token::EndArgs);
        assert_eq!(iter.next().unwrap()?, Token::Whitespace(25, ' '));
        assert_eq!(iter.next().unwrap()?, Token::BeginContent(26));
        assert_eq!(iter.next().unwrap()?, Token::Text(26..37));
        assert_eq!(iter.next().unwrap()?, Token::EndContent(37));
        assert_eq!(iter.next().unwrap()?, Token::EndFunction(37));
        Ok(())
    }

    #[test]
    fn lex_simple_raw_string() -> Result<(), errors::Error> {
        let input = " {<<< text >>>} ";
        let lex = Lexer::new(input);
        let mut iter = lex.iter();
        assert_eq!(iter.next().unwrap()?, Token::Text(0..1));
        assert_eq!(iter.next().unwrap()?, Token::BeginRaw(2..5));
        assert_eq!(iter.next().unwrap()?, Token::Whitespace(5, ' '));
        assert_eq!(iter.next().unwrap()?, Token::Text(6..11));
        assert_eq!(iter.next().unwrap()?, Token::EndRaw(11..14));
        assert_eq!(iter.next().unwrap()?, Token::Text(15..16));
        Ok(())
    }

    #[test]
    fn lex_raw_strings_everywhere() -> Result<(), errors::Error> {
        let input = "{abc[s={< t>}][uv={<<< wx>>>}y]\nte{<< hello>>}xt}";
        let lex = Lexer::new(input);
        let mut iter = lex.iter();
        assert_eq!(iter.next().unwrap()?, Token::BeginFunction(0));
        assert_eq!(iter.next().unwrap()?, Token::Call(1..4));
        assert_eq!(iter.next().unwrap()?, Token::BeginArgs);
        assert_eq!(iter.next().unwrap()?, Token::ArgKey(5..6));
        assert_eq!(iter.next().unwrap()?, Token::BeginArgValue(7));
        assert_eq!(iter.next().unwrap()?, Token::BeginRaw(8..9));
        assert_eq!(iter.next().unwrap()?, Token::Whitespace(9, ' '));
        assert_eq!(iter.next().unwrap()?, Token::Text(10..11));
        assert_eq!(iter.next().unwrap()?, Token::EndRaw(11..12));
        assert_eq!(iter.next().unwrap()?, Token::EndArgValue(13));
        assert_eq!(iter.next().unwrap()?, Token::ArgKey(15..17));
        assert_eq!(iter.next().unwrap()?, Token::BeginArgValue(18));
        assert_eq!(iter.next().unwrap()?, Token::BeginRaw(19..22));
        assert_eq!(iter.next().unwrap()?, Token::Whitespace(22, ' '));
        assert_eq!(iter.next().unwrap()?, Token::Text(23..25));
        assert_eq!(iter.next().unwrap()?, Token::EndRaw(25..28));
        assert_eq!(iter.next().unwrap()?, Token::Text(29..30));
        assert_eq!(iter.next().unwrap()?, Token::EndArgValue(30));
        assert_eq!(iter.next().unwrap()?, Token::EndArgs);
        assert_eq!(iter.next().unwrap()?, Token::Whitespace(31, '\n'));
        assert_eq!(iter.next().unwrap()?, Token::BeginContent(32));
        assert_eq!(iter.next().unwrap()?, Token::Text(32..34));
        assert_eq!(iter.next().unwrap()?, Token::BeginRaw(35..37));
        assert_eq!(iter.next().unwrap()?, Token::Whitespace(37, ' '));
        assert_eq!(iter.next().unwrap()?, Token::Text(38..43));
        assert_eq!(iter.next().unwrap()?, Token::EndRaw(43..45));
        assert_eq!(iter.next().unwrap()?, Token::Text(46..48));
        assert_eq!(iter.next().unwrap()?, Token::EndContent(48));
        assert_eq!(iter.next().unwrap()?, Token::EndFunction(48));
        Ok(())
    }
}