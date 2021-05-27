#![allow(unused)]
pub mod codegen;
pub mod frontend;

fn main() {
    // ast::run();
    // backend::run();
    // frontend::parser::dev();
    frontend::ast::dev();
}
