#![allow(unused)]
use subscript_compiler;
use subscript_compiler::frontend::parser::*;

fn main() {
    let source = include_str!("./other/valid.txt");
    let words = init_words(source, init_characters(source));
    let nodes = ParseTree::parse_words(words);
    for node in nodes {
        println!("{:#?}", node);
    }
}
