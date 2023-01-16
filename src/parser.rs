//! Parser for litua text documents

use std::collections::HashMap;
use std::iter;
use std::ops;
use std::path;
use std::ffi::OsString;

use crate::tree;
use crate::lexer;

/// `Parser` holds a reference to the text document source code.
/// To generate better error messages, we also store the filepath.
/// The parsing process fills a tree with data.
///
/// A typical parsing process is done with the following methods:
/// `consume_iter(iter)` takes a `LexingIterator` and consumes the
/// generated tokens. Then `finalize` declares the termination of
/// the token consumption. Finally one can fetch the resulting
/// abstract syntax tree by calling the method `tree()`.
pub struct Parser<'s> {
    pub filepath: path::PathBuf,
    pub source_code: &'s str,
    pub tree: tree::DocumentNode,
}

impl<'s> Parser<'s> {
    pub fn new(filepath: &path::Path, source_code: &'s str) -> Parser<'s> {
        Parser{
            filepath: filepath.to_owned(),
            source_code,
            tree: tree::DocumentNode::new(),
        }
    }

    fn get_start_pos_of_token(current_pos: usize, tok: lexer::Token) -> usize {
        match tok {
            lexer::Token::BeginFunction(pos) |
            lexer::Token::BeginArgValue(pos) |
            lexer::Token::EndArgValue(pos) |
            lexer::Token::BeginContent(pos) |
            lexer::Token::EndContent(pos) |
            lexer::Token::EndFunction(pos) |
            lexer::Token::Whitespace(pos, _) |
            lexer::Token::EndOfFile(pos) => pos,
            lexer::Token::Call(range) |
            lexer::Token::ArgKey(range) |
            lexer::Token::BeginRaw(range) |
            lexer::Token::EndRaw(range) |
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

    fn parse_raw(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<tree::DocumentElement> {
        let whitespace;
        let name;
        let text;

        // (1) consume BeginRaw
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::BeginRaw(range) => {
                        // NOTE: expected token, yay!
                        name = &self.source_code[range];
                    },
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        // (2) consume Whitespace
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::Whitespace(_, ws) => {
                        whitespace = ws;
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        // (3) consume Text
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::Text(range) => {
                        text = &self.source_code[range];
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        // (4) consume EndRaw
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::EndRaw(_) => {
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        // Ok(tree::DocumentElement::Text(text.to_owned()))  // NOTE would not convey `whitespace`
        let mut h = HashMap::new();
        h.insert("=whitespace".to_owned(), vec![ tree::DocumentElement::Text(whitespace.to_string()) ]);
        Ok(tree::DocumentElement::Function(tree::DocumentFunction {
            name: name.to_string(),
            args: h,
            content: vec![tree::DocumentElement::Text(text.to_owned())],
        }))
    }

    fn parse_content(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<tree::DocumentNode> {
        let mut content = tree::DocumentNode::new();

        // (1) consume BeginContent
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::BeginContent(_) => {
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
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
                BeginRaw,
                Text,
                EndContent,
                Unexpected,
            }

            let mut next_token = NextToken::Unexpected;

            if let Some(token_or_err) = iter.peek() {
                next_token = match token_or_err {
                    Ok(lexer::Token::BeginFunction(_)) => NextToken::BeginFunction,
                    Ok(lexer::Token::BeginRaw(_)) => NextToken::BeginRaw,
                    Ok(lexer::Token::Text(_)) => NextToken::Text,
                    Ok(lexer::Token::EndContent(_)) => NextToken::EndContent,
                    _ => NextToken::Unexpected,
                };
            }

            match next_token {
                NextToken::BeginFunction => {
                    // (3)   if BeginFunction
                    // (4)     parse_function
                    let func = self.parse_function(iter)?;
                    content.push(func);
                },
                NextToken::BeginRaw => {
                    let text = self.parse_raw(iter)?;
                    content.push(text);
                },
                NextToken::Text => {
                    // (7)   if Text
                    // (8)     add text
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
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        Ok(content)
    }

    fn parse_argument_value(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<tree::DocumentNode> {
        let mut arg_value = tree::DocumentNode::new();

        // (1) consume BeginArgValue
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::BeginArgValue(_) => {
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
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
                BeginRaw,
                Text,
                EndArgValue,
                Unexpected,
            }

            let mut next_token = NextToken::Unexpected;

            if let Some(token_or_err) = iter.peek() {
                next_token = match token_or_err {
                    Ok(lexer::Token::BeginFunction(_)) => NextToken::BeginFunction,
                    Ok(lexer::Token::BeginRaw(_)) => NextToken::BeginRaw,
                    Ok(lexer::Token::Text(_)) => NextToken::Text,
                    Ok(lexer::Token::EndArgValue(_)) => NextToken::EndArgValue,
                    _ => NextToken::Unexpected,
                };
            }

            match next_token {
                NextToken::BeginFunction => {
                    // (3)   if BeginFunction
                    // (4)     parse_function
                    let func = self.parse_function(iter)?;
                    arg_value.push(func);
                },
                NextToken::BeginRaw => {
                    let text = self.parse_raw(iter)?;
                    arg_value.push(text);
                },
                NextToken::Text => {
                    // (7)   if Text
                    // (8)     add text
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
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        Ok(arg_value)
    }

    fn parse_function(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<tree::DocumentElement> {
        let mut func = tree::DocumentFunction::new();

        // (01) consume BeginFunction
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::BeginFunction(_) => {
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
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
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        // (03) optionally consume Whitespace
        if let Some(Ok(lexer::Token::Whitespace(_, _))) = iter.peek() {
            match iter.next() {
                Some(tok_or_err) => {
                    let token = tok_or_err?;
                    match token {
                        lexer::Token::Whitespace(_, whitespace) => {
                            func.args.insert("=whitespace".to_owned(), vec![tree::DocumentElement::Text(format!("{whitespace}"))]);
                        },
                        lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                        _ => return Self::unexpected_token(&token),
                    }
                },
                None => return Self::unexpected_eof(),
            }
        }

        // (04) if BeginArgs
        if let Some(Ok(lexer::Token::BeginArgs)) = iter.peek() {
            // (05)   consume BeginArgs
            match iter.next() {
                Some(tok_or_err) => {
                    let token = tok_or_err?;
                    match token {
                        lexer::Token::BeginArgs => {
                            // NOTE: expected token, yay!
                        },
                        lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                        _ => return Self::unexpected_token(&token),
                    }
                },
                None => return Self::unexpected_eof(),
            }

            // (06)   loop if ArgKey
            while let Some(Ok(lexer::Token::ArgKey(_))) = iter.peek() {
                // NOTE: ok, we consume an argument key-value pair

                // (07)     consume ArgKey
                let arg_name = match iter.next() {
                    Some(token_or_err) => {
                        let token = token_or_err?;
                        match token {
                            lexer::Token::EndArgs => {
                                // NOTE: end of arguments? Ok.
                                break;
                            },
                            lexer::Token::ArgKey(range) => {
                                &self.source_code[range]
                            }
                            lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                            _ => return Self::unexpected_token(&token),
                        }
                    },
                    None => return Self::unexpected_eof(),
                }.to_owned();

                // (08)     parse_argument_value
                let arg_value = self.parse_argument_value(iter)?;
                func.args.insert(arg_name, arg_value);
            }

            // (09)   consume EndArgs
            match iter.next() {
                Some(tok_or_err) => {
                    let token = tok_or_err?;
                    match token {
                        lexer::Token::EndArgs => {
                            // NOTE: expected token, yay!
                        },
                        lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                        _ => return Self::unexpected_token(&token),
                    }
                },
                None => return Self::unexpected_eof(),
            }

            // (10)   optionally consume Whitespace
            if let Some(Ok(lexer::Token::Whitespace(_, _))) = iter.peek() {
                match iter.next() {
                    Some(tok_or_err) => {
                        let token = tok_or_err?;
                        match token {
                            lexer::Token::Whitespace(_, whitespace) => {
                                func.args.insert("=whitespace".to_owned(), vec![tree::DocumentElement::Text(format!("{whitespace}"))]);
                            },
                            lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                            _ => return Self::unexpected_token(&token),
                        }
                    },
                    None => return Self::unexpected_eof(),
                }
            }
        }

        // (11) if BeginContent
        let mut found_content = false;
        if let Some(Ok(lexer::Token::BeginContent(_))) = iter.peek() {
            found_content = true;
        }

        if found_content {
            // (12)   parse_content
            func.content = self.parse_content(iter)?;
        }

        // (13) consume EndFunction
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::EndFunction(_) => {
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token),
                }
            },
            None => return Self::unexpected_eof(),
        }

        Ok(tree::DocumentElement::Function(func))
    }

    /// Consumes the tokens provided by the `LexingIterator` argument
    pub fn consume_iter(&mut self, iter: lexer::LexingIterator) -> anyhow::Result<()> {
        let mut peekable_iter = iter.peekable();

        // admissible tokens
        enum NextToken {
            BeginFunction,
            BeginRaw,
            Text,
            EndOfFile,
            Unexpected,
        }

        loop {
            let mut next_token = NextToken::Unexpected;

            if let Some(token_or_err) = peekable_iter.peek() {
                next_token = match token_or_err {
                    Ok(lexer::Token::BeginFunction(_)) => NextToken::BeginFunction,
                    Ok(lexer::Token::BeginRaw(_)) => NextToken::BeginRaw,
                    Ok(lexer::Token::Text(_)) => NextToken::Text,
                    Ok(lexer::Token::EndOfFile(_)) => NextToken::EndOfFile,
                    _ => NextToken::Unexpected,
                }
            }

            match next_token {
                NextToken::BeginFunction => {
                    let func = self.parse_function(&mut peekable_iter)?;
                    self.tree.push(func);
                },
                NextToken::BeginRaw => {
                    let text = self.parse_raw(&mut peekable_iter)?;
                    self.tree.push(text);
                },
                NextToken::Text => {
                    if let Some(Ok(lexer::Token::Text(range))) = peekable_iter.next() {
                        let text = &self.source_code[range];
                        self.tree.push(tree::DocumentElement::Text(text.to_owned()));
                    }
                },
                NextToken::EndOfFile => {
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

    /// Declares the end of the text document
    pub fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Returns the Abstract Syntax Tree to be processed further
    pub fn tree(self) -> tree::DocumentTree {
        let mut args = HashMap::new();
        if let Some(fp) = self.filepath.to_str() {
            args.insert("filepath".to_owned(), vec![tree::DocumentElement::Text(fp.to_owned())]);
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
        println!("{name}");
        if indent_change >= 0 { (*indent) += indent_change; }
    }

    pub(crate) fn show_pos(name: &str, pos: usize, indent: &mut i32, indent_change: i32, src: &str) {
        if indent_change < 0 { (*indent) += indent_change; }
        print!("{}", "  ".repeat(*indent as usize));
        let content: char = match &src[pos..].chars().next() {
            Some(c) => *c,
            None => panic!("invalid UTF-8 offset position {pos} in token {name} received"),
        };
        println!("{name}({content})");
        if indent_change >= 0 { (*indent) += indent_change; }
    }

    pub(crate) fn show_range(name: &str, range: ops::Range<usize>, indent: &mut i32, indent_change: i32, src: &str) {
        if indent_change < 0 { (*indent) += indent_change; }
        print!("{}", "  ".repeat(*indent as usize));
        let content: &str = &src[range];
        println!("{name}({content})");
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
                        lexer::Token::BeginRaw(range) => Self::show_range("BeginRaw", range, &mut indent, 1, self.source_code),
                        lexer::Token::EndRaw(range) => Self::show_range("EndRaw", range, &mut indent, -1, self.source_code),
                        lexer::Token::Whitespace(pos, ws) => Self::show_pos(&format!("Whitespace({ws})"), pos, &mut indent, 0, self.source_code),
                        lexer::Token::Text(range) => Self::show_range("Text", range, &mut indent, 0, self.source_code),
                        lexer::Token::EndOfFile(_) => Self::show_token("EOF", &mut indent, 0),
                    }
                },
                Err(e) => { eprintln!("{e:?}"); break },
            }
        }
    }
}
