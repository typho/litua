//! Tree structure of a litua text document

use std::collections::HashMap;

/// `DocumentTree` represents the root element of the Abstract Syntax Tree
#[derive(Clone,Debug,PartialEq)]
pub struct DocumentTree(pub DocumentElement);

impl DocumentTree {
    /// Create a new `DocumentTree`, which consists of one root
    /// call `document`.
    pub fn new() -> DocumentTree {
        DocumentTree(DocumentElement::Function(DocumentFunction {
            name: "document".to_owned(),
            args: HashMap::new(),
            content: Vec::new()
        }))
    }
}

impl Default for DocumentTree {
    fn default() -> Self {
        Self::new()
    }
}

impl<'lua> mlua::ToLua<'lua> for &DocumentTree {
    fn to_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        self.0.to_lua(lua)
    }
}

/// `DocumentFunction` is a function call in the text document. For example,
/// ``{text[style=bold] message}`` is a `DocumentFunction` with `name` “text”,
/// `args` such that `style` is associated with `DocumentNode::Text` “bold”
/// and `content` is given as `DocumentNode::Text` “message”.
#[derive(Clone,Debug,PartialEq)]
pub struct DocumentFunction {
    pub name: String,
    pub args: HashMap<String, DocumentNode>,
    pub content: DocumentNode,
}

impl DocumentFunction {
    /// Returns an empty `DocumentFunction` without args or content and `name` is set to “”.
    pub fn new() -> DocumentFunction {
        DocumentFunction { name: "".to_owned(), args: HashMap::new(), content: Vec::new() }
    }

    /// Returns an empty `DocumentElement::Function` without args or content and `name` is set to “”.
    pub fn empty_element() -> DocumentElement {
        DocumentElement::Function(Self::new())
    }
}

impl Default for DocumentFunction {
    fn default() -> Self {
        Self::new()
    }
}

impl<'lua> mlua::ToLua<'lua> for &DocumentFunction {
    /// Lua representation of a `DocumentFunction`
    fn to_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        let node = lua.create_table()?;

        // define call
        node.set("call", self.name.clone())?;

        // define args
        let args = lua.create_table()?;
        for (arg, elements) in self.args.iter() {
            let lua_value = lua.create_table()?;
            for (i, element) in elements.iter().enumerate() {
                lua_value.set(i + 1, element)?;
            }
            args.set(arg.as_str(), lua_value)?;
        }
        node.set("args", args)?;

        // define content
        let content = lua.create_table()?;
        for (i, child) in self.content.iter().enumerate() {
            content.set(i + 1, child)?;
        }
        node.set("content", content)?;

        Ok(mlua::Value::Table(node))
    }
}

/// `DocumentElement` is either a function (call with arguments and text content)
/// or simply Unicode text without association to a function.
#[derive(Clone,Debug,PartialEq)]
pub enum DocumentElement {
    Function(DocumentFunction),
    Text(String),
}

impl<'lua> mlua::ToLua<'lua> for &DocumentElement {
    /// Lua representation of a `DocumentElement`.
    fn to_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        match self {
            DocumentElement::Function(func) => func.to_lua(lua),
            DocumentElement::Text(text) => text.clone().to_lua(lua),
        }
    }
}

/// `DocumentNode` is a node establishing a tree.
/// Each node consists of zero or more elements constituting its children.
pub type DocumentNode = Vec<DocumentElement>;
