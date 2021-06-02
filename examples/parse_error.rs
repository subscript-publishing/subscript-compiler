#![allow(unused)]
use subscript_compiler;

fn main() {
    let source = include_str!("./other/invalid.txt");
    let xs = subscript_compiler::frontend::parser::parse_source(source);
    for x in xs {
        println!("{:#?}", x);
    }
}
