use std::rc::Rc;
pub mod parser;
pub mod ast;
pub mod passes;
pub mod highlighter;

use ast::Node;

/// Parses the given source code and returns a normalized backend AST vector.
pub fn run_frontend<'a>(source: &'a str) -> Vec<crate::backend::Ast<'a>> {
    use crate::backend;
    // PARSE SOURCE CODE
    let children = parser::parse_source(source);
    // TO BACKEND AST
    let children = passes::to_unnormalized_backend_ir(children);
    // NORMALIZE SETUP
    let transfomer = backend::ast::ChildListTransformer {
        parameters: Rc::new(std::convert::identity),
        block: Rc::new(passes::normalize_ir),
        rewrite_rules: Rc::new(std::convert::identity),
        marker: std::marker::PhantomData
    };
    let node = backend::Ast::new_fragment(children);
    // NORMALIZE BACKNED IR
    let node = node.child_list_transformer(Rc::new(transfomer));
    // DONE
    node.into_fragment()
}

