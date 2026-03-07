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
    /// Initialize a new .intent spec file
    Init {
        /// Module name (defaults to directory name)
        #[arg(long)]
        name: Option<String>,
        /// Output file path (defaults to <name>.intent)
        #[arg(short = 'o', long = "out")]
        out: Option<PathBuf>,
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

            let errors = intent_check::check_file(&ast);
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
            let ir = intent_ir::lower_file(&ast);
            json_out(&ir);
        }
        Commands::Verify { file, incremental } => {
            let source = read_source(&file);
            let ast = parse_or_exit(&source, &file);

            // Run semantic checks first
            let check_errors = intent_check::check_file(&ast);
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
