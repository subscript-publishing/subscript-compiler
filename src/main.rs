#![allow(unused)]
pub mod parser;
pub mod parser_utils;
pub mod backend;
pub mod codegen;
pub mod frontend;

fn main() {
    // ast::run();
    // backend::run();
    frontend::run();
}
