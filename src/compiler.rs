//! External Compiler API


pub fn compile_to_html(source: &str) -> String {
    let parsed = crate::frontend::run_compiler_frontend(source);
    let result = crate::backend::passes::to_html_pipeline(parsed);
    crate::codegen::html::render_document(result)
}

