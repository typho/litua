use std::collections::HashMap;
use std::ffi::OsString;

pub(crate) struct DocumentTree(pub(crate) DocumentElement);

impl DocumentTree {
    pub(crate) fn new() -> DocumentTree {
        DocumentTree(DocumentElement::Function(DocumentFunction {
            name: "document".to_owned(),
            args: HashMap::new(),
            content: Vec::new()
        }))
    }

    pub(crate) fn from_filepath(filepath: OsString) -> DocumentTree {
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

pub(crate) struct DocumentFunction {
    pub(crate) name: String,
    pub(crate) args: HashMap<String, DocumentNode>,
    pub(crate) content: DocumentNode,
}

impl DocumentFunction {
    pub(crate) fn new() -> DocumentFunction {
        DocumentFunction { name: "".to_owned(), args: HashMap::new(), content: Vec::new() }
    }

    pub(crate) fn empty_element() -> DocumentElement {
        DocumentElement::Function(Self::new())
    }
}

/// `DocumentElement` is either a function (call a name with arguments and text content)
/// or simply text without association to a function. 
pub(crate) enum DocumentElement {
    Function(DocumentFunction),
    Text(String),
}

/// `DocumentNode` is a node establishing a tree.
/// Each node consists of zero or more elements constituting its children.
pub(crate) type DocumentNode = Vec<DocumentElement>;

