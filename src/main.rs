mod lexer;
mod parser;
mod tree;
mod errors;

use mlua::prelude::*;
use clap::Parser;

use std::fs;
use std::io;
use std::io::prelude::*;
use std::path;
use std::str;

use std::error;
use std::fmt;

// Error type
#[derive(Debug)]
enum Error {
    CLIArg(String),
    Io(io::Error),
    Encoding(str::Utf8Error),
    Litua(errors::Error),
    Mlua(mlua::Error),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            CLIArg(msg) => write!(f, "{msg}"),
            Io(err) => write!(f, "{err:?}"),
            Encoding(err) => write!(f, "{err:?}"),
            Litua(err) => write!(f, "{err:?}"),
            Mlua(err) => write!(f, "{err:?}"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<str::Utf8Error> for Error {
    fn from(error: str::Utf8Error) -> Self {
        Self::Encoding(error)
    }
}

impl From<errors::Error> for Error {
    fn from(error: errors::Error) -> Self {
        Self::Litua(error)
    }
}

impl From<mlua::Error> for Error {
    fn from(error: mlua::Error) -> Self {
        Self::Mlua(error)
    }
}


fn run_lua<A: AsRef<path::Path>, B: AsRef<path::Path>, C: AsRef<path::Path>>(_conf: &Settings, dst: A, doc: &tree::DocumentTree, hooks_dir: B, luapath_additions: C) -> Result<(), Error> {
    // NOTE: 'debug' library is only available with Lua::unsafe_new()
    //       https://github.com/khvzak/mlua/issues/39
    let lua = unsafe { Lua::unsafe_new() };

    let addition_str = path::PathBuf::from(luapath_additions.as_ref());
    match addition_str.to_str() {
        Some(s) if !s.is_empty() => lua.load(&format!("package.path = package.path .. ';{s}'")).exec()?,
        Some(_) => {},
        None => return Err(Error::CLIArg("cannot convert the luapath extension path (supplied as --add-require-path) to a UTF-8 string. But this is sadly required by the mlua interface (the library to run Lua)".to_owned())),
    };

    // (1) load litua libraries
    let litua_table = include_str!("litua.lua");
    lua.load(litua_table).set_name("litua.lua")?.exec()?;
    let litua_lib = include_str!("litua.lib.lua");
    lua.load(litua_lib).set_name("litua.lib.lua")?.exec()?;

    // (2) find hook files
    let mut hook_files = vec![];
    for dir_entry in fs::read_dir(hooks_dir)? {
        let entry = dir_entry?;
        let basename = entry.file_name();
        if let Some(name) = basename.to_str() {
            if name.starts_with("hook") && name.ends_with(".lua") {
                println!("Loading hook file '{name}'");
                hook_files.push(entry.path());
            }
        }
    }

    // (3) read hook files
    for hook_file in hook_files.iter() {
        let lua_file_src = fs::read_to_string(hook_file)?;
        let mut chunk = lua.load(&lua_file_src);
        {
            let filepath = hook_file.display();
            chunk = chunk.set_name(&filepath.to_string())?;
        }
        chunk.exec()?;
    }

    // (4) load tree to lua environment
    let tree = doc.to_lua(&lua)?;

    // (5) load transform function and node object (libraries, users cannot modify)
    let litua_trans = include_str!("litua.transform.lua");
    lua.load(litua_trans).set_name("litua.transform.lua")?.exec()?;
    let litua_node = include_str!("litua.node.lua");
    lua.load(litua_node).set_name("litua.node.lua")?.exec()?;

    // (6) call transformation
    let globals = lua.globals();
    let global_litua: mlua::Table = globals.get("Litua")?;
    let transform: mlua::Function = global_litua.get("transform")?;
    println!("Running transformation.");
    let lua_result = transform.call::<mlua::Value, mlua::String>(tree)?;
    let result = lua_result.to_str()?;

    // (7) print the result
    fs::write(dst, result)?;

    Ok(())
}

fn derive_destination_filepath(p: &path::Path) -> path::PathBuf {
    if let Some(ext) = p.extension() {
        if ext == "lit" {
            p.with_extension("out")
        } else {
            p.with_extension("lit")
        }
    } else {
        path::PathBuf::from("doc.out")
    }
}

fn lex_and_parse(conf: &Settings, src: &path::Path) -> Result<tree::DocumentTree, Error> {
    let mut fd = fs::File::open(src)?;
    let mut buf = Vec::new();
    fd.read_to_end(&mut buf)?;

    let source_code = str::from_utf8(&buf)?;
    let l = lexer::Lexer::new(source_code);

    if conf.dump_lexed {
        for tok_or_err in l.iter() {
            let token = tok_or_err?;
            println!("Token= {token:?}");
        }
    } else if conf.dump_parsed {
        let mut p = parser::Parser::new(src, source_code);
        p.consume_iter(l.iter())?;
        p.finalize()?;

        println!("{:?}", p.tree());
    } else {
        let mut p = parser::Parser::new(src, source_code);
        p.consume_iter(l.iter())?;
        p.finalize()?;

        return Ok(p.tree());
    }

    Ok(tree::DocumentTree::new())
}

#[derive(Parser, Debug)]
#[command(name = "litua")]
#[command(author = "tajpulo <admin@lukas-prokop.at>")]
#[command(version = "0.9")]
#[command(about = "Read document as tree and apply Lua functions to nodes")]
#[command(author, version, about, long_about = None)]
struct Settings {
    // helpful for debugging 
    #[arg(long)]
    dump_lexed: bool,
    #[arg(long)]
    dump_parsed: bool,
    #[arg(long)]
    dump_hooks: bool,

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


fn main() -> Result<(), Error> {
    let conf = Settings::parse();

    let src = conf.source.as_path();
    let derived_dst = derive_destination_filepath(src);
    let dst = match &conf.destination {
        Some(p) => p.as_path(),
        None => derived_dst.as_path(),
    };

    let default_hooks_dir = path::PathBuf::from(".");
    let default_lua_path_additions = path::PathBuf::from("");

    let hooks_dir = match &conf.hooks_dir {
        Some(d) => d.as_path(),
        None => conf.source.parent().unwrap_or(default_hooks_dir.as_path()),
    };
    let lua_path_additions = match &conf.add_require_path {
        Some(d) => d.as_path(),
        None => &default_lua_path_additions,
    };

    let doctree = lex_and_parse(&conf, src)?;
    if !conf.dump_lexed && !conf.dump_parsed {
        run_lua(&conf, dst, &doctree, hooks_dir, lua_path_additions)?;
        println!("File '{}' read.", src.display());
        println!("File '{}' written.", dst.display());
    }

    Ok(())
}
