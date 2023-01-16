use std::collections::VecDeque;
use std::fmt;
use std::mem;
use std::ops;
use std::str;

const OPEN_FUNCTION: char = '{'; // U+007B  LEFT CURLY BRACKET
const CLOSE_FUNCTION: char = '}'; // U+007D  RIGHT CURLY BRACKET
const OPEN_ARG: char = '['; // U+005B  LEFT SQUARE BRACKET
const CLOSE_ARG: char = ']'; // U+005D  RIGHT SQUARE BRACKET
const ASSIGN: char = '='; // U+003D  EQUALS SIGN

#[derive(Clone,Debug,PartialEq)]
pub struct Lexer<'l> {
    source: &'l str,
}

impl<'l> Lexer<'l> {
    pub fn new<'a>(src: &'a str) -> Lexer<'a> {
        Lexer { source: src }
    }

    pub fn iter(&'l self) -> LexingIterator {
        LexingIterator::new(self.source)
    }
}

#[derive(Clone,Debug,Hash,PartialEq)]
enum LexingScope {
    ContentInDocument,
    ContentInFunction,
    ArgumentValueInFunction,
    FunctionInContent,
    FunctionInArgumentValue,
}

#[derive(Clone,Debug,PartialEq)]
pub(crate) enum LexingState {
    ReadingContent,
    ReadingContentText,
    ReadingArgumentValue,
    ReadingArgumentValueText,
    FoundCallOpening,
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
            LexingState::ReadingCallName => write!(f, "reading the name of a function call"),
            LexingState::FoundArgumentOpening => write!(f, "reading a function argument"),
            LexingState::FoundArgumentClosing => write!(f, "finishing one function argument"),
            LexingState::Terminated => write!(f, "terminating"),
        }
    }
}

#[derive(Debug)]
pub struct LexingIterator<'l> {
    /// state of this iterator
    state: LexingState,
    /// byte offset where the current token started
    token_start: usize,
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
    next_tokens: VecDeque<Token>,
    /// if an error occured, the error is returned once
    /// and the lexer switches to the infinite EOF state
    occured_error: Option<anyhow::Error>,
}

impl<'l> LexingIterator<'l> {
    pub fn new(src: &str) -> LexingIterator {
        LexingIterator {
            state: LexingState::ReadingContent,
            token_start: 0,
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
                self.occured_error = Some(anyhow::anyhow!("unbalanced end of function call at byte {}", self.token_start));
                return LexingScope::ContentInFunction; // NOTE: arbitrary token
            }
        };

        match top.clone() {
            ContentInDocument => {
                self.pop_scope(byte_offset);
            },
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

        //println!("{:?}", self.stack);

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
                self.state = Terminated;
                return Some(Token::EOF(self.token_start));
            },
        };

        match self.state {
            ReadingContent => {
                match chr {
                    OPEN_FUNCTION => {
                        self.next_tokens.push_back(Token::BeginFunction(byte_offset));
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
                    },
                }
            },
            ReadingContentText => {
                match chr {
                    OPEN_FUNCTION => {
                        self.next_tokens.push_back(Token::Text(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::BeginFunction(byte_offset));
                        self.push_scope(LexingScope::FunctionInContent, byte_offset);
                        self.state = FoundCallOpening;
                    },
                    CLOSE_FUNCTION => {
                        self.next_tokens.push_back(Token::Text(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::EndContent(byte_offset));
                        assert_eq!(self.pop_scope(byte_offset), LexingScope::ContentInFunction);
                    },
                    _ => {},
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
                        self.next_tokens.push_back(Token::BeginFunction(byte_offset));
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
                        self.next_tokens.push_back(Token::BeginFunction(byte_offset));
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
                        self.state = Terminated;
                        self.occured_error = Some(anyhow::anyhow!("the call '{}' was immediately close by '{}' - empty calls are not allowed", OPEN_FUNCTION, CLOSE_FUNCTION));
                    },
                    _ => {
                        self.state = ReadingCallName;
                        self.token_start = byte_offset;
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

                match chr {
                    ASSIGN => {
                        self.next_tokens.push_back(Token::ArgKey(self.token_start..byte_offset));
                        self.push_scope(LexingScope::ArgumentValueInFunction, byte_offset);
                        self.token_start = usize::MAX; // invalidate value
                        self.state = ReadingArgumentValue;
                    },
                    _ => {},
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
                    _ => {
                        self.next_tokens.push_back(Token::EndArgs);
                        self.next_tokens.push_back(Token::BeginContent(byte_offset));
                        self.push_scope(LexingScope::ContentInFunction, byte_offset);
                        self.state = ReadingContent;
                    }
                }
            },
            Terminated => {},
        }

        self.next_tokens.pop_front()
    }

    pub(crate) fn emit_occured_error(&mut self) -> Option<anyhow::Error> {
        mem::take(&mut self.occured_error)
    }
}

#[derive(Clone,Debug,PartialEq)]
pub enum Token {
    BeginFunction(usize),
    Call(ops::Range<usize>),
    BeginArgs,
    ArgKey(ops::Range<usize>),
    BeginArgValue(usize),
    EndArgValue(usize),
    EndArgs,
    BeginContent(usize),
    EndContent(usize),
    EndFunction(usize),
    Text(ops::Range<usize>),
    EOF(usize),
}

impl Eq for Token {}

impl<'l> Iterator for LexingIterator<'l> {
    /// An item identifies when this token started (UTF-8 byte offset)
    /// and whether we get an error here (Err) or some token (Ok).
    type Item = anyhow::Result<Token>;

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
                Some(Token::EOF(pos)) => return Some(Ok(Token::EOF(pos))),
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
