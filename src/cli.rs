use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name="subscript",
    about = "compile subscript markup into HTML, or PDF (WIP)",
)]
enum Cli {
    Compile {
        #[structopt(short, long, parse(from_os_str))]
        source: PathBuf,
        #[structopt(short, long, parse(from_os_str))]
        output: Option<PathBuf>,
    },
}

pub fn run_cli() {
    match Cli::from_args() {
        Cli::Compile{source: source_path, output} => {
            let source = std::fs::read_to_string(&source_path).unwrap();
            let output_path = output.unwrap_or_else(|| {
                let source_path = source_path.clone();
                let default = std::ffi::OsStr::new("html");
                let ext = source_path.extension().unwrap_or(default);
                let mut path = source_path.clone();
                assert!(path.set_extension(ext));
                path
            });
            if let Some(parent) = output_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            // let output = crate::frontend::pass::to_html::compile_to_html(&source);
            let output = crate::codegen::html::Document::from_source(&source);
            let output = output.render_to_string();
            std::fs::write(&output_path, output).unwrap();
        }
    }
}


