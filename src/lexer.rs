use std::collections::VecDeque;
use std::fmt;
use std::mem;
use std::ops;
use std::str;

const OPEN_CALL: char = '{'; // U+007B  LEFT CURLY BRACKET
const CLOSE_CALL: char = '}'; // U+007D  RIGHT CURLY BRACKET
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

#[derive(Clone,Debug,PartialEq)]
enum LexingScope {
    ContentInCall,
    ArgumentValueInCall,
    CallInContent,
    CallInArgumentValue,
}

#[derive(Clone,Debug,PartialEq)]
pub(crate) enum LexingState {
    ReadingContent,
    ReadingArgumentValue,
    FoundCallOpening,
    ReadingCallName,
    FoundArgumentOpening,
    FoundArgumentClosing,
    EOF,
    Error,
}

impl fmt::Display for LexingState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadingContent => write!(f, "reading text"),
            ReadingArgumentValue => write!(f, "reading argument value"),
            FoundCallOpening => write!(f, "reading the start of a function call"),
            ReadingCallName => write!(f, "reading the name of a function call"),
            FoundArgumentOpening => write!(f, "reading a function argument"),
            FoundArgumentClosing => write!(f, "finishing one function argument"),
            EOF => write!(f, "reaching end of the file"),
            Error => write!(f, "failing due to an error"),
        }
    }
}

#[derive(Debug)]
pub struct LexingIterator<'l> {
    /// state of this iterator
    state: LexingState,
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
    /// byte offset where the current token started
    token_start: usize,
    /// if an error occured, the error is returned once
    /// and the lexer switches to the infinite EOF state
    occured_error: Option<anyhow::Error>,
}

impl<'l> LexingIterator<'l> {
    fn new(src: &str) -> LexingIterator {
        let mut toks = VecDeque::new();
        toks.push_back(Token::BeginContent(0));
        LexingIterator {
            state: LexingState::ReadingContent,
            chars: src.char_indices(),
            stack: Vec::new(),
            next_tokens: toks,
            token_start: 0,
            occured_error: None,
        }
    }
}

impl<'l> LexingIterator<'l> {
    fn push_scope(&mut self, sc: LexingScope, byte_offset: usize) {
        self.token_start = byte_offset;
        self.stack.push(sc);
    }

    fn pop_scope(&mut self, byte_offset: usize) -> LexingScope {
        use LexingScope::*;

        let top = self.stack.pop().unwrap();

        match top.clone() {
            ArgumentValueInCall => {
                self.state = LexingState::FoundArgumentClosing;
            },
            CallInContent => {
                self.next_tokens.push_back(Token::BeginContent(byte_offset));
                self.state = LexingState::ReadingContent;
            },
            CallInArgumentValue => {
                self.state = LexingState::ReadingArgumentValue;
            },
            ContentInCall => {
                self.next_tokens.push_back(Token::EndFunction(byte_offset));
                self.pop_scope(byte_offset);
            },
        };

        self.token_start = byte_offset;
        top
    }

    /// Continue reading the next Unicode scalar.
    /// Maybe the result is some Token to omit or maybe the result is None,
    /// since the token consists of multiple scalars.
    pub(crate) fn progress(&mut self) -> Option<anyhow::Result<Token>> {
        use LexingState::*;

        if self.occured_error.is_some() {
            return None;
        }

        let front = self.next_tokens.pop_front();
        if let Some(tok) = front {
            return Some(Ok(tok));
        }

        if self.state == Error || self.state == EOF { 
            return None;
        }

        let (byte_offset, chr) = match self.chars.next() {
            Some((bo, ch)) => (bo, ch),
            None => return Some(Ok(Token::EOF)),
        };

        match self.state {
            ReadingContent => {
                match chr {
                    OPEN_CALL => {
                        self.next_tokens.push_back(Token::EndContent(byte_offset));
                        self.next_tokens.push_back(Token::BeginFunction(byte_offset));
                        self.push_scope(LexingScope::CallInContent, byte_offset);
                        self.state = FoundCallOpening;
                    },
                    CLOSE_CALL => {
                        self.next_tokens.push_back(Token::EndContent(byte_offset));
                        assert_eq!(self.pop_scope(byte_offset), LexingScope::ContentInCall);
                    },
                    _ => {},
                }
            },
            ReadingArgumentValue => {
                match chr {
                    OPEN_CALL => {
                        self.next_tokens.push_back(Token::BeginFunction(byte_offset));
                        self.push_scope(LexingScope::CallInArgumentValue, byte_offset);
                        self.state = FoundCallOpening;
                    },
                    CLOSE_ARG => {
                        self.next_tokens.push_back(Token::EndArgValue(byte_offset));
                        assert_eq!(self.pop_scope(byte_offset), LexingScope::ArgumentValueInCall);
                    },
                    _ => {},
                }
            },
            FoundCallOpening => {
                match chr {
                    CLOSE_CALL => {
                        self.state = Error;
                        return Some(Err(anyhow::anyhow!("the call '{}' was immediately close by '{}' - empty calls are not allowed", OPEN_CALL, CLOSE_CALL)));
                    },
                    _ => {
                        self.state = ReadingCallName;
                        self.token_start = byte_offset;
                    }
                }
            },
            ReadingCallName => {
                match chr {
                    CLOSE_CALL => {
                        self.next_tokens.push_back(Token::Call(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::EndFunction(byte_offset));
                        self.pop_scope(byte_offset);
                    },
                    c if c.is_whitespace() => {
                        self.next_tokens.push_back(Token::Call(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::BeginContent(byte_offset));
                        self.push_scope(LexingScope::ContentInCall, byte_offset);
                        self.state = ReadingContent;
                    },
                    OPEN_ARG => {
                        self.next_tokens.push_back(Token::Call(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::BeginArgs);
                        self.state = FoundArgumentOpening;
                        self.token_start = byte_offset;
                    },
                    _ => {},
                }
            },
            FoundArgumentOpening => {
                match chr {
                    ASSIGN => {
                        self.next_tokens.push_back(Token::ArgKey(self.token_start..byte_offset));
                        self.next_tokens.push_back(Token::BeginArgValue(byte_offset));
                        self.push_scope(LexingScope::ArgumentValueInCall, byte_offset);
                        self.state = ReadingArgumentValue;
                    },
                    _ => {},
                }
            },
            FoundArgumentClosing => {
                match chr {
                    OPEN_ARG => {
                        self.state = FoundArgumentOpening;
                        self.token_start = byte_offset;
                    },
                    CLOSE_CALL => {
                        self.next_tokens.push_back(Token::EndArgs);
                        self.pop_scope(byte_offset);
                    },
                    _ => {
                        self.next_tokens.push_back(Token::EndArgs);
                        self.next_tokens.push_back(Token::BeginContent(byte_offset));
                        self.push_scope(LexingScope::ContentInCall, byte_offset);
                        self.state = ReadingContent;
                    }
                }
            },
            EOF => {
                assert_eq!(self.stack.len(), 0);
                return None;
            },
            Error => {
                return None;
            },
        }

        None
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
    EOF,
}

impl Eq for Token {}

impl Token {
    pub fn format_with_src(&self, source: &str) -> String {
        use Token::*;
        match self {
            BeginFunction(pos) => match source.get(*pos..) {
                Some(substr) => format!("BeginFunction @ {}({})", pos, substr.chars().take(1).collect::<String>()),
                None => format!("BeginFunction @ {}", pos),
            },
            Call(range) => match source.get(range.start..range.end) {
                Some(substr) => format!("Call({})", substr),
                None => format!("Call()"),
            },
            BeginArgs => format!("BeginArgs()"),
            ArgKey(range) => match source.get(range.start..range.end) {
                Some(substr) => format!("ArgKey({})", substr),
                None => format!("ArgKey()"),
            },
            BeginArgValue(pos) => match source.get(*pos..) {
                Some(substr) => format!("BeginArgValue @ {}({:?})", pos, substr.chars().take(1).collect::<String>()),
                None => format!("BeginArgValue @ {}", pos),
            },
            EndArgValue(pos) => match source.get(*pos..) {
                Some(substr) => format!("EndArgValue @ {}({:?})", pos, substr.chars().take(1).collect::<String>()),
                None => format!("EndArgValue @ {}", pos),
            },
            EndArgs => format!("EndArgs()"),
            BeginContent(pos) => match source.get(*pos..) {
                Some(substr) => format!("BeginContent @ {}({:?})", pos, substr.chars().take(1).collect::<String>()),
                None => format!("BeginContent @ {}", pos),
            },
            EndContent(pos) => match source.get(*pos..) {
                Some(substr) => format!("EndContent @ {}({:?})", pos, substr.chars().take(1).collect::<String>()),
                None => format!("EndContent @ {}", pos),
            },
            EndFunction(pos) => match source.get(*pos..) {
                Some(substr) => format!("EndFunction @ {}({:?})", pos, substr.chars().take(1).collect::<String>()),
                None => format!("EndFunction @ {}", pos),
            },
            EOF => format!("EOF"),
        }
    }
}

impl<'l> Iterator for LexingIterator<'l> {
    /// An item identifies when this token started (UTF-8 byte offset)
    /// and whether we get an error here (Err) or some token (Ok).
    //type Item = (usize, anyhow::Result<Token>);
    type Item = anyhow::Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.progress() {
                Some(Ok(Token::EOF)) => {
                    assert!(self.stack.is_empty());
                    return None;
                },
                Some(tok) => return Some(tok),
                None => continue,
            }
        }
    }
}
