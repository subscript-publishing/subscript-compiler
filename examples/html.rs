//! Compiles the given Subscript source code to HTML. 


fn main() {
    let source = include_str!("./source/electrical-engineering.txt");
    let output = subscript_compiler::codegen::html::Document::from_source(source);
    let output = output.render_to_string();
    println!("{}", output);
}
