use std::fs;
use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

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
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { file } => {
            let source = match fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not read {}: {}", file.display(), e);
                    process::exit(1);
                }
            };

            match intent_parser::parse_file(&source) {
                Ok(ast) => {
                    println!(
                        "OK: {} — {} top-level item(s)",
                        ast.module.name,
                        ast.items.len()
                    );
                }
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            }
        }
        Commands::Render { file } => {
            let source = match fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not read {}: {}", file.display(), e);
                    process::exit(1);
                }
            };

            match intent_parser::parse_file(&source) {
                Ok(ast) => {
                    let md = intent_render::markdown::render(&ast);
                    print!("{}", md);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            }
        }
    }
}
