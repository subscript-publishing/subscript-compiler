#![allow(unused)]
use subscript_compiler;

fn main() {
    let source = include_str!("../source.txt");
    let xs = subscript_compiler::frontend::run_highlighter(source);
    for x in xs {
        println!("{:#?}", x);
    }
}
