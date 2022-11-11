use anyhow;

use std::collections::HashMap;
use std::iter;
use std::mem;
use std::path;
use std::ffi::OsString;

use crate::tree;
use crate::lexer;

pub struct Parser<'s> {
    filepath: OsString,
    source_code: &'s str,
    tree: tree::DocumentTree,
    path: Vec<usize>,
}

impl<'s> Parser<'s> {
    pub fn new(filepath: OsString, source_code: &'s str) -> Parser<'s> {
        Parser{
            filepath: filepath.clone(),
            source_code,
            tree: tree::DocumentTree::from_filepath(filepath.to_owned()),
            path: Vec::new(),
        }
    }

    /*fn current_element(&mut self) -> &mut tree::DocumentNode {
        let mut current: &mut tree::DocumentNode = match &mut self.tree.0 {
            tree::DocumentElement::Function(tree::DocumentFunction {content, ..}) => content,
            tree::DocumentElement::Text(_) => panic!("the document tree was initialized incorrectly"),
        };
        for index in self.path.iter() {
            let element: &mut tree::DocumentElement = &mut current[*index];
            mem::replace(current, match element {
                tree::DocumentElement::Function(tree::DocumentFunction { content, .. }) => content,
                tree::DocumentElement::Text(_) => break,
            });
        }

        current
    }

    pub fn feed(&mut self, tok: lexer::Token) -> anyhow::Result<()> {
        use lexer::Token::*;

        
        self.current_element().push(tree::DocumentElement::Text("hello".to_owned()));

        match tok {
            BeginFunction(_) => {
                let func = tree::DocumentElement::empty_function();
                //cur.push(func);
                // TODO put empty function into tree
            },
            Call(range) => {
                
            },
            BeginArgs => todo!(),
            ArgKey(range) => todo!(),
            BeginArgValue(_) => todo!(),
            EndArgValue(_) => todo!(),
            EndArgs => todo!(),
            BeginContent(_) => todo!(),
            EndContent(_) => todo!(),
            EndFunction(_) => todo!(),
            EOF => todo!(),
            Error(_) => todo!(),
        }

        Ok(())
    }*/

    pub fn parse_function(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<tree::DocumentElement> {
        let mut func = tree::DocumentFunction::new();

        if let Some(tok) = iter.next() {
            match tok? {
                lexer::Token::BeginFunction(_start) => {
                    // everything ok. TODO Do something with _start?
                },
                _ => {
                    // TODO protocol violated
                },
            }
        }

        if let Some(tok) = iter.next() {
            match tok? {
                lexer::Token::Call(range) => func.name = String::from(&self.source_code[range.start..range.end]),
                _ => {
                    // TODO protocol violated
                },
            }
        }

        if let Some(tok) = iter.peek() {
            match tok {
                Ok(lexer::Token::BeginArgs) => {
                    func.args = self.parse_args(iter)?;
                },
                Ok(lexer::Token::BeginContent(_)) => {
                    func.content = self.parse_content(iter)?;
                },
                Err(err) => return Err(*(err.clone())),
                _ => {
                    // TODO protocol violated
                }
            }
        }

        if let Some(tok) = iter.next() {
            match tok? {
                lexer::Token::EndFunction(_start) => {
                    // everything ok. TODO Do something with _start?
                },
                _ => {
                    // TODO protocol violated
                },
            }
        }

        Ok(tree::DocumentElement::Function(func))
    }

    pub fn parse_args(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<HashMap<String, tree::DocumentNode>> {
        let mut args = HashMap::<String, tree::DocumentNode>::new();

        if let Some(tok) = iter.next() {
            match tok? {
                lexer::Token::BeginArgs => {},
                _ => {
                    // TODO protocol violated
                },
            }
        }

        loop {
            let name = String::new();

            if let Some(tok) = iter.peek() {
                match (*tok)? {
                    // TODO update end_of_previous_token with parse_function. Then end_of_previous_token.._start is text to add
                    lexer::Token::ArgKey(range) => {
                        iter.next();
                        name = String::from(&self.source_code[range.start..range.end]);
                    },
                    lexer::Token::BeginArgValue(_) => {
                        let content = self.parse_arg_value(iter)?;
                        args.insert(name, content);
                    },
                    lexer::Token::EndArgs => break,
                    _ => {
                        // TODO protocol violated
                    }
                }
            };
        }

        iter.next();

        Ok(args)
    }

    pub fn parse_arg_value(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<tree::DocumentNode> {
        let mut node = tree::DocumentNode::new();
        let mut end_of_previous_token = 0;

        if let Some(tok) = iter.next() {
            match tok? {
                lexer::Token::BeginArgValue(this_start) => end_of_previous_token = this_start,
                _ => {
                    // TODO protocol violated
                },
            }
        }
        
        loop {
            if let Some(tok) = iter.peek() {
                match (*tok)? {
                    // TODO update end_of_previous_token with parse_function. Then end_of_previous_token.._start is text to add
                    lexer::Token::BeginFunction(_start) => node.push(self.parse_function(iter)?),
                    lexer::Token::EndArgValue(_) => break,
                    _ => {
                        // TODO protocol violated
                    }
                }
            };
        }

        iter.next();

        Ok(node)
    }

    pub fn parse_content(&mut self, iter: &mut iter::Peekable<lexer::LexingIterator>) -> anyhow::Result<tree::DocumentNode> {
        let mut node = tree::DocumentNode::new();
        let mut end_of_previous_token = 0;

        if let Some(tok) = iter.next() {
            match tok? {
                lexer::Token::BeginContent(this_start) => end_of_previous_token = this_start,
                _ => {
                    // TODO protocol violated
                },
            }
        }
        
        loop {
            if let Some(tok) = iter.peek() {
                match (*tok)? {
                    // TODO update end_of_previous_token with parse_function. Then end_of_previous_token.._start is text to add
                    lexer::Token::BeginFunction(_start) => node.push(self.parse_function(iter)?),
                    lexer::Token::EndContent(_) => break,
                    _ => {
                        // TODO protocol violated
                    }
                }
            };
        }

        iter.next();

        Ok(node)
    }

    pub fn consume_iter(&mut self, iter: lexer::LexingIterator) -> anyhow::Result<()> {
        let mut peekable_iter = iter.peekable();
        let mut end_of_previous_token = 0;

        let read_as_text = |start| {
            if start != end_of_previous_token {
                match self.tree.0 {
                    tree::DocumentElement::Function(tree::DocumentFunction{ content: c, .. }) => {
                        c.push(tree::DocumentElement::Text(String::from(&self.source_code[start..end_of_previous_token])));
                    },
                    _ => {},
                }
            }
            end_of_previous_token = start;
        };

        loop {
            match peekable_iter.peek() {
                Some(Ok(lexer::Token::BeginFunction(start))) => {
                    read_as_text(*start);

                    let func = self.parse_function(&mut peekable_iter)?;
                    match &mut self.tree.0 {
                        tree::DocumentElement::Function(tree::DocumentFunction{ content: c, .. }) => c.push(func),
                        _ => {},
                    }
                },
                Some(Ok(lexer::Token::EOF)) => break,
                Some(Err(_)) => {},
                None => {},
                _ => {},
            };
        }

        Ok(())
    }

    pub fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn tree(&mut self) -> tree::DocumentTree {
        mem::replace(&mut self.tree, tree::DocumentTree::new())
    }
}