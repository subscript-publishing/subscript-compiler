//! Compiles the given Subscript source code to HTML. 


fn main() {
    let source = include_str!("./source/electrical-engineering.txt");
    let html = subscript_compiler::compiler::compile_to_html(source);
    println!("{}", html);
}
