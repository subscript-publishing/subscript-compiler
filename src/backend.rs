pub mod ast;

use ast::*;

pub fn run() {
    let source = include_str!("../source.txt");
    let result = crate::parser::run_parser(source)
        .into_iter()
        .map(crate::parser::ast::Ast::to_backend)
        .collect::<Vec<_>>();
    println!("{:#?}", result);
}

