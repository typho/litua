use std::collections::HashMap;
use std::ffi::OsString;

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

/// `DocumentElement` is either a function (call a name with arguments and text content)
/// or simply text without association to a function. 
pub enum DocumentElement {
    Function(DocumentFunction),
    Text(String),
}

/// `DocumentNode` is a node establishing a tree.
/// Each node consists of zero or more elements constituting its children.
pub type DocumentNode = Vec<DocumentElement>;

