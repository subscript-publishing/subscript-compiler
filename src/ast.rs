#[derive(Debug, Clone)]
pub struct RewriteRule {
    pattern: Ast,
}

#[derive(Debug, Clone)]
pub struct Block {
    name: String,
    parameters: Vec<String>,
    body: Vec<Ast>,
    rewrite_rules: Vec<RewriteRule>,
}

#[derive(Debug, Clone)]
pub enum Ast {
    Block(Block),
}

pub fn run() {
    // let source = include_str!("../source.txt");
    // let result = run_parser(source);
}