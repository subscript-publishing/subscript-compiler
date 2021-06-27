#![allow(unused)]
pub mod codegen;
pub mod backend;
pub mod cli;
pub mod compiler;
pub mod frontend;


fn main() {
    cli::run_cli();
}
