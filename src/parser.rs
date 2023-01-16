use anyhow;

use std::collections::HashMap;
use std::iter;
use std::ffi::OsString;

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

    pub fn parse_function(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<(usize, tree::DocumentElement)> {
        let mut func = tree::DocumentFunction::new();
        let (start_pos, end_pos) = (0, 0);

        match iter.next() {
            Some(lexer::Token::BeginFunction(start)) => start_pos = start,
            tok => return Err(anyhow::anyhow!("unexpected token '{:?}' while parsing start of a function", tok)),
        }

        match iter.next() {
            Some(lexer::Token::Call(range)) => func.name = String::from(&self.source_code[range.start..range.end]),
            Some(tok) => return Err(anyhow::anyhow!("unexpected token '{:?}' while parsing function", tok)),
            None => return Err(anyhow::anyhow!("expected token '{:?}' while parsing function", tok)),
        }

        match iter.peek() {
            Some(lexer::Token::BeginArgs) => {
                let (pos_end, args) = self.parse_args(iter)?;
                func.args = args;
            },
            Some(lexer::Token::BeginContent(_)) => {
                let (pos, c) = self.parse_content(iter)?;
                func.content = c;
            },
            _ => {
                // TODO protocol violated
            }
        }

        match iter.next() {
            Some(lexer::Token::EndFunction(_start)) => {
                // everything ok. TODO Do something with _start?
            },
            _ => {
                // TODO protocol violated
            },
        }

        Ok((pos, tree::DocumentElement::Function(func)))
    }

    pub fn parse_args(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<(usize, HashMap<String, tree::DocumentNode>)> {
        let mut args = HashMap::<String, tree::DocumentNode>::new();

        match iter.next() {
            Some(lexer::Token::BeginArgs) => {},
            _ => {
                // TODO protocol violated
            },
        }

        loop {
            let mut name = String::new();
            let mut consume_one_item = false;

            match iter.peek() {
                // TODO update end_of_previous_token with parse_function. Then end_of_previous_token.._start is text to add
                Some(lexer::Token::ArgKey(range)) => {
                    name = String::from(&self.source_code[range.start..range.end]);
                    consume_one_item = true;
                },
                Some(lexer::Token::BeginArgValue(_)) => {
                    let content = self.parse_arg_value(iter)?;
                    args.insert(name, content);
                },
                Some(lexer::Token::EndArgs) => break,
                _ => {
                    // TODO protocol violated
                }
            }

            if consume_one_item {
                iter.next();
            }
        }

        match iter.next() {
            Some(lexer::Token::EndArgs) => {},
            _ => {
                // TODO protocol violated
            },
        }

        Ok(args)
    }

    pub fn parse_arg_value(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<(usize, tree::DocumentNode)> {
        let mut node = tree::DocumentNode::new();
        let mut end_of_previous_token = 0;

        match iter.next() {
            Some(lexer::Token::BeginArgValue(this_start)) => end_of_previous_token = this_start,
            _ => {
                // TODO protocol violated
            },
        }
        
        loop {
            match iter.peek() {
                // TODO update end_of_previous_token with parse_function. Then end_of_previous_token.._start is text to add
                Some(lexer::Token::BeginFunction(_start)) => node.push(self.parse_function(iter)?),
                Some(lexer::Token::EndArgValue(_)) => break,
                _ => {
                    // TODO protocol violated
                }
            }
        }

        match iter.next() {
            Some(lexer::Token::EndArgValue(_)) => {},
            _ => {
                // TODO protocol violated
            },
        }

        Ok(node)
    }

    pub fn parse_content(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<(usize, tree::DocumentNode)> {
        let mut node = tree::DocumentNode::new();
        let mut end_of_previous_token = 0;

        match iter.next() {
            Some(token_or_err) => {
                if let Ok(lexer::Token::BeginFunction(pos)) = token_or_err {
                    end_of_previous_token = pos;
                } else {
                    return Err(anyhow::anyhow!("unexpected token {:?} while starting to parse content", token_or_err));
                }
            },
            None => return Err(anyhow::anyhow!("unexpected end of tokens while starting to parse content")),
        }
        
        loop {
            match iter.peek() {
                // TODO update end_of_previous_token with parse_function. Then end_of_previous_token.._start is text to add
                Some(Ok(lexer::Token::BeginFunction(_start))) => node.push(self.parse_function(iter)?),
                Some(Ok(lexer::Token::EndContent(_))) => break,
                None => return Err(anyhow::anyhow!("unexpected end of tokens while starting to parse content")),
            }
        }

        iter.next();

        Ok(node)
    }

    pub fn consume_iter(&mut self, iter: lexer::LexingIterator) -> anyhow::Result<()> {
        let mut peekable_iter = iter.peekable();
        let mut pos_latest = 0;

        loop {
            let mut found_function = false;
            let errmsg = anyhow::anyhow!("unexpected end of tokens at byte offset {}", pos_latest);
            /*
            let next = (*peekable_iter.peek().ok_or(errmsg)?)?;
            if let lexer::Token::BeginFunction(_) = next {
                found_function = true;
            } else if let lexer::Token::EOF = next {
                break;
            } else {}
            */

            match peekable_iter.peek() {
                Some(token_or_err) => {
                    if let Ok(lexer::Token::BeginFunction(_)) = token_or_err {
                        found_function = true;
                    } else if let Ok(lexer::Token::EOF) = token_or_err {
                        break;
                    }
                },
                _ => return Err(anyhow::anyhow!(errmsg)),
            }

            if found_function {
                let (pos_end, func) = self.parse_function(&mut peekable_iter)?;
                pos_latest = pos_end;
                self.tree.push(func);
            }
        }

        Ok(())
    }

    pub fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn tree(&mut self) -> tree::DocumentTree {
        let mut args = HashMap::new();
        if !self.filepath.is_empty() {
            if let Some(fp) = self.filepath.to_str() {
                args.insert("filepath".to_owned(), vec![tree::DocumentElement::Text(fp.to_owned())]);
            }
        }

        let elem = tree::DocumentElement::Function(tree::DocumentFunction {
            name: "document".to_owned(),
            args,
            content: tree::DocumentNode::new(),
        });
        tree::DocumentTree(elem)
    }
}