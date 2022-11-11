use anyhow;

mod lexer;
mod parser;
mod tree;

use std::collections::HashMap;
use std::fs;
use std::ffi::OsString;
use std::io::prelude::*;
use std::path;
use std::str;

fn run_lua(doc: &tree::DocumentTree) -> anyhow::Result<()> {
    Ok(())
}

fn handle_file(filepath: OsString) -> anyhow::Result<()> {
    let mut fd = fs::File::open(filepath.clone())?;
    let mut buf = Vec::new();
    fd.read_to_end(&mut buf)?;

    let source_code = str::from_utf8(&buf)?;
    let mut l = lexer::Lexer::new(source_code);
    let mut p = parser::Parser::new(filepath, source_code);
    /*for token in l.iter() {
        match token {
            Ok(tok) => {
                println!("{:?}", tok.format_with_src(str::from_utf8(buf.as_ref())?));
                p.feed(tok)?;
            },
            Err(e) => {
                println!("{:?}", e);
                return Err(e);
            }
        }
    }*/
    let mut err = None;
    p.consume_iter(l.iter().take_while(|elem| {
        match elem {
            Err(e) => { err = Some(*e); false },
            Ok(_) => true,
        }
    }).map(|elem| elem?));
    p.finalize()?;

    let tree = p.tree();
    run_lua(&tree)?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    for filepath in std::env::args_os().skip(1) {
        handle_file(filepath.clone())?;

        match filepath.to_str() {
            Some(s) => println!("File '{}' handled.", s),
            None => println!("File handled."),
        }
    }

    Ok(())
}
