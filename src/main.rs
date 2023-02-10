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

// Error type (covers all error cases)
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

// auxiliary functions

/// Does the given Path correspond to an empty string?
fn path_is_empty(p: &path::Path) -> bool {
    match p.to_str() {
        Some(s) => s.is_empty(),
        None => false, // NOTE: debatable, but meaningful for us
    }
}

/// Determine the set of hook files in the directory at the given filepath
fn find_hook_files(hooks_dir: &path::Path) -> Result<Vec<path::PathBuf>, io::Error> {
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
    Ok(hook_files)
}

/// Take a `conf` with settings and run the Lua runtime on the tree `doc`
fn run_lua(conf: &Settings, doc: &tree::DocumentTree) -> Result<(), Error> {
    // NOTE: 'debug' library is only available with Lua::unsafe_new()
    //       https://github.com/khvzak/mlua/issues/39
    let lua = unsafe { Lua::unsafe_new() };

    for lua_path in conf.lua_path_additions.iter() {
        let addition_str = path::PathBuf::from(&lua_path);
        match addition_str.to_str() {
            Some(s) if !s.is_empty() => lua.load(&format!("package.path = package.path .. ';{s}'")).exec()?,
            Some(_) => {},
            None => return Err(Error::CLIArg("cannot convert the luapath extension path (supplied as --add-require-path) to a UTF-8 string. But this is sadly required by the mlua interface (the library to run Lua)".to_owned())),
        };
    }

    // (1) load litua libraries
    let litua_table = include_str!("litua.lua");
    lua.load(litua_table).set_name("litua.lua")?.exec()?;
    let litua_lib = include_str!("litua.lib.lua");
    lua.load(litua_lib).set_name("litua.lib.lua")?.exec()?;

    // (2) find hook files
    let hook_files = find_hook_files(&conf.hooks_dir).map_err(Error::Io)?;

    // (3) read hook files
    for hook_file in hook_files.iter() {
        eprintln!("Loading hook file '{}'", hook_file.display());

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
    eprintln!("Running transformation.");
    let lua_result = transform.call::<mlua::Value, mlua::String>(tree)?;
    let result = lua_result.to_str()?;

    // (7) print the result
    fs::write(&conf.destination, result)?;

    Ok(())
}

/// Read the source file mentioned in `conf`, lex its source code,
/// and then parse it. The resulting tree will be returned.
fn lex_and_parse(conf: &Settings) -> Result<tree::DocumentTree, Error> {
    let mut fd = fs::File::open(&conf.source)?;
    let mut buf = Vec::new();
    fd.read_to_end(&mut buf)?;

    let source_code = str::from_utf8(&buf)?;
    let l = lexer::Lexer::new(source_code);

    let mut p = parser::Parser::new(&conf.source, source_code);
    p.consume_iter(l.iter())?;
    p.finalize()?;

    Ok(p.tree())
}

// Function invoked due to some operation argument

/// Read the source file mentioned in `conf`, lex its source code,
/// then parse it and finally process the resulting tree with Lua.
/// In conclusion, this is Litua's default main routine.
fn lex_and_parse_and_run(conf: &Settings) -> Result<(), Error> {
    let doctree = lex_and_parse(conf)?;
    println!("File '{}' read.", conf.source.display());

    run_lua(conf, &doctree)?;
    println!("File '{}' written.", conf.destination.display());

    Ok(())
}

/// Read the source file mentioned in `conf`, lex its source code,
/// then parse it and print the resulting tree. Useful for debugging.
fn lex_and_parse_and_dump(conf: &Settings) -> Result<(), Error> {
    let doctree = lex_and_parse(conf)?;
    println!("{doctree:?}");

    Ok(())
}

/// Read the source file mentioned in `conf` and lex its source code.
/// Print the resulting sequence of tokens. Useful for debugging.
fn lex_and_dump(conf: &Settings) -> Result<(), Error> {
    let mut fd = fs::File::open(&conf.source)?;
    let mut buf = Vec::new();
    fd.read_to_end(&mut buf)?;

    let source_code = str::from_utf8(&buf)?;
    let l = lexer::Lexer::new(source_code);

    for tok_or_err in l.iter() {
        let token = tok_or_err?;
        println!("Token= {token:?}");
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[command(name = "litua")]
#[command(author = "tajpulo <admin@lukas-prokop.at>")]
#[command(version = "1.1.1")]
#[command(about = "Read document as tree and apply Lua functions to nodes")]
#[command(author, version, about, long_about = None)]
struct CLISettings {
    // helpful for debugging 
    #[arg(long, help = "if set, only prints the runtime configuration and exits")]
    dump_config: bool,
    #[arg(long, help = "if set, only lexes the source file, prints its tokens and exits")]
    dump_lexed: bool,
    #[arg(long, help = "if set, only parses the source file, prints the resulting tree and exits")]
    dump_parsed: bool,

    // configuration
    #[arg(long, value_name = "DIR", help = "filepath to directory with hook files (default: same as source file)")]
    hooks_dir: Option<path::PathBuf>,
    #[arg(long, value_name = "DIR", help = "directories to add as search location for require(…) calls")]
    add_require_path: Vec<path::PathBuf>,

    // optional argument
    #[arg(short = 'o', long, value_name = "PATH")]
    destination: Option<path::PathBuf>,

    // positional argument
    source: path::PathBuf,
}

#[derive(Debug)]
struct Settings {
    hooks_dir: path::PathBuf,
    lua_path_additions: Vec<path::PathBuf>,
    source: path::PathBuf,
    destination: path::PathBuf,
}

fn main() -> Result<(), Error> {
    // CLI argument parsing
    let settings = CLISettings::parse();

    let derived_dst = derive_destination_filepath(&settings.source);
    let dst = match &settings.destination {
        Some(p) => p.as_path(),
        None => derived_dst.as_path(),
    };

    // if you specified some hook directory, use it.
    // if not, use the folder the source file lies within
    let default_hooks_dir = path::PathBuf::from(".");
    let hooks_dir = match &settings.hooks_dir {
        Some(d) if path_is_empty(&d) => default_hooks_dir.as_path(),
        Some(d) => d.as_path(),
        None => match settings.source.parent() {
            Some(p) if path_is_empty(p) => &default_hooks_dir.as_path(),
            Some(p) => p,
            None => &default_hooks_dir.as_path(),
        },
    };

    let mut lua_path_additions = vec![];
    for dir in settings.add_require_path.iter() {
        lua_path_additions.push(dir.to_owned());
    }

    // define execution configuration
    let conf = Settings {
        hooks_dir: hooks_dir.to_owned(),
        lua_path_additions,
        source: settings.source,
        destination: dst.to_owned(),
    };

    // run main routine
    if settings.dump_config {
        println!("{:?}", &conf);
        Ok(())
    } else if settings.dump_lexed {
        lex_and_dump(&conf)
    } else if settings.dump_parsed {
        lex_and_parse_and_dump(&conf)
    } else {
        lex_and_parse_and_run(&conf)
    }
}
