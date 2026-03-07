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
    /// Compile an intent specification to IR (JSON)
    Compile {
        /// Path to the .intent file
        file: PathBuf,
    },
    /// Verify an intent specification's IR for structural correctness
    Verify {
        /// Path to the .intent file
        file: PathBuf,
    },
    /// Show the audit trace map (spec items → IR constructs)
    Audit {
        /// Path to the .intent file
        file: PathBuf,
    },
    /// Show coverage summary for an intent specification
    Coverage {
        /// Path to the .intent file
        file: PathBuf,
    },
    /// Show spec-level diff between two versions of an intent file
    Diff {
        /// Path to the old .intent file
        old: PathBuf,
        /// Path to the new .intent file
        new: PathBuf,
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
                    let report = miette::Report::new(err.clone()).with_source_code(source.clone());
                    handler.render_report(&mut buf, report.as_ref()).ok();
                    eprint!("{buf}");
                }
                eprintln!("{} error(s) in {}", errors.len(), file.display());
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
        Commands::Compile { file } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);
            let ir = intent_ir::lower_file(&ast);
            let json = serde_json::to_string_pretty(&ir).expect("IR serialization failed");
            println!("{json}");
        }
        Commands::Verify { file } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);

            // Run semantic checks first
            let check_errors = intent_check::check_file(&ast);
            if !check_errors.is_empty() {
                let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
                for err in &check_errors {
                    let mut buf = String::new();
                    let report = miette::Report::new(err.clone()).with_source_code(source.clone());
                    handler.render_report(&mut buf, report.as_ref()).ok();
                    eprint!("{buf}");
                }
                eprintln!(
                    "{} semantic error(s) in {}",
                    check_errors.len(),
                    file.display()
                );
                process::exit(1);
            }

            // Lower to IR and verify
            let ir = intent_ir::lower_file(&ast);
            let ir_errors = intent_ir::verify_module(&ir);
            if ir_errors.is_empty() {
                println!(
                    "VERIFIED: {} — {} function(s), {} invariant(s), {} struct(s)",
                    ir.name,
                    ir.functions.len(),
                    ir.invariants.len(),
                    ir.structs.len(),
                );

                // Coherence analysis: show verification obligations
                let obligations = intent_ir::analyze_obligations(&ir);
                if !obligations.is_empty() {
                    println!("\nVerification obligations:");
                    for ob in &obligations {
                        println!("  - {ob}");
                    }
                }
            } else {
                for err in &ir_errors {
                    eprintln!(
                        "verify: {} (in {}.{}:{})",
                        err, err.trace.module, err.trace.item, err.trace.part
                    );
                }
                eprintln!(
                    "{} verification error(s) in {}",
                    ir_errors.len(),
                    file.display()
                );
                process::exit(1);
            }
        }
        Commands::Audit { file } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);
            let ir = intent_ir::lower_file(&ast);
            let errors = intent_ir::verify_module(&ir);
            let obligations = intent_ir::analyze_obligations(&ir);
            let report = intent_ir::generate_audit(&source, &ir, &errors, &obligations);
            print!("{}", report.format_trace_map());
        }
        Commands::Coverage { file } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);
            let ir = intent_ir::lower_file(&ast);
            let errors = intent_ir::verify_module(&ir);
            let obligations = intent_ir::analyze_obligations(&ir);
            let report = intent_ir::generate_audit(&source, &ir, &errors, &obligations);
            print!("{}", report.format_coverage());
        }
        Commands::Diff { old, new } => {
            let old_source = read_source(&old);
            let old_ast = parse_or_exit(&old_source, &old);
            let old_ir = intent_ir::lower_file(&old_ast);
            let old_errors = intent_ir::verify_module(&old_ir);
            let old_obligations = intent_ir::analyze_obligations(&old_ir);
            let old_report =
                intent_ir::generate_audit(&old_source, &old_ir, &old_errors, &old_obligations);

            let new_source = read_source(&new);
            let new_ast = parse_or_exit(&new_source, &new);
            let new_ir = intent_ir::lower_file(&new_ast);
            let new_errors = intent_ir::verify_module(&new_ir);
            let new_obligations = intent_ir::analyze_obligations(&new_ir);
            let new_report =
                intent_ir::generate_audit(&new_source, &new_ir, &new_errors, &new_obligations);

            let diff = intent_ir::diff_reports(&old_report, &new_report);
            print!("{}", diff.format());
        }
    }
}
