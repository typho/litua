use anyhow;

use std::collections::HashMap;
use std::iter;
use std::ops;
use std::ffi::OsString;
use std::thread::current;

use crate::tree;
use crate::lexer;

pub struct Parser<'s> {
    filepath: OsString,
    source_code: &'s str,
    tree: tree::DocumentNode,
    path: Vec<usize>,
}

impl<'s> Parser<'s> {
    pub fn new(filepath: OsString, source_code: &'s str) -> Parser<'s> {
        Parser{
            filepath: filepath.clone(),
            source_code,
            tree: tree::DocumentNode::new(),
            path: Vec::new(),
        }
    }

    fn get_start_pos_from_token(current_pos: usize, tok: lexer::Token) -> usize {
        match tok {
            lexer::Token::BeginFunction(pos) |
            lexer::Token::BeginArgValue(pos) |
            lexer::Token::EndArgValue(pos) |
            lexer::Token::BeginContent(pos) |
            lexer::Token::EndContent(pos) |
            lexer::Token::EndFunction(pos) |
            lexer::Token::EOF(pos) => pos,
            lexer::Token::Call(range) |
            lexer::Token::ArgKey(range) |
            lexer::Token::Text(range) => range.start,
            lexer::Token::BeginArgs | lexer::Token::EndArgs => current_pos,
        }
    }

    #[inline]
    fn unexpected_token<T>(tok: &lexer::Token) -> anyhow::Result<T> {
        Err(anyhow::anyhow!("unexpected token {:?}", tok))
    }

    #[inline]
    fn unexpected_eof<T>() -> anyhow::Result<T> {
        Err(anyhow::anyhow!("unexpected end of lexer tokens iterator"))
    }

    pub fn parse_content(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<tree::DocumentNode> {
        let mut content = tree::DocumentNode::new();

        // (1) consume BeginContent
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::BeginContent(_) => {
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EOF(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        // (2) loop
        loop {
            // admissible tokens
            enum NextToken {
                BeginFunction,
                Text,
                EndContent,
                Unexpected,
            }

            let mut next_token = NextToken::Unexpected;

            match iter.peek() {
                Some(token_or_err) => {
                    next_token = match token_or_err {
                        Ok(lexer::Token::BeginFunction(_)) => NextToken::BeginFunction,
                        Ok(lexer::Token::Text(_)) => NextToken::Text,
                        Ok(lexer::Token::EndContent(_)) => NextToken::EndContent,
                        _ => NextToken::Unexpected,
                    };
                },
                _ => {},
            }

            match next_token {
                NextToken::BeginFunction => {
                    // (3)   if BeginFunction
                    // (4)     parse_function
                    let func = self.parse_function(iter)?;
                    content.push(func);
                },
                NextToken::Text => {
                    // (5)   if Text
                    // (6)     add text
                    if let Some(Ok(lexer::Token::Text(range))) = iter.next() {
                        let text = &self.source_code[range];
                        content.push(tree::DocumentElement::Text(text.to_owned()));
                    }
                },
                NextToken::EndContent => break,
                NextToken::Unexpected => {
                    // protocol violation
                    match iter.next() {
                        Some(Ok(tok)) => return Self::unexpected_token(&tok),
                        Some(Err(err)) => Err(err)?,
                        None => return Self::unexpected_eof(),
                    }
                },
            }
        }
        // (8) consume EndContent
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::EndContent(_) => {
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EOF(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        Ok(content)
    }

    pub fn parse_argument_value(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<tree::DocumentNode> {
        let mut arg_value = tree::DocumentNode::new();

        // (1) consume BeginArgValue
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::BeginArgValue(_) => {
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EOF(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        // (2) loop
        loop {
            // admissible tokens
            enum NextToken {
                BeginFunction,
                Text,
                EndArgValue,
                Unexpected,
            }

            let mut next_token = NextToken::Unexpected;

            match iter.peek() {
                Some(token_or_err) => {
                    next_token = match token_or_err {
                        Ok(lexer::Token::BeginFunction(_)) => NextToken::BeginFunction,
                        Ok(lexer::Token::Text(_)) => NextToken::Text,
                        Ok(lexer::Token::EndArgValue(_)) => NextToken::EndArgValue,
                        _ => NextToken::Unexpected,
                    };
                },
                _ => {},
            }

            match next_token {
                NextToken::BeginFunction => {
                    // (3)   if BeginFunction
                    // (4)     parse_function
                    let func = self.parse_function(iter)?;
                    arg_value.push(func);
                },
                NextToken::Text => {
                    // (5)   if Text
                    // (6)     add text
                    if let Some(Ok(lexer::Token::Text(range))) = iter.next() {
                        let content = &self.source_code[range];
                        arg_value.push(tree::DocumentElement::Text(content.to_owned()));
                    }
                },
                NextToken::EndArgValue => break,
                NextToken::Unexpected => {
                    // protocol violation
                    match iter.next() {
                        Some(Ok(tok)) => return Self::unexpected_token(&tok),
                        Some(Err(err)) => Err(err)?,
                        None => return Self::unexpected_eof(),
                    }
                },
            }
        }

        // (8) consume EndArgValue
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::EndArgValue(_) => {
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EOF(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        Ok(arg_value)
    }

    pub fn parse_function(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<tree::DocumentElement> {
        let mut func = tree::DocumentFunction::new();

        // (01) consume BeginFunction
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::BeginFunction(_) => {
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EOF(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        // (02) consume Call
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::Call(range) => {
                        let name = &self.source_code[range];
                        func.name = name.to_owned();
                    },
                    lexer::Token::EOF(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        // (03) if BeginArgs
        if let Some(Ok(lexer::Token::BeginArgs)) = iter.peek() {
            // (04)   consume BeginArgs
            match iter.next() {
                Some(tok_or_err) => {
                    let token = tok_or_err?;
                    match token {
                        lexer::Token::BeginArgs => {
                            // NOTE: expected token, yay!
                        },
                        lexer::Token::EOF(_) => return Self::unexpected_eof(),
                        _ => return Self::unexpected_token(&token),
                    }
                },
                None => return Self::unexpected_eof(),
            }

            // (05)   loop if ArgKey
            loop {
                if let Some(Ok(lexer::Token::ArgKey(_))) = iter.peek() {
                    // NOTE: ok, we consume an argument key-value pair
                } else {
                    break;
                }

                // (06)     consume ArgKey
                let arg_name = match iter.next() {
                    Some(token_or_err) => {
                        let token = token_or_err?;
                        match token {
                            lexer::Token::EndArgs => {
                                // NOTE: end of arguments? Ok.
                                break;
                            },
                            lexer::Token::ArgKey(range) => {
                                (&self.source_code[range]).to_owned()
                            }
                            lexer::Token::EOF(_) => return Self::unexpected_eof(),
                            _ => return Self::unexpected_token(&token),
                        }
                    },
                    None => return Self::unexpected_eof(),
                };

                // (07)     parse_argument_value
                let arg_value = self.parse_argument_value(iter)?;
                func.args.insert(arg_name, arg_value);
            }

            // (08)   consume EndArgs
            match iter.next() {
                Some(tok_or_err) => {
                    let token = tok_or_err?;
                    match token {
                        lexer::Token::EndArgs => {
                            // NOTE: expected token, yay!
                        },
                        lexer::Token::EOF(_) => return Self::unexpected_eof(),
                        _ => return Self::unexpected_token(&token),
                    }
                },
                None => return Self::unexpected_eof(),
            }
        }

        // (09) if BeginContent
        let mut found_content = false;
        if let Some(Ok(lexer::Token::BeginContent(_))) = iter.peek() {
            found_content = true;
        }

        if found_content {
            // (10)   parse_content
            func.content = self.parse_content(iter)?;
        }

        // (11) consume EndFunction
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::EndFunction(_) => {
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EOF(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        Ok(tree::DocumentElement::Function(func))
    }

    pub fn consume_iter(&mut self, iter: lexer::LexingIterator) -> anyhow::Result<()> {
        let mut peekable_iter = iter.peekable();

        // admissible tokens
        enum NextToken {
            BeginFunction,
            Text,
            EOF,
            Unexpected,
        }

        loop {
            let mut next_token = NextToken::Unexpected;

            match peekable_iter.peek() {
                Some(token_or_err) => {
                    next_token = match token_or_err {
                        Ok(lexer::Token::BeginFunction(_)) => NextToken::BeginFunction,
                        Ok(lexer::Token::Text(_)) => NextToken::Text,
                        Ok(lexer::Token::EOF(_)) => NextToken::EOF,
                        _ => NextToken::Unexpected,
                    };
                },
                _ => {},
            }

            match next_token {
                NextToken::BeginFunction => {
                    let func = self.parse_function(&mut peekable_iter)?;
                    self.tree.push(func);
                },
                NextToken::Text => {
                    if let Some(Ok(lexer::Token::Text(range))) = peekable_iter.next() {
                        let text = &self.source_code[range];
                        self.tree.push(tree::DocumentElement::Text(text.to_owned()));
                    }
                },
                NextToken::EOF => {
                    // Already done? How sad.
                    break;
                },
                NextToken::Unexpected => {
                    // protocol violation
                    match peekable_iter.next() {
                        Some(Ok(tok)) => return Err(anyhow::anyhow!("unexpected token {:?}", tok)),
                        Some(Err(err)) => Err(err)?,
                        None => return Err(anyhow::anyhow!("unexpected end of lexer tokens iterator")),
                    }
                },
            }
        }

        Ok(())
    }

    pub fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn tree(self) -> tree::DocumentTree {
        let mut args = HashMap::new();
        if !self.filepath.is_empty() {
            if let Some(fp) = self.filepath.to_str() {
                args.insert("filepath".to_owned(), vec![tree::DocumentElement::Text(fp.to_owned())]);
            }
        }

        let elem = tree::DocumentElement::Function(tree::DocumentFunction {
            name: "document".to_owned(),
            args,
            content: self.tree,
        });
        tree::DocumentTree(elem)
    }
}




pub(crate) struct DebuggingParser<'s> {
    filepath: OsString,
    source_code: &'s str,
}

impl<'s> DebuggingParser<'s> {

    pub(crate) fn show_token(name: &str, indent: &mut i32, indent_change: i32) {
        if indent_change < 0 { (*indent) += indent_change; }
        print!("{}", "  ".repeat(*indent as usize));
        println!("{}", name);
        if indent_change >= 0 { (*indent) += indent_change; }
    }

    pub(crate) fn show_pos(name: &str, pos: usize, indent: &mut i32, indent_change: i32, src: &str) {
        if indent_change < 0 { (*indent) += indent_change; }
        print!("{}", "  ".repeat(*indent as usize));
        let content: char = match &src[pos..].chars().next() {
            Some(c) => *c,
            None => panic!("invalid UTF-8 offset position {} in token {} received", pos, name),
        };
        println!("{}({})", name, content);
        if indent_change >= 0 { (*indent) += indent_change; }
    }

    pub(crate) fn show_range(name: &str, range: ops::Range<usize>, indent: &mut i32, indent_change: i32, src: &str) {
        if indent_change < 0 { (*indent) += indent_change; }
        print!("{}", "  ".repeat(*indent as usize));
        let content: &str = &src[range];
        println!("{}({})", name, content);
        if indent_change >= 0 { (*indent) += indent_change; }
    }

    pub(crate) fn consume_iter(&self, iter: lexer::LexingIterator) {
        let mut indent = 0;
        for tok_or_err in iter {
            match tok_or_err {
                Ok(tok) => {
                    match tok {
                        lexer::Token::BeginFunction(pos) => Self::show_pos("BeginFunction", pos, &mut indent, 1, self.source_code),
                        lexer::Token::Call(range) => Self::show_range("Call", range, &mut indent, 0, self.source_code),
                        lexer::Token::BeginArgs => Self::show_token("BeginArgs", &mut indent, 1),
                        lexer::Token::ArgKey(range) => Self::show_range("ArgKey", range, &mut indent, 0, self.source_code),
                        lexer::Token::BeginArgValue(pos) => Self::show_pos("BeginArgValue", pos, &mut indent, 1, self.source_code),
                        lexer::Token::EndArgValue(pos) => Self::show_pos("EndArgValue", pos, &mut indent, -1, self.source_code),
                        lexer::Token::EndArgs => Self::show_token("EndArgs", &mut indent, -1),
                        lexer::Token::BeginContent(pos) => Self::show_pos("BeginContent", pos, &mut indent, 1, self.source_code),
                        lexer::Token::EndContent(pos) => Self::show_pos("EndContent", pos, &mut indent, -1, self.source_code),
                        lexer::Token::EndFunction(pos) => Self::show_pos("EndFunction", pos, &mut indent, -1, self.source_code),
                        lexer::Token::Text(range) => Self::show_range("Text", range, &mut indent, 0, self.source_code),
                        lexer::Token::EOF(_) => Self::show_token("EOF", &mut indent, 0),
                    }
                },
                Err(e) => { println!("{:?}", e); break },
            }
        }
    }
}
