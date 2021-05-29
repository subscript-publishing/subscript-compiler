#![feature(iter_intersperse)]
#![feature(linked_list_cursors)]
#![feature(slice_group_by)]
#![allow(unused)]
pub mod codegen;
pub mod backend;
pub mod cli;
pub mod compiler;
pub mod frontend;


fn main() {
    // ast::run();
    // backend::run();
    frontend::parser::dev();
    // frontend::ast::dev();
    // codegen::html::dev();
    // cli::run_cli();
}
