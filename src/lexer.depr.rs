use std::fmt;
use std::fs;
use std::mem;
use std::path;
use std::str;

use std::io::Read;
use std::sync::ONCE_INIT;

const OPEN_CHAR: char = '{'; // U+007B  LEFT CURLY BRACKET
const CLOSE_CHAR: char = '}'; // U+007D  RIGHT CURLY BRACKET

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
enum LexingState {
    ReadingContent,
    FoundCallOpening,
    ReadingCallName,
    FoundArgumentOpening,
    ReadingArgumentKey,
    FoundArgumentAssignment,
    ReadingArgumentValue,
    FoundArgumentClosing,
    EOF,
    Error,
}

impl fmt::Display for LexingState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadingContent => write!(f, "reading text"),
            FoundCallOpening => write!(f, "reading the start of a function call"),
            ReadingCallName => write!(f, "reading the name of a function call"),
            FoundArgumentOpening => write!(f, "reading the start of a function argument"),
            ReadingArgumentKey => write!(f, "reading key of a function argument"),
            FoundArgumentAssignment => write!(f, "reading equality symbol '=' in a function argument"),
            ReadingArgumentValue => write!(f, "reading value of a function argument"),
            FoundArgumentClosing => write!(f, "reading the end of a function argument"),
            EOF => write!(f, "reaching end of the file"),
            Error => write!(f, "failing due to an error"),
        }
    }
}

const IN_ARGUMENT: bool = false;
const IN_CALL: bool = true;

#[derive(Clone,Debug)]
pub struct LexingIterator<'l> {
    chars: str::CharIndices<'l>,
    byte_pos: usize,
    state: LexingState,
    /// `layer` specifies whether we start a call inside an argument
    /// or as part of the content
    layer: Vec<bool>,
    /// `next_token` stores the next token to emit. The return value of
    /// `progress()` is one token, but sometimes two tokens are generated.
    /// In this case, the first is emitted and the second emitted on the
    /// next call.
    next_token: Option<Token>,
    cache: String,
}

impl<'l> LexingIterator<'l> {
    fn new(src: &str) -> LexingIterator {
        LexingIterator {
            chars: src.char_indices(),
            byte_pos: 0,
            state: LexingState::ReadingContent,
            layer: Vec::new(),
            next_token: None,
            cache: String::new(),
        }
    }
}

impl<'l> LexingIterator<'l> {
    /// Continue reading the next Unicode scalar.
    /// Maybe the result is some Token to omit or maybe the result is None,
    /// since the token consists of multiple scalars.
    fn progress(&mut self) -> Option<Token> {
        use LexingState::*;

        if self.next_token.is_some() {
            return mem::replace(&mut self.next_token, None);
        }

        if self.state == Error || self.state == EOF { 
            return None;
        }

        let (byte_offset, chr) = match self.chars.next() {
            Some((bo, ch)) => (bo, ch),
            None => return Some(Token::EOF),
        };

        match self.state {
            ReadingContent => {
                match chr {
                    OPEN_CHAR => {
                        self.state = FoundCallOpening;
                        self.layer.push(IN_CALL);
                        if !self.cache.is_empty() {
                            self.next_token = Some(Token::BeginFunction);
                            return Some(Token::Content(self.swap_text()));
                        } else {
                            return Some(Token::BeginFunction);
                        }
                    },
                    CLOSE_CHAR => {
                        match self.layer.pop() {
                            Some(IN_CALL) => {
                                println!("  in call");
                                self.layer.push(IN_CALL);
                                self.next_token = Some(Token::CloseFunction);
                                return Some(Token::Content(self.swap_text()));
                            },
                            Some(IN_ARGUMENT) => {
                                println!("  in argument");
                                self.state = ReadingArgumentValue;
                                self.next_token = Some(Token::CloseFunction);
                                return Some(Token::ArgValue(self.swap_text()));
                            },
                            None => todo!(),
                        }
                    },
                    _ => self.cache.push(chr),
                }
            },
            FoundCallOpening => {
                match chr {
                    CLOSE_CHAR => {
                        self.state = Error;
                        return Some(Token::Error(format!("cannot call a function without a name; '{}{}' is not allowed", OPEN_CHAR, CLOSE_CHAR)));
                    },
                    _ => {
                        self.cache.push(chr);
                        if !chr.is_whitespace() {
                            self.state = ReadingCallName;
                        }
                    },
                }
            },
            ReadingCallName => {
                match chr {
                    CLOSE_CHAR => {
                        assert_eq!(self.layer.pop(), Some(IN_CALL));
                        self.state = ReadingContent;
                    },
                    OPEN_CHAR => {
                        self.state = FoundArgumentOpening;
                        self.layer.push(IN_ARGUMENT);
                        return Some(Token::Call(self.swap_text()));
                    },
                    _ if chr.is_whitespace() => {
                        self.state = ReadingContent;
                        return Some(Token::Call(self.swap_text()));
                    },
                    _ => self.cache.push(chr),

                }
            },
            FoundArgumentOpening => {
                match chr {
                    CLOSE_CHAR => {
                        self.state = Error;
                        return Some(Token::Error(format!("'{}{}' is not allowed as argument", OPEN_CHAR, CLOSE_CHAR)));
                    },
                    '=' => {
                        self.state = Error;
                        return Some(Token::Error(format!("argument key cannot be an empty string")));
                    },
                    _ => {
                        self.state = ReadingArgumentKey;
                        self.cache.push(chr);
                    }
                }
            },
            ReadingArgumentKey => {
                match chr {
                    '=' => {
                        self.state = FoundArgumentAssignment;
                        return Some(Token::ArgKey(self.swap_text()));
                    },
                    _ => self.cache.push(chr),
                }
            },
            FoundArgumentAssignment => {
                match chr {
                    CLOSE_CHAR => {
                        self.state = FoundArgumentClosing;
                        assert_eq!(self.layer.pop(), Some(IN_ARGUMENT));
                        return Some(Token::ArgValue("".to_owned()));
                    },
                    OPEN_CHAR => {
                        self.layer.push(IN_CALL);
                        self.state = FoundCallOpening;
                        return Some(Token::BeginFunction);
                    },
                    _ => {
                        self.state = ReadingArgumentValue;
                        self.cache.push(chr)
                    },
                }
            },
            ReadingArgumentValue => {
                match chr {
                    CLOSE_CHAR => {
                        self.state = FoundArgumentClosing;
                        assert_eq!(self.layer.pop(), Some(IN_ARGUMENT));
                        return Some(Token::ArgValue(self.swap_text()));
                    },
                    OPEN_CHAR => {
                        self.state = Error;
                        return Some(Token::Error(format!("argument value must not contain braces")));
                    },
                    _ => {
                        self.cache.push(chr);
                    }
                }

            },
            FoundArgumentClosing => {
                match chr {
                    CLOSE_CHAR => match self.layer.pop() {
                        Some(IN_ARGUMENT) => {
                            self.state = ReadingArgumentValue;
                        },
                        Some(IN_CALL) => {
                            self.state = ReadingContent;
                        },
                        None => {},
                    },
                    OPEN_CHAR => {
                        self.layer.push(IN_ARGUMENT);
                        self.state = FoundArgumentOpening;
                    },
                    _ if chr.is_whitespace() => {
                        self.state = ReadingContent;
                    },
                    _ => {
                        self.state = Error;
                        return Some(Token::Error("after function arguments, a whitespace character must occur".to_owned()));
                    }
                }
            },
            EOF => {
                assert_eq!(self.layer.len(), 0);
                return None;
            },
            Error => {
                return None;
            },
        }

        None
    }

    fn swap_text(&mut self) -> String {
        std::mem::replace(&mut self.cache, String::new())
    }
}

#[derive(Clone,Debug,PartialEq)]
pub enum Token {
    BeginFunction,
    Call(String),
    BeginArgs,
    ArgKey(String),
    BeginArgValue,
    EndArgValue,
    EndArgs,
    Content(String),
    CloseFunction,
    EOF,
    Error(String),
}

impl Eq for Token {}

/*impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Token::*;

        match self {
            BeginFunction | BeginArg => write!(f, "{}", OPEN_CHAR),
            Call(name) => write!(f, "{}", name),
            ArgKey(key) => write!(f, "{}", key),
            ArgValue(val) => write!(f, "{}", val),
            EndArgValue | CloseFunction => write!(f, "{}", CLOSE_CHAR),
            Content(text) => write!(f, "{}", text),
            EOF => write!(f, ""),
            Error(errmsg) => write!(f, "{}", errmsg),
        }
    }
}*/

impl<'l> Iterator for LexingIterator<'l> {
    type Item = anyhow::Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.progress() {
                Some(Token::EOF) => return None,
                Some(Token::Error(errmsg)) => return None,
                Some(t) => return Some(Ok(t)),
                None => continue,
            }
        }
    }
}
