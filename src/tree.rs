use std::collections::HashMap;
use std::ffi::OsString;
use mlua;

const KEY_CALL: &str = "=call";
const KEY_ARGNAMES: &str = "=args";
const KEY_CONTENT: &str = "=content";

#[derive(Clone,Debug,PartialEq)]
pub struct DocumentTree(pub DocumentElement);

impl DocumentTree {
    pub fn new() -> DocumentTree {
        DocumentTree(DocumentElement::Function(DocumentFunction {
            name: "document".to_owned(),
            args: HashMap::new(),
            content: Vec::new()
        }))
    }

    pub fn from_filepath(filepath: OsString) -> DocumentTree {
        let mut attrs = HashMap::new();
        if let Some(file) = filepath.to_str() {
            attrs.insert("filepath".to_owned(), vec![DocumentElement::Text(file.to_owned())]);
        }

        DocumentTree(DocumentElement::Function(DocumentFunction{
            name: "document".to_owned(),
            args: attrs,
            content: Vec::new()
        }))
    }
}

impl<'lua> mlua::ToLua<'lua> for &DocumentTree {
    fn to_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        Ok(self.0.to_lua(lua)?)
    }
}

#[derive(Clone,Debug,PartialEq)]
pub struct DocumentFunction {
    pub name: String,
    pub args: HashMap<String, DocumentNode>,
    pub content: DocumentNode,
}

impl DocumentFunction {
    pub fn new() -> DocumentFunction {
        DocumentFunction { name: "".to_owned(), args: HashMap::new(), content: Vec::new() }
    }

    pub fn empty_element() -> DocumentElement {
        DocumentElement::Function(Self::new())
    }
}

impl<'lua> mlua::ToLua<'lua> for &DocumentFunction {
    fn to_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        let tbl = lua.create_table()?;
        tbl.set("=call", self.name.clone())?;

        let arg_names = lua.create_table()?;
        for (i, (arg, _)) in self.args.iter().enumerate() {
            arg_names.set(i + 1, (*arg).clone())?;
        }
        tbl.set(KEY_ARGNAMES, arg_names)?;

        for (arg, elements) in self.args.iter() {
            let lua_value = lua.create_table()?;
            for (i, element) in elements.iter().enumerate() {
                lua_value.set(i + 1, element)?;
            }
            tbl.set((*arg).clone(), lua_value)?;
        }

        let lua_content = lua.create_table()?;
        for (i, child) in self.content.iter().enumerate() {
            lua_content.set(i + 1, child)?;
        }
        tbl.set(KEY_CONTENT, lua_content)?;
        Ok(mlua::Value::Table(tbl))
    }
}

/// `DocumentElement` is either a function (call a name with arguments and text content)
/// or simply text without association to a function. 
#[derive(Clone,Debug,PartialEq)]
pub enum DocumentElement {
    Function(DocumentFunction),
    Text(String),
}

impl<'lua> mlua::ToLua<'lua> for &DocumentElement {
    fn to_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        match self {
            DocumentElement::Function(func) => func.to_lua(lua),
            DocumentElement::Text(text) => Ok(text.clone().to_lua(lua)?),
        }
    }
}

/// `DocumentNode` is a node establishing a tree.
/// Each node consists of zero or more elements constituting its children.
pub type DocumentNode = Vec<DocumentElement>;
