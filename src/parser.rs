//! Parser for litua text documents

use std::collections::HashMap;
use std::iter;
use std::path;

use crate::tree;
use crate::lexer;
use crate::errors;

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
    pub root: tree::DocumentFunction,
}

impl<'s> Parser<'s> {
    pub fn new(filepath: &path::Path, source_code: &'s str) -> Parser<'s> {
        let mut args = HashMap::new();
        if let Some(fp) = filepath.to_str() {
            args.insert("filepath".to_owned(), vec![tree::DocumentElement::Text(fp.to_owned())]);
        }

        let root = tree::DocumentFunction {
            call: "document".to_owned(),
            args,
            content: vec!(),
        };

        Parser{
            filepath: filepath.to_owned(),
            source_code,
            root,
        }
    }

    #[inline]
    fn unexpected_token<T>(tok: &lexer::Token, expected: &str) -> Result<T, errors::Error> {
        Err(errors::Error::UnexpectedToken(tok.clone(), expected.to_owned()))
    }

    #[inline]
    fn unexpected_eof<T>() -> Result<T, errors::Error> {
        Err(errors::Error::UnexpectedEOF("unexpected end of lexer tokens iterator".to_owned()))
    }

    fn parse_raw(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> Result<tree::DocumentElement, errors::Error> {
        let whitespace_before;
        let whitespace_after;
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
                    _ => return Self::unexpected_token(&token, "start of raw string"),
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
                        whitespace_before = ws;
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token, "whitespace before"),
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
                    _ => return Self::unexpected_token(&token, "text string"),
                }
            },
            None => return Self::unexpected_eof(),
        }


        // (4) consume Whitespace
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::Whitespace(_, ws) => {
                        whitespace_after = ws;
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token, "whitespace after raw string"),
                }
            },
            None => return Self::unexpected_eof(),
        }

        // (5) consume EndRaw
        match iter.next() {
            Some(tok_or_err) => {
                let token = tok_or_err?;
                match token {
                    lexer::Token::EndRaw(_) => {
                        // NOTE: expected token, yay!
                    },
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token, "end of raw string"),
                }
            },
            None => return Self::unexpected_eof(),
        }

        // Ok(tree::DocumentElement::Text(text.to_owned()))  // NOTE would not convey `whitespace`
        let mut h = HashMap::new();
        h.insert("=whitespace".to_owned(), vec![ tree::DocumentElement::Text(whitespace_before.to_string()) ]);
        h.insert("=whitespace-after".to_owned(), vec![ tree::DocumentElement::Text(whitespace_after.to_string()) ]);
        Ok(tree::DocumentElement::Function(tree::DocumentFunction {
            call: name.to_string(),
            args: h,
            content: vec![tree::DocumentElement::Text(text.to_owned())],
        }))
    }

    fn parse_content(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> Result<tree::DocumentNode, errors::Error> {
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
                    _ => return Self::unexpected_token(&token, "start of content"),
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
                        Some(Ok(tok)) => return Self::unexpected_token(&tok, "start of function/raw string or some text or end of content"),
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
                    _ => return Self::unexpected_token(&token, "end of content"),
                }
            },
            None => return Self::unexpected_eof(),
        }

        Ok(content)
    }

    fn parse_argument_value(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> Result<tree::DocumentNode, errors::Error> {
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
                    _ => return Self::unexpected_token(&token, "start of argument value"),
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
                        Some(Ok(tok)) => return Self::unexpected_token(&tok, "start of function/raw string or some text or end of argument value"),
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
                    _ => return Self::unexpected_token(&token, "end of argument value"),
                }
            },
            None => return Self::unexpected_eof(),
        }

        Ok(arg_value)
    }

    fn parse_function(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> Result<tree::DocumentElement, errors::Error> {
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
                    _ => return Self::unexpected_token(&token, "start of function"),
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
                        func.call = name.to_owned();
                    },
                    lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                    _ => return Self::unexpected_token(&token, "call name"),
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
                        _ => return Self::unexpected_token(&token, "whitespace"),
                    }
                },
                None => return Self::unexpected_eof(),
            }
        }

        // (04) if BeginArgs
        if let Some(Ok(lexer::Token::BeginArgs(_))) = iter.peek() {
            // (05)   consume BeginArgs
            match iter.next() {
                Some(tok_or_err) => {
                    let token = tok_or_err?;
                    match token {
                        lexer::Token::BeginArgs(_) => {
                            // NOTE: expected token, yay!
                        },
                        lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                        _ => return Self::unexpected_token(&token, "start of arguments"),
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
                            lexer::Token::EndArgs(_) => {
                                // NOTE: end of arguments? Ok.
                                break;
                            },
                            lexer::Token::ArgKey(range) => {
                                &self.source_code[range]
                            }
                            lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                            _ => return Self::unexpected_token(&token, "end of arguments or the next argument key"),
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
                        lexer::Token::EndArgs(_) => {
                            // NOTE: expected token, yay!
                        },
                        lexer::Token::EndOfFile(_) => return Self::unexpected_eof(),
                        _ => return Self::unexpected_token(&token, "end of arguments"),
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
                            _ => return Self::unexpected_token(&token, "some whitespace"),
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
                    _ => return Self::unexpected_token(&token, "end of function"),
                }
            },
            None => return Self::unexpected_eof(),
        }

        Ok(tree::DocumentElement::Function(func))
    }

    /// Consumes the tokens provided by the `LexingIterator` argument
    pub fn consume_iter(&mut self, iter: lexer::LexingIterator) -> Result<(), errors::Error> {
        let mut peekable_iter = iter.peekable();

        // admissible tokens
        enum NextToken {
            BeginFunction,
            BeginContent,
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
                    Ok(lexer::Token::BeginContent(_)) => NextToken::BeginContent,
                    Ok(lexer::Token::BeginRaw(_)) => NextToken::BeginRaw,
                    Ok(lexer::Token::Text(_)) => NextToken::Text,
                    Ok(lexer::Token::EndOfFile(_)) => NextToken::EndOfFile,
                    _ => NextToken::Unexpected,
                }
            }

            match next_token {
                NextToken::BeginFunction => {
                    let func = self.parse_function(&mut peekable_iter)?;
                    self.root.content.push(func);
                },
                NextToken::BeginContent => {
                    let mut content = self.parse_content(&mut peekable_iter)?;
                    self.root.content.append(&mut content);
                },
                NextToken::BeginRaw => {
                    let text = self.parse_raw(&mut peekable_iter)?;
                    self.root.content.push(text);
                },
                NextToken::Text => {
                    if let Some(Ok(lexer::Token::Text(range))) = peekable_iter.next() {
                        let text = &self.source_code[range];
                        self.root.content.push(tree::DocumentElement::Text(text.to_owned()));
                    }
                },
                NextToken::EndOfFile => {
                    // Already done? How sad.
                    break;
                },
                NextToken::Unexpected => {
                    // protocol violation
                    match peekable_iter.next() {
                        Some(Ok(tok)) => return Self::unexpected_token(&tok, &format!("unexpected token {:?} while parsing document", peekable_iter.peek())),
                        Some(Err(err)) => Err(err)?,
                        None => return Self::unexpected_token(&lexer::Token::EndOfFile(0), "unexpected end of lexer tokens iterator"),
                    }
                },
            }
        }

        Ok(())
    }

    /// Declares the end of the text document
    pub fn finalize(&mut self) -> Result<(), errors::Error> {
        Ok(())
    }

    /// Returns the Abstract Syntax Tree to be processed further
    pub fn tree(self) -> tree::DocumentTree {
        tree::DocumentTree(tree::DocumentElement::Function(self.root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use path;

    #[test]
    fn lex_only_text() -> Result<(), errors::Error> {
        let input = "{e_lement[a_ttr=v_alue] c_ontent}";
        let lex = lexer::Lexer::new(input);
        let mut par = Parser::new(path::Path::new("example"), input);
        par.consume_iter(lex.iter())?;
        let tree = par.tree();

        match tree.0 {
            tree::DocumentElement::Function(doc) => {
                assert_eq!(doc.call, "document");
                assert_eq!(doc.args["filepath"], vec![tree::DocumentElement::Text("example".to_string())]);
                match &doc.content[0] {
                    tree::DocumentElement::Function(elem) => {
                        assert_eq!(elem.call, "e_lement");
                        assert_eq!(elem.args["a_ttr"], vec![tree::DocumentElement::Text("v_alue".to_string())]);
                        assert_eq!(elem.content, vec![tree::DocumentElement::Text("c_ontent".to_string())]);
                    },
                    _ => { assert!(false) },
                }
            },
            tree::DocumentElement::Text(_) => assert!(false),
        }

        Ok(())
    }
}
