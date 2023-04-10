//! rust components of litua - a tool to read a text document,
//! receive its tree in Lua and manipulate it before representing it as string.

pub mod errors;
pub mod lexer;
pub mod parser;
pub mod tree;
pub(crate) mod lines_with_indices;
