
pub fn compile_to_html(source: &str) -> String {
    let parsed = crate::frontend::parser::run_parser(source);
    let result = crate::frontend::passes::to_html_pipeline(parsed);
    let output = crate::codegen::html::render_document(result);
    output
}

