use anyhow;

mod lexer;
mod parser;
mod tree;

use mlua::prelude::*;

use std::collections::HashMap;
use std::fs;
use std::ops;
use std::ffi::OsString;
use std::io::prelude::*;
use std::path;
use std::str;

const FILE_PREDEBUG_HOOKS: &str = "hooks.pre-debug.lua";
const FILE_TOSTRING_HOOKS: &str = "hooks.node-to-string.lua";
const FILE_MODIFY_HOOKS: &str = "hooks.modify-node.lua";
const FILE_POSTDEBUG_HOOKS: &str = "hooks.post-debug.lua";

fn run_lua(doc: &tree::DocumentTree) -> anyhow::Result<()> {
    let mut lua = Lua::new();

    // (1) load litua library
    let litua_lib = include_str!("litua.lua");
    lua.load(litua_lib).exec()?;

    // (2) run script FILE_PREDEBUG_HOOKS
    {
        let filepath = std::path::Path::new(FILE_PREDEBUG_HOOKS);
        if filepath.exists() {
            let string_hooks_src = fs::read_to_string(FILE_PREDEBUG_HOOKS)
                .expect("Should have been able to read the file");
            lua.load(&string_hooks_src).exec()?;
        }
    }

    // (3) run script FILE_TOSTRING_HOOKS
    {
        let filepath = std::path::Path::new(FILE_TOSTRING_HOOKS);
        if filepath.exists() {
            let string_hooks_src = fs::read_to_string(FILE_TOSTRING_HOOKS)
                .expect("Should have been able to read the file");
            lua.load(&string_hooks_src).exec()?;
        }
    }

    // (4) run script FILE_MODIFY_HOOKS
    {
        let filepath = std::path::Path::new(FILE_MODIFY_HOOKS);
        if filepath.exists() {
            let string_hooks_src = fs::read_to_string(FILE_MODIFY_HOOKS)
                .expect("Should have been able to read the file");
            lua.load(&string_hooks_src).exec()?;
        }
    }

    // (5) run script FILE_POSTDEBUG_HOOKS
    {
        let filepath = std::path::Path::new(FILE_POSTDEBUG_HOOKS);
        if filepath.exists() {
            let string_hooks_src = fs::read_to_string(FILE_POSTDEBUG_HOOKS)
                .expect("Should have been able to read the file");
            lua.load(&string_hooks_src).exec()?;
        }
    }

    // (6) load tree to lua environment
    let tree = doc.to_lua(&lua)?;

    // (7) load transform function
    let litua_lib = include_str!("litua_transform.lua");
    lua.load(litua_lib).exec()?;

    // (8) call transformation
    let globals = lua.globals();
    let global_litua: mlua::Table = globals.get("Litua")?;
    let transform: mlua::Function = global_litua.get("transform")?;
    let lua_result = transform.call::<mlua::Value, mlua::String>(tree)?;
    let result = lua_result.to_str()?;

    // (9) print the result
    println!("final string result <<{}>>", result);

    Ok(())
}

fn handle_file(filepath: OsString) -> anyhow::Result<()> {
    let mut fd = fs::File::open(filepath.clone())?;
    let mut buf = Vec::new();
    fd.read_to_end(&mut buf)?;

    let source_code = str::from_utf8(&buf)?;
    let l = lexer::Lexer::new(source_code);

    let mut p = parser::Parser::new(filepath, source_code);
    p.consume_iter(l.iter())?;
    p.finalize()?;

    let tree = p.tree();
    println!("{:?}", tree);
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
