use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use miette::{GraphicalReportHandler, GraphicalTheme};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "intent", version, about = "IntentLang specification toolchain")]
struct Cli {
    /// Output format: human-readable (default) or JSON for agent consumption
    #[arg(long, global = true, default_value = "human")]
    output: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CodegenLang {
    Rust,
    Typescript,
    Python,
    Go,
    Java,
    Csharp,
    Swift,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InvariantPattern {
    /// forall a, b: Entity => a != b => a.field != b.field
    Unique,
    /// forall e: Entity => e.field >= 0
    NonNegative,
    /// forall a: Entity => exists b: RefEntity => a.field == b.ref_field
    NoDanglingRef,
    /// Add idempotent: true to an action's properties block
    Idempotent,
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
        /// Enable incremental verification (cache results, re-verify only changed items)
        #[arg(long)]
        incremental: bool,
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
    /// Query specific items from a spec (for agent integration)
    Query {
        /// Path to the .intent file
        file: PathBuf,
        /// What to query: entities, actions, invariants, edge-cases, or a specific name
        target: String,
    },
    /// Claim a spec item for an agent (multi-agent collaboration)
    Lock {
        /// Path to the .intent file
        file: PathBuf,
        /// Name of the spec item to claim
        item: String,
        /// Agent identifier
        #[arg(long)]
        agent: String,
    },
    /// Release a claimed spec item
    Unlock {
        /// Path to the .intent file
        file: PathBuf,
        /// Name of the spec item to release
        item: String,
        /// Agent identifier
        #[arg(long)]
        agent: String,
    },
    /// Show lock status for all spec items
    Status {
        /// Path to the .intent file
        file: PathBuf,
    },
    /// Format an intent specification file
    Fmt {
        /// Path to the .intent file
        file: PathBuf,
        /// Write formatted output back to the file (default: print to stdout)
        #[arg(long)]
        write: bool,
        /// Check if file is formatted (exit 1 if not)
        #[arg(long)]
        check: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
    /// Generate an .intent spec from a natural language description
    Generate {
        /// Natural language description of the spec to generate
        description: String,
        /// Confidence level 1-5 (higher = agent assumes more)
        #[arg(long, default_value = "3")]
        confidence: u8,
        /// Maximum validation retries
        #[arg(long, default_value = "2")]
        max_retries: u32,
        /// LLM model override (default: AI_MODEL env var or gpt-4o)
        #[arg(long)]
        model: Option<String>,
        /// Write output to file instead of stdout
        #[arg(short = 'o', long = "out")]
        out: Option<PathBuf>,
        /// Edit an existing spec file with the given description
        #[arg(long)]
        edit: Option<PathBuf>,
        /// Show diff when editing (instead of full output)
        #[arg(long)]
        diff: bool,
        /// Print raw LLM responses to stderr for debugging
        #[arg(long)]
        debug: bool,
    },
    /// Serve a spec as a REST API (stateless runtime)
    Serve {
        /// Path to the .intent file
        file: PathBuf,
        /// Address to bind to (default: 127.0.0.1:3000)
        #[arg(long, default_value = "127.0.0.1:3000")]
        addr: String,
    },
    /// Run spec-level tests defined in test blocks
    Test {
        /// Path to the .intent file
        file: PathBuf,
        /// Run only tests whose name contains this string
        #[arg(long)]
        filter: Option<String>,
    },
    /// Generate skeleton code from an intent specification
    Codegen {
        /// Path to the .intent file
        file: PathBuf,
        /// Target language: rust, typescript, python, or go
        #[arg(long, value_enum)]
        lang: CodegenLang,
        /// Output directory (default: print to stdout)
        #[arg(short = 'o', long = "out-dir")]
        out_dir: Option<PathBuf>,
    },
    /// Generate an OpenAPI 3.0 spec from an intent specification
    Openapi {
        /// Path to the .intent file
        file: PathBuf,
        /// Output file (default: print to stdout)
        #[arg(short = 'o', long = "out")]
        out: Option<PathBuf>,
    },
    /// Generate a full implementation from an intent specification using AI
    Implement {
        /// Path to the .intent file
        file: PathBuf,
        /// Target language: rust, typescript, or python
        #[arg(long, short = 'l', value_enum, default_value = "rust")]
        lang: CodegenLang,
        /// Output directory (default: print to stdout)
        #[arg(short = 'o', long = "out-dir")]
        out_dir: Option<PathBuf>,
        /// LLM model override (default: AI_MODEL env var or gpt-4o)
        #[arg(long)]
        model: Option<String>,
        /// Maximum validation retries (default: 2)
        #[arg(long, default_value = "2")]
        max_retries: u32,
        /// Print raw LLM responses to stderr for debugging
        #[arg(long)]
        debug: bool,
    },
    /// Generate contract test harness from spec test blocks
    TestHarness {
        /// Path to the .intent file
        file: PathBuf,
        /// Target language: rust (default)
        #[arg(long, short = 'l', value_enum, default_value = "rust")]
        lang: CodegenLang,
        /// Output file (default: print to stdout)
        #[arg(short = 'o', long = "out")]
        out: Option<PathBuf>,
    },
    /// Initialize a new .intent spec file
    Init {
        /// Module name (defaults to directory name)
        #[arg(long)]
        name: Option<String>,
        /// Output file path (defaults to <name>.intent)
        #[arg(short = 'o', long = "out")]
        out: Option<PathBuf>,
    },
    /// Add an invariant from a built-in pattern to a spec file
    AddInvariant {
        /// Path to the .intent file
        file: PathBuf,
        /// Pattern to use: unique, non-negative, no-dangling-ref, idempotent
        #[arg(long, value_enum)]
        pattern: InvariantPattern,
        /// Entity name (required for unique, non-negative, no-dangling-ref)
        #[arg(long)]
        entity: Option<String>,
        /// Action name (required for idempotent)
        #[arg(long)]
        action: Option<String>,
        /// Field arguments (pattern-dependent)
        fields: Vec<String>,
        /// Preview the generated invariant without modifying the file
        #[arg(long)]
        dry_run: bool,
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

/// Resolve a module graph (root + all transitive imports) or exit on error.
fn resolve_or_exit(file: &Path) -> intent_parser::ModuleGraph {
    match intent_parser::resolve(file) {
        Ok(graph) => graph,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Get the imported files for a given file from a module graph.
fn imported_files_for<'a>(
    file: &'a intent_parser::ast::File,
    graph: &'a intent_parser::ModuleGraph,
) -> Vec<&'a intent_parser::ast::File> {
    file.imports
        .iter()
        .filter_map(|use_decl| {
            graph
                .modules
                .values()
                .find(|m| m.module.name == use_decl.module_name)
        })
        .collect()
}

/// Helper to build an audit report from a file.
fn build_audit(source: &str, file: &Path) -> intent_ir::AuditReport {
    let ast = parse_or_exit(source, file);
    let ir = intent_ir::lower_file(&ast);
    let errors = intent_ir::verify_module(&ir);
    let obligations = intent_ir::analyze_obligations(&ir);
    intent_ir::generate_audit(source, &ir, &errors, &obligations)
}

fn json_out(value: &impl Serialize) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("JSON serialization failed")
    );
}

fn main() {
    let cli = Cli::parse();
    let json = matches!(cli.output, OutputFormat::Json);

    match cli.command {
        Commands::Check { file } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);

            let errors = if ast.imports.is_empty() {
                intent_check::check_file(&ast)
            } else {
                let graph = resolve_or_exit(&file);
                let root_file = &graph.modules[&graph.root];
                let imported = imported_files_for(root_file, &graph);
                intent_check::check_file_with_imports(root_file, &imported)
            };
            if json {
                json_out(&CheckResult {
                    ok: errors.is_empty(),
                    module: ast.module.name.clone(),
                    items: ast.items.len(),
                    errors: errors.iter().map(|e| format!("{e}")).collect(),
                });
                if !errors.is_empty() {
                    process::exit(1);
                }
            } else if errors.is_empty() {
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

            // Run semantic checks (with imports if present)
            let check_errors = if ast.imports.is_empty() {
                intent_check::check_file(&ast)
            } else {
                let graph = resolve_or_exit(&file);
                let root_file = &graph.modules[&graph.root];
                let imported = imported_files_for(root_file, &graph);
                intent_check::check_file_with_imports(root_file, &imported)
            };
            if !check_errors.is_empty() {
                let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
                for err in &check_errors {
                    let mut buf = String::new();
                    let report = miette::Report::new(err.clone()).with_source_code(source.clone());
                    handler.render_report(&mut buf, report.as_ref()).ok();
                    eprint!("{buf}");
                }
                eprintln!(
                    "{} error(s) in {} — fix before compiling",
                    check_errors.len(),
                    file.display()
                );
                process::exit(1);
            }

            let ir = intent_ir::lower_file(&ast);
            json_out(&ir);
        }
        Commands::Verify { file, incremental } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);

            // Run semantic checks first (with imports if present)
            let check_errors = if ast.imports.is_empty() {
                intent_check::check_file(&ast)
            } else {
                let graph = resolve_or_exit(&file);
                let root_file = &graph.modules[&graph.root];
                let imported = imported_files_for(root_file, &graph);
                intent_check::check_file_with_imports(root_file, &imported)
            };
            if !check_errors.is_empty() {
                if json {
                    json_out(&VerifyResult {
                        ok: false,
                        module: ast.module.name.clone(),
                        errors: check_errors.iter().map(|e| format!("{e}")).collect(),
                        obligations: vec![],
                        incremental: None,
                    });
                } else {
                    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
                    for err in &check_errors {
                        let mut buf = String::new();
                        let report =
                            miette::Report::new(err.clone()).with_source_code(source.clone());
                        handler.render_report(&mut buf, report.as_ref()).ok();
                        eprint!("{buf}");
                    }
                    eprintln!(
                        "{} semantic error(s) in {}",
                        check_errors.len(),
                        file.display()
                    );
                }
                process::exit(1);
            }

            // Lower to IR
            let ir = intent_ir::lower_file(&ast);

            if incremental {
                // Incremental verification with cache.
                let cache_path = cache_path_for(&file);
                let cache = load_cache(&cache_path);
                let result = intent_ir::incremental_verify(&ir, cache.as_ref());

                // Save updated cache.
                save_cache(&cache_path, &result.cache);

                if json {
                    json_out(&VerifyResult {
                        ok: result.errors.is_empty(),
                        module: ir.name.clone(),
                        errors: result.errors.iter().map(|e| format!("{e}")).collect(),
                        obligations: result.obligations.iter().map(|o| format!("{o}")).collect(),
                        incremental: Some(result.stats),
                    });
                    if !result.errors.is_empty() {
                        process::exit(1);
                    }
                } else if result.errors.is_empty() {
                    println!(
                        "VERIFIED: {} — {} function(s), {} invariant(s), {} struct(s)",
                        ir.name,
                        ir.functions.len(),
                        ir.invariants.len(),
                        ir.structs.len(),
                    );
                    println!(
                        "  (incremental: {} re-verified, {} cached, {} total)",
                        result.stats.reverified, result.stats.cached, result.stats.total_items,
                    );
                    if !result.obligations.is_empty() {
                        println!("\nVerification obligations:");
                        for ob in &result.obligations {
                            println!("  - {ob}");
                        }
                    }
                } else {
                    for err in &result.errors {
                        eprintln!(
                            "verify: {} (in {}.{}:{})",
                            err, err.trace.module, err.trace.item, err.trace.part
                        );
                    }
                    eprintln!(
                        "{} verification error(s) in {}",
                        result.errors.len(),
                        file.display()
                    );
                    process::exit(1);
                }
            } else {
                // Full verification (no cache).
                let ir_errors = intent_ir::verify_module(&ir);
                let obligations = intent_ir::analyze_obligations(&ir);

                if json {
                    json_out(&VerifyResult {
                        ok: ir_errors.is_empty(),
                        module: ir.name.clone(),
                        errors: ir_errors.iter().map(|e| format!("{e}")).collect(),
                        obligations: obligations.iter().map(|o| format!("{o}")).collect(),
                        incremental: None,
                    });
                    if !ir_errors.is_empty() {
                        process::exit(1);
                    }
                } else if ir_errors.is_empty() {
                    println!(
                        "VERIFIED: {} — {} function(s), {} invariant(s), {} struct(s)",
                        ir.name,
                        ir.functions.len(),
                        ir.invariants.len(),
                        ir.structs.len(),
                    );
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
        }
        Commands::Audit { file } => {
            let source = read_source(&file);
            let report = build_audit(&source, &file);
            if json {
                json_out(&report);
            } else {
                print!("{}", report.format_trace_map());
            }
        }
        Commands::Coverage { file } => {
            let source = read_source(&file);
            let report = build_audit(&source, &file);
            if json {
                json_out(&report.summary);
            } else {
                print!("{}", report.format_coverage());
            }
        }
        Commands::Diff { old, new } => {
            let old_source = read_source(&old);
            let old_report = build_audit(&old_source, &old);

            let new_source = read_source(&new);
            let new_report = build_audit(&new_source, &new);

            let diff = intent_ir::diff_reports(&old_report, &new_report);
            if json {
                json_out(&diff);
            } else {
                print!("{}", diff.format());
            }
        }
        Commands::Query { file, target } => {
            let source = read_source(&file);
            let report = build_audit(&source, &file);

            match target.as_str() {
                "entities" => {
                    let items: Vec<_> = report
                        .entries
                        .iter()
                        .filter(|e| e.kind == intent_ir::SpecItemKind::Entity)
                        .collect();
                    if json {
                        json_out(&items);
                    } else {
                        for item in &items {
                            println!("{} [L{}]", item.name, item.line);
                            for part in &item.parts {
                                println!("  {}: {}", part.label, part.ir_desc);
                            }
                        }
                    }
                }
                "actions" => {
                    let items: Vec<_> = report
                        .entries
                        .iter()
                        .filter(|e| e.kind == intent_ir::SpecItemKind::Action)
                        .collect();
                    if json {
                        json_out(&items);
                    } else {
                        for item in &items {
                            println!("{} [L{}]", item.name, item.line);
                            for part in &item.parts {
                                println!("  {}: {}", part.label, part.ir_desc);
                            }
                        }
                    }
                }
                "invariants" => {
                    let items: Vec<_> = report
                        .entries
                        .iter()
                        .filter(|e| e.kind == intent_ir::SpecItemKind::Invariant)
                        .collect();
                    if json {
                        json_out(&items);
                    } else {
                        for item in &items {
                            println!("{} [L{}]", item.name, item.line);
                        }
                    }
                }
                "edge-cases" => {
                    let items: Vec<_> = report
                        .entries
                        .iter()
                        .filter(|e| e.kind == intent_ir::SpecItemKind::EdgeCases)
                        .collect();
                    if json {
                        json_out(&items);
                    } else {
                        for item in &items {
                            for part in &item.parts {
                                println!("{}: {}", part.label, part.ir_desc);
                            }
                        }
                    }
                }
                "obligations" => {
                    if json {
                        json_out(&report.obligations);
                    } else {
                        if report.obligations.is_empty() {
                            println!("No obligations.");
                        } else {
                            for ob in &report.obligations {
                                println!("- {ob}");
                            }
                        }
                    }
                }
                "summary" => {
                    if json {
                        json_out(&report.summary);
                    } else {
                        print!("{}", report.format_coverage());
                    }
                }
                // Query by name — find any entry matching the target name.
                name => {
                    let items: Vec<_> = report.entries.iter().filter(|e| e.name == name).collect();
                    if items.is_empty() {
                        if json {
                            json_out(&serde_json::Value::Array(vec![]));
                        } else {
                            eprintln!("No item named '{}' found.", name);
                            process::exit(1);
                        }
                    } else if json {
                        json_out(&items);
                    } else {
                        for item in &items {
                            println!("{} {} [L{}]", item.kind, item.name, item.line);
                            for part in &item.parts {
                                println!("  {}: {}", part.label, part.ir_desc);
                            }
                            if !item.related_obligations.is_empty() {
                                println!("  Obligations:");
                                for ob in &item.related_obligations {
                                    println!("    - {ob}");
                                }
                            }
                        }
                    }
                }
            }
        }
        Commands::Lock { file, item, agent } => {
            let source = read_source(&file);
            let report = build_audit(&source, &file);
            let spec_items = intent_ir::extract_spec_items(&report);

            let lock_path = lock_path_for(&file);
            let mut lockfile = load_lockfile(&lock_path).unwrap_or(intent_ir::LockFile {
                module: report.module_name.clone(),
                claims: Default::default(),
            });

            let now = chrono_now();
            match intent_ir::lock_item(&mut lockfile, &spec_items, &item, &agent, &now) {
                Ok(()) => {
                    save_lockfile(&lock_path, &lockfile);
                    if json {
                        json_out(&serde_json::json!({
                            "ok": true,
                            "item": item,
                            "agent": agent,
                            "action": "locked",
                        }));
                    } else {
                        println!("Locked '{}' for agent '{}'", item, agent);
                    }
                }
                Err(e) => {
                    if json {
                        json_out(&serde_json::json!({
                            "ok": false,
                            "error": format!("{e}"),
                        }));
                    } else {
                        eprintln!("error: {e}");
                    }
                    process::exit(1);
                }
            }
        }
        Commands::Unlock { file, item, agent } => {
            let lock_path = lock_path_for(&file);
            let mut lockfile = match load_lockfile(&lock_path) {
                Some(lf) => lf,
                None => {
                    if json {
                        json_out(&serde_json::json!({
                            "ok": false,
                            "error": format!("'{}' is not claimed", item),
                        }));
                    } else {
                        eprintln!("error: '{}' is not claimed", item);
                    }
                    process::exit(1);
                }
            };

            match intent_ir::unlock_item(&mut lockfile, &item, &agent) {
                Ok(()) => {
                    save_lockfile(&lock_path, &lockfile);
                    if json {
                        json_out(&serde_json::json!({
                            "ok": true,
                            "item": item,
                            "agent": agent,
                            "action": "unlocked",
                        }));
                    } else {
                        println!("Unlocked '{}' for agent '{}'", item, agent);
                    }
                }
                Err(e) => {
                    if json {
                        json_out(&serde_json::json!({
                            "ok": false,
                            "error": format!("{e}"),
                        }));
                    } else {
                        eprintln!("error: {e}");
                    }
                    process::exit(1);
                }
            }
        }
        Commands::Status { file } => {
            let source = read_source(&file);
            let report = build_audit(&source, &file);
            let spec_items = intent_ir::extract_spec_items(&report);

            let lock_path = lock_path_for(&file);
            let lockfile = load_lockfile(&lock_path).unwrap_or(intent_ir::LockFile {
                module: report.module_name.clone(),
                claims: Default::default(),
            });

            if json {
                json_out(&lockfile);
            } else {
                print!("{}", intent_ir::format_status(&lockfile, &spec_items));
            }
        }
        Commands::Fmt { file, write, check } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);
            let formatted = intent_render::format::format(&ast);

            if check {
                if source != formatted {
                    eprintln!("{} is not formatted", file.display());
                    process::exit(1);
                }
            } else if write {
                if source != formatted {
                    if let Err(e) = fs::write(&file, &formatted) {
                        eprintln!("error: could not write {}: {}", file.display(), e);
                        process::exit(1);
                    }
                    println!("Formatted {}", file.display());
                } else {
                    println!("{} already formatted", file.display());
                }
            } else {
                print!("{}", formatted);
            }
        }
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "intent", &mut std::io::stdout());
        }
        Commands::Generate {
            description,
            confidence,
            max_retries,
            model,
            out,
            edit,
            diff,
            debug,
        } => {
            let client = match intent_gen::LlmClient::from_env() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error: {e}");
                    eprintln!(
                        "hint: set AI_API_KEY (and optionally AI_API_BASE, AI_MODEL) environment variables"
                    );
                    process::exit(1);
                }
            };
            let client = if let Some(m) = model {
                client.with_model(m)
            } else {
                client
            };

            let mut options = intent_gen::GenerateOptions {
                max_retries,
                confidence,
                debug,
                ..Default::default()
            };

            if let Some(edit_path) = &edit {
                let existing = read_source(edit_path);
                options.existing_spec = Some(existing.clone());
                options.edit_instruction = Some(description.clone());

                match intent_gen::generate(&client, &description, &options) {
                    Ok(spec) => {
                        if diff {
                            print_diff(&existing, &spec);
                        } else if let Some(out_path) = out {
                            write_or_exit(&out_path, &spec);
                            println!("Generated spec written to {}", out_path.display());
                        } else {
                            // Write back to the edited file
                            write_or_exit(edit_path, &spec);
                            println!("Updated {}", edit_path.display());
                        }
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        process::exit(1);
                    }
                }
            } else {
                match intent_gen::generate(&client, &description, &options) {
                    Ok(spec) => {
                        if let Some(out_path) = out {
                            write_or_exit(&out_path, &spec);
                            println!("Generated spec written to {}", out_path.display());
                        } else {
                            print!("{}", spec);
                        }
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        process::exit(1);
                    }
                }
            }
        }
        Commands::Serve { file, addr } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);

            let check_errors = if ast.imports.is_empty() {
                intent_check::check_file(&ast)
            } else {
                let graph = resolve_or_exit(&file);
                let root_file = &graph.modules[&graph.root];
                let imported = imported_files_for(root_file, &graph);
                intent_check::check_file_with_imports(root_file, &imported)
            };
            if !check_errors.is_empty() {
                let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
                for err in &check_errors {
                    let mut buf = String::new();
                    let report = miette::Report::new(err.clone()).with_source_code(source.clone());
                    handler.render_report(&mut buf, report.as_ref()).ok();
                    eprint!("{buf}");
                }
                eprintln!(
                    "{} error(s) in {} — fix before serving",
                    check_errors.len(),
                    file.display()
                );
                process::exit(1);
            }

            let ir = intent_ir::lower_file(&ast);
            if let Err(e) = intent_runtime::serve(ir, &addr) {
                eprintln!("error: {e}");
                process::exit(1);
            }
        }
        Commands::Test { file, filter } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);
            let ir = intent_ir::lower_file(&ast);

            // Extract test declarations from AST.
            let tests: Vec<_> = ast
                .items
                .iter()
                .filter_map(|item| {
                    if let intent_parser::ast::TopLevelItem::Test(t) = item {
                        Some(t)
                    } else {
                        None
                    }
                })
                .filter(|t| filter.as_ref().is_none_or(|f| t.name.contains(f.as_str())))
                .collect();

            if tests.is_empty() {
                if filter.is_some() {
                    eprintln!("No tests matching filter in {}", file.display());
                } else {
                    eprintln!("No test blocks found in {}", file.display());
                }
                process::exit(1);
            }

            let results = intent_runtime::run_tests(&ir, &tests);
            let passed = results.iter().filter(|r| r.passed).count();
            let failed = results.iter().filter(|r| !r.passed).count();

            if json {
                json_out(&TestResultOutput {
                    total: results.len(),
                    passed,
                    failed,
                    results: results
                        .iter()
                        .map(|r| TestResultEntry {
                            name: r.name.clone(),
                            passed: r.passed,
                            message: r.message.clone(),
                        })
                        .collect(),
                });
                if failed > 0 {
                    process::exit(1);
                }
            } else {
                for r in &results {
                    if r.passed {
                        println!("  PASS  {}", r.name);
                    } else {
                        println!("  FAIL  {}", r.name);
                        if let Some(msg) = &r.message {
                            println!("        {msg}");
                        }
                    }
                }
                println!();
                if failed > 0 {
                    println!("{passed} passed, {failed} failed ({} total)", results.len());
                    process::exit(1);
                } else {
                    println!("{passed} passed ({} total)", results.len());
                }
            }
        }
        Commands::Codegen {
            file,
            lang,
            out_dir,
        } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);

            // Run semantic checks before generating
            let check_errors = if ast.imports.is_empty() {
                intent_check::check_file(&ast)
            } else {
                let graph = resolve_or_exit(&file);
                let root_file = &graph.modules[&graph.root];
                let imported = imported_files_for(root_file, &graph);
                intent_check::check_file_with_imports(root_file, &imported)
            };
            if !check_errors.is_empty() {
                let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
                for err in &check_errors {
                    let mut buf = String::new();
                    let report = miette::Report::new(err.clone()).with_source_code(source.clone());
                    handler.render_report(&mut buf, report.as_ref()).ok();
                    eprint!("{buf}");
                }
                eprintln!(
                    "{} error(s) in {} — fix before generating code",
                    check_errors.len(),
                    file.display()
                );
                process::exit(1);
            }

            let il_lang = match lang {
                CodegenLang::Rust => intent_codegen::Language::Rust,
                CodegenLang::Typescript => intent_codegen::Language::TypeScript,
                CodegenLang::Python => intent_codegen::Language::Python,
                CodegenLang::Go => intent_codegen::Language::Go,
                CodegenLang::Java => intent_codegen::Language::Java,
                CodegenLang::Csharp => intent_codegen::Language::CSharp,
                CodegenLang::Swift => intent_codegen::Language::Swift,
            };
            let code = intent_codegen::generate(&ast, il_lang);

            if let Some(out_dir) = out_dir {
                let filename = intent_codegen::output_filename(&ast.module.name, il_lang);
                let out_path = out_dir.join(filename);
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent).ok();
                }
                write_or_exit(&out_path, &code);
                println!("Generated {}", out_path.display());
            } else {
                print!("{}", code);
            }
        }
        Commands::Openapi { file, out } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);

            // Run semantic checks before generating
            let check_errors = if ast.imports.is_empty() {
                intent_check::check_file(&ast)
            } else {
                let graph = resolve_or_exit(&file);
                let root_file = &graph.modules[&graph.root];
                let imported = imported_files_for(root_file, &graph);
                intent_check::check_file_with_imports(root_file, &imported)
            };
            if !check_errors.is_empty() {
                let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
                for err in &check_errors {
                    let mut buf = String::new();
                    let report = miette::Report::new(err.clone()).with_source_code(source.clone());
                    handler.render_report(&mut buf, report.as_ref()).ok();
                    eprint!("{buf}");
                }
                eprintln!(
                    "{} error(s) in {} — fix before generating OpenAPI spec",
                    check_errors.len(),
                    file.display()
                );
                process::exit(1);
            }

            let spec = intent_codegen::openapi::generate(&ast);
            let json = serde_json::to_string_pretty(&spec).expect("JSON serialization failed");

            if let Some(out_path) = out {
                write_or_exit(&out_path, &json);
                println!("Generated OpenAPI spec: {}", out_path.display());
            } else {
                println!("{json}");
            }
        }
        Commands::Implement {
            file,
            lang,
            out_dir,
            model,
            max_retries,
            debug,
        } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);

            // Run semantic checks (with imports if present)
            let check_errors = if ast.imports.is_empty() {
                intent_check::check_file(&ast)
            } else {
                let graph = resolve_or_exit(&file);
                let root_file = &graph.modules[&graph.root];
                let imported = imported_files_for(root_file, &graph);
                intent_check::check_file_with_imports(root_file, &imported)
            };
            if !check_errors.is_empty() {
                let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
                for err in &check_errors {
                    let mut buf = String::new();
                    let report = miette::Report::new(err.clone()).with_source_code(source.clone());
                    handler.render_report(&mut buf, report.as_ref()).ok();
                    eprint!("{buf}");
                }
                eprintln!(
                    "{} error(s) in {} — fix before generating implementation",
                    check_errors.len(),
                    file.display()
                );
                process::exit(1);
            }

            let client = match intent_gen::LlmClient::from_env() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error: {e}");
                    eprintln!(
                        "hint: set AI_API_KEY (and optionally AI_API_BASE, AI_MODEL) environment variables"
                    );
                    process::exit(1);
                }
            };
            let client = if let Some(m) = model {
                client.with_model(m)
            } else {
                client
            };

            let il_lang = match lang {
                CodegenLang::Rust => intent_codegen::Language::Rust,
                CodegenLang::Typescript => intent_codegen::Language::TypeScript,
                CodegenLang::Python => intent_codegen::Language::Python,
                CodegenLang::Go => intent_codegen::Language::Go,
                CodegenLang::Java => intent_codegen::Language::Java,
                CodegenLang::Csharp => intent_codegen::Language::CSharp,
                CodegenLang::Swift => intent_codegen::Language::Swift,
            };

            let options = intent_implement::ImplementOptions {
                language: il_lang,
                max_retries,
                debug,
            };

            match intent_implement::implement(&client, &ast, &options) {
                Ok(code) => {
                    if let Some(out_dir) = out_dir {
                        let filename = intent_codegen::output_filename(&ast.module.name, il_lang);
                        let out_path = out_dir.join(filename);
                        if let Some(parent) = out_path.parent() {
                            fs::create_dir_all(parent).ok();
                        }
                        write_or_exit(&out_path, &code);
                        println!("Generated {}", out_path.display());
                    } else {
                        print!("{}", code);
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            }
        }
        Commands::TestHarness { file, lang, out } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);

            let il_lang = match lang {
                CodegenLang::Rust => intent_codegen::Language::Rust,
                _ => {
                    eprintln!(
                        "error: only Rust is currently supported for test harness generation"
                    );
                    process::exit(1);
                }
            };

            let harness = intent_codegen::test_harness::generate(&ast, il_lang);
            if harness.is_empty() {
                eprintln!("No test blocks found in {}", file.display());
                process::exit(0);
            }

            if let Some(out_path) = out {
                write_or_exit(&out_path, &harness);
                println!("Generated test harness: {}", out_path.display());
            } else {
                print!("{harness}");
            }
        }
        Commands::Init { name, out } => {
            let module_name = name.unwrap_or_else(|| {
                std::env::current_dir()
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| "MyModule".to_string())
            });
            // Capitalize first letter for module name convention
            let module_name = capitalize(&module_name);
            let file_path = out
                .unwrap_or_else(|| PathBuf::from(format!("{}.intent", module_name.to_lowercase())));

            if file_path.exists() {
                eprintln!("error: {} already exists", file_path.display());
                process::exit(1);
            }

            let content = generate_scaffold(&module_name);
            if let Err(e) = fs::write(&file_path, &content) {
                eprintln!("error: could not write {}: {}", file_path.display(), e);
                process::exit(1);
            }
            println!("Created {} (module {})", file_path.display(), module_name);
        }
        Commands::AddInvariant {
            file,
            pattern,
            entity,
            action,
            fields,
            dry_run,
        } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);

            // Validate the file first
            let check_errors = if ast.imports.is_empty() {
                intent_check::check_file(&ast)
            } else {
                let graph = resolve_or_exit(&file);
                let root_file = &graph.modules[&graph.root];
                let imported = imported_files_for(root_file, &graph);
                intent_check::check_file_with_imports(root_file, &imported)
            };
            if !check_errors.is_empty() {
                let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
                for err in &check_errors {
                    let mut buf = String::new();
                    let report = miette::Report::new(err.clone()).with_source_code(source.clone());
                    handler.render_report(&mut buf, report.as_ref()).ok();
                    eprint!("{buf}");
                }
                eprintln!(
                    "{} error(s) in {} — fix before adding invariants",
                    check_errors.len(),
                    file.display()
                );
                process::exit(1);
            }

            match pattern {
                InvariantPattern::Idempotent => {
                    let action_name = match &action {
                        Some(a) => a.clone(),
                        None => {
                            eprintln!("error: --action is required for the idempotent pattern");
                            process::exit(1);
                        }
                    };
                    handle_idempotent(&ast, &source, &file, &action_name, dry_run);
                }
                _ => {
                    let entity_name = match &entity {
                        Some(e) => e.clone(),
                        None => {
                            eprintln!(
                                "error: --entity is required for the {} pattern",
                                pattern_name(pattern)
                            );
                            process::exit(1);
                        }
                    };
                    handle_entity_invariant(
                        &ast,
                        &source,
                        &file,
                        pattern,
                        &entity_name,
                        &fields,
                        dry_run,
                    );
                }
            }
        }
    }
}

// ── JSON output types ─────────────────────────────────────

#[derive(Serialize)]
struct CheckResult {
    ok: bool,
    module: String,
    items: usize,
    errors: Vec<String>,
}

#[derive(Serialize)]
struct VerifyResult {
    ok: bool,
    module: String,
    errors: Vec<String>,
    obligations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    incremental: Option<intent_ir::IncrementalStats>,
}

#[derive(Serialize)]
struct TestResultOutput {
    total: usize,
    passed: usize,
    failed: usize,
    results: Vec<TestResultEntry>,
}

#[derive(Serialize)]
struct TestResultEntry {
    name: String,
    passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

// ── Cache helpers ─────────────────────────────────────────

fn cache_path_for(file: &Path) -> PathBuf {
    let parent = file.parent().unwrap_or(Path::new("."));
    let stem = file.file_stem().unwrap_or_default();
    let cache_dir = parent.join(".intent-cache");
    cache_dir.join(format!("{}.json", stem.to_string_lossy()))
}

fn load_cache(path: &Path) -> Option<intent_ir::VerifyCache> {
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_cache(path: &Path, cache: &intent_ir::VerifyCache) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        fs::write(path, json).ok();
    }
}

// ── Lock file helpers ─────────────────────────────────────

fn lock_path_for(file: &Path) -> PathBuf {
    let parent = file.parent().unwrap_or(Path::new("."));
    let stem = file.file_stem().unwrap_or_default();
    let lock_dir = parent.join(".intent-lock");
    lock_dir.join(format!("{}.json", stem.to_string_lossy()))
}

fn load_lockfile(path: &Path) -> Option<intent_ir::LockFile> {
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_lockfile(path: &Path, lockfile: &intent_ir::LockFile) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Ok(json) = serde_json::to_string_pretty(lockfile) {
        fs::write(path, json).ok();
    }
}

fn chrono_now() -> String {
    // Simple ISO 8601 timestamp without chrono dependency.
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}Z", dur.as_secs())
}

// ── Init helpers ─────────────────────────────────────────

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// ── Add-invariant helpers ─────────────────────────────────

/// Convert a snake_case or lowercase identifier to PascalCase.
/// e.g. "customer_id" -> "CustomerId", "balance" -> "Balance"
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|part| !part.is_empty())
        .map(capitalize)
        .collect()
}

fn pattern_name(p: InvariantPattern) -> &'static str {
    match p {
        InvariantPattern::Unique => "unique",
        InvariantPattern::NonNegative => "non-negative",
        InvariantPattern::NoDanglingRef => "no-dangling-ref",
        InvariantPattern::Idempotent => "idempotent",
    }
}

/// Find an entity declaration by name in the AST.
fn find_entity<'a>(
    ast: &'a intent_parser::ast::File,
    name: &str,
) -> Option<&'a intent_parser::ast::EntityDecl> {
    ast.items.iter().find_map(|item| {
        if let intent_parser::ast::TopLevelItem::Entity(e) = item
            && e.name == name
        {
            return Some(e);
        }
        None
    })
}

/// Find an action declaration by name in the AST.
fn find_action<'a>(
    ast: &'a intent_parser::ast::File,
    name: &str,
) -> Option<&'a intent_parser::ast::ActionDecl> {
    ast.items.iter().find_map(|item| {
        if let intent_parser::ast::TopLevelItem::Action(a) = item
            && a.name == name
        {
            return Some(a);
        }
        None
    })
}

/// Check whether an invariant name already exists in the AST.
fn invariant_exists(ast: &intent_parser::ast::File, name: &str) -> bool {
    ast.items.iter().any(|item| {
        if let intent_parser::ast::TopLevelItem::Invariant(inv) = item {
            inv.name == name
        } else {
            false
        }
    })
}

/// Check whether an entity has a field with the given name.
fn entity_has_field(entity: &intent_parser::ast::EntityDecl, field: &str) -> bool {
    entity.fields.iter().any(|f| f.name == field)
}

/// Generate invariant text for entity-based patterns.
fn generate_invariant_text(
    pattern: InvariantPattern,
    entity_name: &str,
    fields: &[String],
) -> Result<(String, String), String> {
    match pattern {
        InvariantPattern::Unique => {
            if fields.len() != 1 {
                return Err(format!(
                    "unique pattern requires exactly 1 field argument, got {}",
                    fields.len()
                ));
            }
            let field = &fields[0];
            let inv_name = format!("Unique{}{}", entity_name, to_pascal_case(field));
            let text = format!(
                "\ninvariant {} {{\n  forall a: {} => forall b: {} =>\n    a != b => a.{} != b.{}\n}}\n",
                inv_name, entity_name, entity_name, field, field
            );
            Ok((inv_name, text))
        }
        InvariantPattern::NonNegative => {
            if fields.len() != 1 {
                return Err(format!(
                    "non-negative pattern requires exactly 1 field argument, got {}",
                    fields.len()
                ));
            }
            let field = &fields[0];
            let inv_name = format!("NonNegative{}{}", entity_name, to_pascal_case(field));
            let text = format!(
                "\ninvariant {} {{\n  forall a: {} => a.{} >= 0\n}}\n",
                inv_name, entity_name, field
            );
            Ok((inv_name, text))
        }
        InvariantPattern::NoDanglingRef => {
            if fields.len() != 3 {
                return Err(format!(
                    "no-dangling-ref pattern requires 3 field arguments (field RefEntity ref_field), got {}",
                    fields.len()
                ));
            }
            let field = &fields[0];
            let ref_entity = &fields[1];
            let ref_field = &fields[2];
            let inv_name = format!("NoDangling{}{}", entity_name, to_pascal_case(field));
            let text = format!(
                "\ninvariant {} {{\n  forall a: {} => exists b: {} => a.{} == b.{}\n}}\n",
                inv_name, entity_name, ref_entity, field, ref_field
            );
            Ok((inv_name, text))
        }
        InvariantPattern::Idempotent => {
            // Handled separately
            unreachable!()
        }
    }
}

/// Handle entity-based invariant patterns (unique, non-negative, no-dangling-ref).
fn handle_entity_invariant(
    ast: &intent_parser::ast::File,
    source: &str,
    file: &Path,
    pattern: InvariantPattern,
    entity_name: &str,
    fields: &[String],
    dry_run: bool,
) {
    // Validate entity exists
    let entity = match find_entity(ast, entity_name) {
        Some(e) => e,
        None => {
            eprintln!(
                "error: entity '{}' not found in {}",
                entity_name,
                file.display()
            );
            process::exit(1);
        }
    };

    // Validate field count first
    let (inv_name, inv_text) = match generate_invariant_text(pattern, entity_name, fields) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    // Validate fields exist on entity
    if matches!(
        pattern,
        InvariantPattern::Unique | InvariantPattern::NonNegative | InvariantPattern::NoDanglingRef
    ) && !entity_has_field(entity, &fields[0])
    {
        eprintln!(
            "error: entity '{}' has no field '{}'",
            entity_name, fields[0]
        );
        process::exit(1);
    }
    if matches!(pattern, InvariantPattern::NoDanglingRef) {
        // Validate referenced entity and field
        let ref_entity_name = &fields[1];
        let ref_entity = match find_entity(ast, ref_entity_name) {
            Some(e) => e,
            None => {
                eprintln!(
                    "error: referenced entity '{}' not found in {}",
                    ref_entity_name,
                    file.display()
                );
                process::exit(1);
            }
        };
        if !entity_has_field(ref_entity, &fields[2]) {
            eprintln!(
                "error: entity '{}' has no field '{}'",
                ref_entity_name, fields[2]
            );
            process::exit(1);
        }
    }

    // Check for duplicate invariant name
    if invariant_exists(ast, &inv_name) {
        eprintln!(
            "error: invariant '{}' already exists in {}",
            inv_name,
            file.display()
        );
        process::exit(1);
    }

    if dry_run {
        print!("{inv_text}");
    } else {
        // Append to file (strip trailing whitespace, then append)
        let trimmed = source.trim_end();
        let new_content = format!("{trimmed}{inv_text}");
        write_or_exit(file, &new_content);
        println!("Added invariant '{}' to {}", inv_name, file.display());
    }
}

/// Handle the idempotent pattern (action property, not entity invariant).
fn handle_idempotent(
    ast: &intent_parser::ast::File,
    source: &str,
    file: &Path,
    action_name: &str,
    dry_run: bool,
) {
    // Validate action exists
    let action = match find_action(ast, action_name) {
        Some(a) => a,
        None => {
            eprintln!(
                "error: action '{}' not found in {}",
                action_name,
                file.display()
            );
            process::exit(1);
        }
    };

    // Check if action already has idempotent property
    if let Some(props) = &action.properties
        && props.entries.iter().any(|e| e.key == "idempotent")
    {
        eprintln!(
            "error: action '{}' already has an 'idempotent' property",
            action_name
        );
        process::exit(1);
    }

    let prop_line = "    idempotent: true";

    if dry_run {
        println!("properties {{");
        println!("{prop_line}");
        println!("}}");
    } else {
        // We need to modify the file to add the property.
        // If the action already has a properties block, insert the property.
        // If not, add a properties block before the action's closing brace.
        let new_source = if let Some(props) = &action.properties {
            // Insert after the opening `properties {` line.
            // The properties block span starts at `properties` keyword.
            // Find the `{` after `properties` in the source.
            let props_start = props.span.start;
            let after_props = &source[props_start..];
            let brace_offset = after_props.find('{').unwrap() + 1;
            let insert_pos = props_start + brace_offset;
            format!(
                "{}\n{}{}",
                &source[..insert_pos],
                prop_line,
                &source[insert_pos..]
            )
        } else {
            // Add a new properties block before the action's closing `}`.
            // The action span ends at the closing `}`.
            let action_end = action.span.end;
            // Find the last `}` in the action span.
            let before_end = &source[..action_end];
            let last_brace = before_end.rfind('}').unwrap();
            format!(
                "{}\n  properties {{\n{}\n  }}\n{}",
                &source[..last_brace],
                prop_line,
                &source[last_brace..]
            )
        };

        write_or_exit(file, &new_source);
        println!(
            "Added idempotent property to action '{}' in {}",
            action_name,
            file.display()
        );
    }
}

fn write_or_exit(path: &Path, content: &str) {
    if let Err(e) = fs::write(path, content) {
        eprintln!("error: could not write {}: {}", path.display(), e);
        process::exit(1);
    }
}

fn print_diff(old: &str, new: &str) {
    use similar::{ChangeTag, TextDiff};
    let diff = TextDiff::from_lines(old, new);
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => print!("-{change}"),
            ChangeTag::Insert => print!("+{change}"),
            ChangeTag::Equal => print!(" {change}"),
        }
    }
}

fn generate_scaffold(module_name: &str) -> String {
    format!(
        r#"module {module_name}

--- TODO: Describe what this module specifies.

entity Example {{
  id: UUID
  name: String
  status: Active | Inactive
}}

action CreateExample {{
  name: String

  requires {{
    name != ""
  }}

  ensures {{
    exists e: Example => e.name == name
  }}
}}

invariant UniqueNames {{
  forall a: Example => forall b: Example =>
    a.id != b.id => a.name != b.name
}}
"#
    )
}
