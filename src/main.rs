use anyhow;

mod lexer;
mod parser;
mod tree;

use mlua::prelude::*;
use clap::Parser;

use std::ffi::OsString;
use std::fs;
use std::io::prelude::*;
use std::path;
use std::str;

fn run_lua<A: AsRef<path::Path>, B: AsRef<path::Path>, C: AsRef<path::Path>>(dst: A, doc: &tree::DocumentTree, hooks_dir: B, luapath_additions: C) -> anyhow::Result<()> {
    let lua = Lua::new();

    let addition_str = path::PathBuf::from(luapath_additions.as_ref());
    //: Result<&str, anyhow::Error> = luapath_additions.try_into();
    match addition_str.to_str() {
        Some(s) => lua.load(&format!("package.path = package.path .. ';{}'", s)).exec()?,
        None => return Err(anyhow::anyhow!("cannot convert the luapath extension path (supplied as --add-require-path) to a UTF-8 string. But this is sadly required by the mlua interface (the library to run Lua)")),
    };

    // (1) load litua libraries
    let litua_table = include_str!("litua.lua");
    lua.load(litua_table).exec()?;
    let litua_lib = include_str!("litua.lib.lua");
    lua.load(litua_lib).exec()?;

    // TODO luapath_additions

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
    fs::write(dst, result)?;

    Ok(())
}

fn derive_destination_filepath(p: &path::Path) -> path::PathBuf {
    if let Some(ext) = p.extension() {
        if ext == OsString::from("lit") {
            p.with_extension("out")
        } else {
            p.with_extension("lit")
        }
    } else {
        path::PathBuf::from("doc.out")
    }
}

fn lex_and_parse(src: &path::Path) -> anyhow::Result<tree::DocumentTree> {
    let mut fd = fs::File::open(src.clone())?;
    let mut buf = Vec::new();
    fd.read_to_end(&mut buf)?;

    let source_code = str::from_utf8(&buf)?;
    let l = lexer::Lexer::new(source_code);

    let mut p = parser::Parser::new(src, source_code);
    p.consume_iter(l.iter())?;
    p.finalize()?;

    Ok(p.tree())
}

#[derive(Parser, Debug)]
#[command(name = "litua")]
#[command(author = "meisterluk <kbknapp@gmail.com>")]
#[command(version = "0.5")]
#[command(about = "Read document as tree and apply Lua functions to nodes")]
#[command(author, version, about, long_about = None)]
struct Settings {
    // helpful for debugging 
    #[arg(long)]
    dump_lexed: bool,
    #[arg(long)]
    dump_parsed: bool,
    #[arg(long)]
    dump_calls: bool,

    // configuration
    #[arg(long, value_name = "DIR")]
    hooks_dir: Option<path::PathBuf>,
    #[arg(long, value_name = "DIR")]
    add_require_path: Option<path::PathBuf>,

    // optional argument
    #[arg(short = 'o', long, value_name = "PATH")]
    destination: Option<path::PathBuf>,

    // positional argument
    source: path::PathBuf,
}


fn main() -> anyhow::Result<()> {
    let set = Settings::parse();

    let src = set.source;
    let dst = match set.destination {
        Some(p) => p,
        None => derive_destination_filepath(src.as_ref()),
    };

    if set.dump_lexed {
        // TODO
    } else if set.dump_parsed {
        // TODO
    } else if set.dump_calls {
        println!("{:?}", set.hooks_dir);
        println!("{:?}", set.add_require_path);
        // TODO
    } else {
        let hooks_dir = set.hooks_dir.unwrap_or(path::PathBuf::from("."));
        let lua_path_additions = set.add_require_path.unwrap_or(path::PathBuf::from(""));

        let doctree = lex_and_parse(&src)?;
        run_lua(&dst, &doctree, hooks_dir, lua_path_additions)?;

        println!("File '{}' read. File '{}' written.", src.display(), dst.display());
    }

    Ok(())
}
