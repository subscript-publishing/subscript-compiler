#![allow(unused)]
pub mod codegen;
pub mod cli;
pub mod frontend;


fn main() {
    cli::run_cli();
}
