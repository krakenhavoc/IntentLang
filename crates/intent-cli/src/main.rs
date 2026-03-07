use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand};
use miette::{GraphicalReportHandler, GraphicalTheme};

#[derive(Parser)]
#[command(name = "intent", version, about = "IntentLang specification toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and validate an intent specification file
    Check {
        /// Path to the .intent file
        file: PathBuf,
    },
    /// Render an intent specification to Markdown
    Render {
        /// Path to the .intent file
        file: PathBuf,
    },
    /// Render an intent specification to HTML
    RenderHtml {
        /// Path to the .intent file
        file: PathBuf,
    },
}

fn read_source(file: &Path) -> String {
    match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read {}: {}", file.display(), e);
            process::exit(1);
        }
    }
}

fn parse_or_exit(source: &str, file: &Path) -> intent_parser::ast::File {
    match intent_parser::parse_file(source) {
        Ok(ast) => ast,
        Err(e) => {
            let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
            let mut buf = String::new();
            let report = miette::Report::new(e).with_source_code(source.to_string());
            handler.render_report(&mut buf, report.as_ref()).ok();
            eprint!("{buf}");
            eprintln!("1 error(s) in {}", file.display());
            process::exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { file } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);

            let errors = intent_check::check_file(&ast);
            if errors.is_empty() {
                println!(
                    "OK: {} — {} top-level item(s), no issues found",
                    ast.module.name,
                    ast.items.len()
                );
            } else {
                let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
                for err in &errors {
                    let mut buf = String::new();
                    let report = miette::Report::new(err.clone())
                        .with_source_code(source.clone());
                    handler.render_report(&mut buf, report.as_ref()).ok();
                    eprint!("{buf}");
                }
                eprintln!(
                    "{} error(s) in {}",
                    errors.len(),
                    file.display()
                );
                process::exit(1);
            }
        }
        Commands::Render { file } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);
            let md = intent_render::markdown::render(&ast);
            print!("{}", md);
        }
        Commands::RenderHtml { file } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);
            let html = intent_render::html::render(&ast);
            print!("{}", html);
        }
    }
}
