//! Internal
#![allow(unused)]
use subscript_compiler;

fn main() {
    let source = include_str!("./other/toc.txt");
    let nodes = subscript_compiler::frontend::pass::pp_normalize::run_compiler_frontend(source);
    // let ast = subscript_compiler::backend::Ast::new_fragment(nodes);
    // let toc = subscript_compiler::backend::query::query_heading_nodes(&ast);
    // for node in toc {
    //     let node = subscript_compiler::backend::Ast::Tag(node);
    //     println!("{:#?}", node.to_string());
    // }
}
