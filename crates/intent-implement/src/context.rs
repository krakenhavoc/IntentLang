//! Build rich context from a parsed intent AST for the LLM prompt.
//!
//! Assembles three pieces: the canonical spec source, the skeleton code,
//! and a structured contracts summary (requires/ensures/invariants/edge_cases).

use intent_codegen::{Language, format_ensures_item, format_expr, format_prop_value};
use intent_parser::ast;

/// All the context the LLM needs to generate a full implementation.
pub struct PromptContext {
    /// Canonical `.intent` source (re-formatted).
    pub spec_source: String,
    /// Skeleton code in the target language.
    pub skeleton: String,
    /// Human-readable contracts summary.
    pub contracts: String,
    /// Generated contract test harness (empty if no test blocks in spec).
    pub test_harness: String,
}

/// Build the full prompt context from an AST and target language.
pub fn build_context(file: &ast::File, lang: Language) -> PromptContext {
    let spec_source = intent_render::format::format(file);
    let skeleton = intent_codegen::generate(file, lang);
    let contracts = build_contracts_summary(file);
    let test_harness = intent_codegen::test_harness::generate(file, lang);

    PromptContext {
        spec_source,
        skeleton,
        contracts,
        test_harness,
    }
}

/// Walk the AST and produce a structured contracts summary.
fn build_contracts_summary(file: &ast::File) -> String {
    let mut out = String::new();

    for item in &file.items {
        match item {
            ast::TopLevelItem::Action(action) => {
                out.push_str(&format!("### Action: {}\n", action.name));

                // Parameters
                if !action.params.is_empty() {
                    let params: Vec<String> = action
                        .params
                        .iter()
                        .map(|p| format!("{} ({})", p.name, format_type(&p.ty)))
                        .collect();
                    out.push_str(&format!("Parameters: {}\n", params.join(", ")));
                }

                // Preconditions
                if let Some(req) = &action.requires {
                    out.push_str("\nPreconditions:\n");
                    for cond in &req.conditions {
                        out.push_str(&format!("  - {}\n", format_expr(cond)));
                    }
                }

                // Postconditions
                if let Some(ens) = &action.ensures {
                    out.push_str("\nPostconditions:\n");
                    for item in &ens.items {
                        out.push_str(&format!("  - {}\n", format_ensures_item(item)));
                    }
                }

                // Properties
                if let Some(props) = &action.properties {
                    out.push_str("\nProperties:\n");
                    for entry in &props.entries {
                        out.push_str(&format!(
                            "  - {}: {}\n",
                            entry.key,
                            format_prop_value(&entry.value)
                        ));
                    }
                }

                out.push('\n');
            }
            ast::TopLevelItem::Invariant(inv) => {
                out.push_str(&format!("### Invariant: {}\n", inv.name));
                out.push_str(&format!("  {}\n\n", format_expr(&inv.body)));
            }
            ast::TopLevelItem::EdgeCases(ec) => {
                out.push_str("### Edge Cases\n");
                for rule in &ec.rules {
                    let args: Vec<String> = rule
                        .action
                        .args
                        .iter()
                        .map(|a| match a {
                            ast::CallArg::Named { key, value, .. } => {
                                format!("{}: {}", key, format_expr(value))
                            }
                            ast::CallArg::Positional(e) => format_expr(e),
                        })
                        .collect();
                    out.push_str(&format!(
                        "  - when {} => {}({})\n",
                        format_expr(&rule.condition),
                        rule.action.name,
                        args.join(", "),
                    ));
                }
                out.push('\n');
            }
            ast::TopLevelItem::Entity(_)
            | ast::TopLevelItem::StateMachine(_)
            | ast::TopLevelItem::Test(_) => {
                // Entities/state machines are fully covered by the skeleton code; tests are not relevant
            }
        }
    }

    out
}

/// Format a type for human-readable display.
fn format_type(ty: &ast::TypeExpr) -> String {
    intent_render::format_type(ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ast::File {
        intent_parser::parse_file(src).expect("parse failed")
    }

    #[test]
    fn test_build_context_has_all_parts() {
        let src = "module Test\n\nentity Foo {\n  id: UUID\n  name: String\n}\n\naction CreateFoo {\n  name: String\n\n  requires {\n    name != \"\"\n  }\n\n  ensures {\n    exists f: Foo => f.name == name\n  }\n}\n";
        let ast = parse(src);
        let ctx = build_context(&ast, Language::Rust);

        assert!(ctx.spec_source.contains("module Test"));
        assert!(ctx.skeleton.contains("struct Foo"));
        assert!(ctx.contracts.contains("Action: CreateFoo"));
        assert!(ctx.contracts.contains("name != \"\""));
        // No test blocks in this spec, so harness is empty
        assert!(ctx.test_harness.is_empty());
    }

    #[test]
    fn test_build_context_includes_test_harness() {
        let src = "module Test\n\nentity Foo {\n  id: UUID\n}\n\naction CreateFoo {\n  name: String\n}\n\ntest \"creates foo\" {\n  given { n = \"hello\" }\n  when CreateFoo { name: n }\n  then fails\n}\n";
        let ast = parse(src);
        let ctx = build_context(&ast, Language::Rust);

        assert!(ctx.test_harness.contains("mod contract_tests"));
        assert!(ctx.test_harness.contains("test_creates_foo"));
    }

    #[test]
    fn test_contracts_summary_includes_invariant() {
        let src = "module Test\n\nentity Acc {\n  balance: Int\n}\n\ninvariant NoNeg {\n  forall a: Acc => a.balance >= 0\n}\n";
        let ast = parse(src);
        let summary = build_contracts_summary(&ast);

        assert!(summary.contains("Invariant: NoNeg"));
        assert!(summary.contains("forall a: Acc => a.balance >= 0"));
    }

    #[test]
    fn test_contracts_summary_includes_edge_cases() {
        let src = "module Test\n\nentity Acc {\n  id: UUID\n}\n\nedge_cases {\n  when amount > 10000 => require_approval(level: \"manager\")\n}\n";
        let ast = parse(src);
        let summary = build_contracts_summary(&ast);

        assert!(summary.contains("Edge Cases"));
        assert!(summary.contains("require_approval"));
    }

    #[test]
    fn test_contracts_summary_includes_properties() {
        let src = "module Test\n\nentity X {\n  id: UUID\n}\n\naction DoThing {\n  x: X\n\n  properties {\n    idempotent: true\n    atomic: true\n  }\n}\n";
        let ast = parse(src);
        let summary = build_contracts_summary(&ast);

        assert!(summary.contains("idempotent: true"));
        assert!(summary.contains("atomic: true"));
    }
}
