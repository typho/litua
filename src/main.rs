use anyhow;

mod lexer;
mod parser;
mod tree;

use mlua::prelude::*;

use std::fs;
use std::ffi::OsString;
use std::io::prelude::*;
use std::path;
use std::str;

fn run_lua<P: AsRef<path::Path>>(doc: &tree::DocumentTree, hooks_dir: P, output_file: P) -> anyhow::Result<()> {
    let lua = Lua::new();

    // (1) load litua libraries
    let litua_table = include_str!("litua.lua");
    lua.load(litua_table).exec()?;
    let litua_filter = include_str!("litua.filter.lua");
    lua.load(litua_filter).exec()?;
    let litua_lib = include_str!("litua.lib.lua");
    lua.load(litua_lib).exec()?;

    // (2) find hook files
    let mut hook_files = vec![];
    for dir_entry in fs::read_dir(hooks_dir)? {
        let entry = dir_entry?;
        let basename = entry.file_name();
        if let Some(name) = basename.to_str() {
            if name.starts_with("hook") && name.ends_with(".lua") {
                hook_files.push(entry.path());
            }
        }
    }

    // (3) read hook files
    for hook_file in hook_files.iter() {
        let lua_file_src = fs::read_to_string(hook_file)?;
        lua.load(&lua_file_src).exec()?;
    }

    // (4) load tree to lua environment
    let tree = doc.to_lua(&lua)?;

    // (5) load transform function and node object (libraries, users cannot modify)
    let litua_trans = include_str!("litua.transform.lua");
    lua.load(litua_trans).exec()?;
    let litua_node = include_str!("litua.node.lua");
    lua.load(litua_node).exec()?;

    // (6) call transformation
    let globals = lua.globals();
    let global_litua: mlua::Table = globals.get("Litua")?;
    let transform: mlua::Function = global_litua.get("transform")?;
    let lua_result = transform.call::<mlua::Value, mlua::String>(tree)?;
    let result = lua_result.to_str()?;

    // (7) print the result
    fs::write(output_file, result)?;

    Ok(())
}

fn handle_file(filepath: OsString) -> anyhow::Result<String> {
    let output_filepath = match (&filepath).to_str() {
        Some(s) => {
            let spath = path::Path::new(s);
            let new_extension = if let Some(ext_osstr) = spath.extension() {
                if let Some(ext) = ext_osstr.to_str() {
                    if ext == "lit" { Some("out") } else { Some("lit") }
                } else { None }
            } else { None };

            match new_extension {
                Some(ext) => {
                    let mut p = String::new();
                    p.push_str(&s[0..s.rfind(".").unwrap()]);
                    p.push_str(".");
                    p.push_str(ext);
                    p
                },
                None =>  {
                    let mut p = s.to_owned();
                    p.push_str(".out");
                    p
                }
            }
        },
        None => "out.txt".to_owned(),
    };

    let mut fd = fs::File::open(filepath.clone())?;
    let mut buf = Vec::new();
    fd.read_to_end(&mut buf)?;

    let source_code = str::from_utf8(&buf)?;
    let l = lexer::Lexer::new(source_code);

    let mut p = parser::Parser::new(filepath, source_code);
    p.consume_iter(l.iter())?;
    p.finalize()?;

    let tree = p.tree();
    run_lua(&tree, ".", &output_filepath)?;

    Ok(output_filepath)
}

fn main() -> anyhow::Result<()> {
    // CLI proposal:
    //   • litua [-h|--help] [-v|--version] [--verbose] [--dump-lexed] [--dump-parsed] [--dump-calls] [--hooks-dir PATH] [--add-require-path PATH] TEXTFILE

    for filepath in std::env::args_os().skip(1) {
        let dst = handle_file(filepath.clone())?;

        match filepath.to_str() {
            Some(src) => println!("File '{}' handled ⇒ '{}' written.", src, dst),
            None => println!("File handled. File '{}' written.", dst),
        }
    }

    Ok(())
}
