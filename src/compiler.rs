//! External Compiler API
use either::Either;
use serde::{Serialize, Deserialize};
pub mod data;

pub fn compile_to_html(source: &str) -> String {
    let parsed = crate::frontend::run_compiler_frontend(source);
    let result = crate::backend::passes::to_html_pipeline(parsed);
    crate::codegen::html::render_document(result)
}

#[derive(Debug, Clone)]
pub enum SourceEntry {
    Html(String),
    Pdf(Vec<String>),
}

pub fn compile_entries(sources: Vec<SourceEntry>) {
    let outputs = sources
        .into_iter()
        .map(|src| {
            
        })
        .collect::<Vec<_>>();
    unimplemented!()
}

