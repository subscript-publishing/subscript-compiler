use std::rc::Rc;
pub mod parser;
pub mod ast;
pub mod query;
pub mod pass;
pub mod data;

use ast::Node;

pub fn run_highlighter<'a>(source: &'a str) -> Vec<ast::Highlight<'a>> {
    let children = parser::parse_source(source);
    let children = Node::new_fragment(children);
    children.into_highlight_ranges(Default::default(), None)
}
