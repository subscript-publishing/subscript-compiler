//! Internal
#![allow(unused)]
use subscript_compiler;

fn main() {
    let source = include_str!("./other/bad-path.txt");
    let parsed = subscript_compiler::frontend::pass::pp_normalize::run_compiler_frontend(source);
    for node in parsed {
        println!("{:#?}", node);
    }
}
