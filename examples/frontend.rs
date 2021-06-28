//! Internal.


fn main() {
    let source = include_str!("./source/electrical-engineering.txt");
    // let source = "\\h1{Hello world}";
    let nodes = subscript_compiler::frontend::pass::pp_normalize::run_compiler_frontend(source);
    // let nodes = subscript_compiler::frontend::pass::html_normalize::html_canonicalization(nodes);
    for node in nodes {
        println!("{}", node.to_string());
    }
}
