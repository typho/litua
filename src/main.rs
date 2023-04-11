use litua;

use mlua::prelude::*;
use clap::Parser;

use std::fs;
use std::io;
use std::io::prelude::*;
use std::path;
use std::str;

use std::error;
use std::fmt;

macro_rules! log {
    ($fmt:literal) => { eprintln!(concat!("LOG[rust]:\t", $fmt)); };
    ($fmt:literal, $($args:expr),+) => { eprintln!(concat!("LOG[rust]:\t", $fmt), $($args),+); };
}

// Error type (covers all error cases)
#[derive(Debug)]
enum Error {
    CLIArg(String),
    Io(io::Error),
    Encoding(str::Utf8Error),
    Litua(litua::errors::Error),
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
            Mlua(err) => write!(f, "{err}"),
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

impl From<litua::errors::Error> for Error {
    fn from(error: litua::errors::Error) -> Self {
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

/// Run the entire pipeline according to the operation specified in `conf`.
/// Might include lexing and parsing unless you specified a debugging operation
/// like dump_lexed or dump_parsed. It reads some source code, prepares the
/// Lua runtime, lexes and parses the source code, applies some hook, and
/// writes the result back to a file.
/// In conclusion, this is Litua's main routine.
fn run(conf: &Settings) -> Result<(), Error> {
    // (0) initialize Lua runtime
    // NOTE: 'debug' library is only available with Lua::unsafe_new()
    //       https://github.com/khvzak/mlua/issues/39
    let lua = unsafe { Lua::unsafe_new() };
    log!("Lua runtime initialized");

    // (1) add paths to Lua path variable
    for lua_path in conf.lua_path_additions.iter() {
        let addition_str = path::PathBuf::from(&lua_path);
        match addition_str.to_str() {
            Some(s) if !s.is_empty() => lua.load(&format!("package.path = package.path .. ';{s}'")).exec()?,
            Some(_) => {},
            None => return Err(Error::CLIArg("cannot convert the luapath extension path (supplied as --add-require-path) to a UTF-8 string. But this is sadly required by the mlua interface (the library to run Lua)".to_owned())),
        };
    }
    log!("Lua paths added");

    // (2) find hook files
    let hook_files = find_hook_files(&conf.hooks_dir).map_err(Error::Io)?;
    log!("{} hook file{} found", hook_files.len(), if hook_files.len() == 1 { "" } else { "" });

    // (3) load litua libraries
    let litua_table = include_str!("litua.lua");
    lua.load(litua_table).set_name("litua.lua")?.exec()?;
    let litua_lib = include_str!("litua_stdlib.lua");
    lua.load(litua_lib).set_name("litua_stdlib.lua")?.exec()?;
    log!("litua standard library loaded");

    // (4) read hook files
    for hook_file in hook_files.iter() {
        log!("Loading hook file '{}'", hook_file.display());

        let lua_file_src = fs::read_to_string(hook_file)?;
        let mut chunk = lua.load(&lua_file_src);
        {
            let filepath = hook_file.display();
            chunk = chunk.set_name(&filepath.to_string())?;
        }
        chunk.exec()?;
    }
    log!("All hook files loaded");

    // (5) run preprocessing hooks
    let mut doc_src = {
        let mut fd = fs::File::open(&conf.source)?;
        let mut buf = Vec::new();
        fd.read_to_end(&mut buf)?;
        str::from_utf8(&buf)?.to_owned()
    };
    log!("source file '{}' read", conf.source.display());

    {
        let globals = lua.globals();
        let global_litua: mlua::Table = globals.get("Litua")?;
        let preprocess: mlua::Function = global_litua.get("preprocess")?;
        let lua_result = preprocess.call::<mlua::Value, mlua::String>(doc_src.to_lua(&lua)?)?;
        // TODO verify which errors are triggered for non-UTF-8 return values
        doc_src = lua_result.to_str()?.to_owned();
    }
    log!("source file '{}' pre-processed", conf.source.display());

    // (6) lex and parse source code to turn it into a tree
    let doc_tree = {
        let l = litua::lexer::Lexer::new(&doc_src);

        if conf.op == "dump_lexed" {
            // Read the source file mentioned in `conf` and lex its source code.
            // Print the resulting sequence of tokens. Useful for debugging.
            let l = litua::lexer::Lexer::new(&doc_src);

            for tok_or_err in l.iter() {
                let token = match tok_or_err {
                    Ok(tok) => tok,
                    Err(e) => return Err(Error::Litua(e.format_with_source(&conf.source, &doc_src))),
                };
                println!("{token:?}");
            }

            return Ok(());
        }

        let mut p = litua::parser::Parser::new(&conf.source, &doc_src);
        p.consume_iter(l.iter())?;
        p.finalize()?;

        p.tree()
    };
    log!("source file '{}' lexed and parsed", conf.source.display());

    if conf.op == "dump_parsed" {
        // Read the source file mentioned in `conf` and lex and parse
        // its source code. Print the resulting tree. Useful for debugging.
        println!("{doc_tree:?}");
        return Ok(());
    }

    // (7) turn tree into a Lua object
    let tree = doc_tree.to_lua(&lua)?;
    log!("parsed tree converted into a Lua table");

    // (8) load transform function and node object (libraries, which users must not modify)
    let litua_trans = include_str!("litua_transform.lua");
    lua.load(litua_trans).set_name("litua_transform.lua")?.exec()?;
    let litua_node = include_str!("litua_node.lua");
    lua.load(litua_node).set_name("litua_node.lua")?.exec()?;
    log!("litua transformation routines loaded");

    // (9) call transformation
    let globals = lua.globals();
    let global_litua: mlua::Table = globals.get("Litua")?;

    let intermediate = {
        let transform: mlua::Function = global_litua.get("transform")?;
        transform.call::<mlua::Value, mlua::String>(tree)?
    };
    log!("litua hooks for tree manipulation finished");

    // (10) run postprocessing hooks
    let postprocess: mlua::Function = global_litua.get("postprocess")?;
    let lua_result = postprocess.call::<mlua::Value, mlua::String>(intermediate.to_lua(&lua)?)?;
    let output = lua_result.to_str()?;
    log!("source file '{}' post-processed", conf.source.display());

    // (11) print the result
    fs::write(&conf.destination, output)?;
    log!("File '{}' written.", conf.destination.display());

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
    op: &'static str,
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
        op: if settings.dump_lexed {
            "dump_lexed"
        } else if settings.dump_parsed {
            "dump_parsed"
        } else {
            "run"
        },
    };

    // run main routine
    if settings.dump_config {
        println!("{:?}", &conf);
        return Ok(());
    }

    run(&conf)
}
