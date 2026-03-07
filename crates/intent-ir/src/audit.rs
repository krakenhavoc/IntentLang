//! Audit Bridge — trace maps and coverage analysis.
//!
//! Cross-references IR constructs with their originating spec items
//! via `SourceTrace` to produce:
//! - Trace maps: which IR constructs implement each spec requirement
//! - Coverage summaries: how much of the spec is compiled and verified

use std::fmt;

use crate::types::*;
use crate::verify::Obligation;

/// A complete audit report for a module.
#[derive(Debug, Clone)]
pub struct AuditReport {
    pub module_name: String,
    pub entries: Vec<AuditEntry>,
    pub summary: CoverageSummary,
    pub verify_error_count: usize,
    pub obligations: Vec<Obligation>,
}

/// One spec-level item and its IR trace.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub kind: SpecItemKind,
    pub name: String,
    pub line: usize,
    pub parts: Vec<TracePart>,
    /// Obligations that reference this item.
    pub related_obligations: Vec<String>,
}

/// What kind of spec item this entry represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecItemKind {
    Entity,
    Action,
    Invariant,
    EdgeCases,
}

/// A single spec sub-item mapped to IR.
#[derive(Debug, Clone)]
pub struct TracePart {
    /// Spec-level label (e.g., "field:balance", "requires[0]", "property:atomic").
    pub label: String,
    /// IR-level description.
    pub ir_desc: String,
    /// 1-based source line number.
    pub line: usize,
}

/// Coverage statistics for the module.
#[derive(Debug, Clone)]
pub struct CoverageSummary {
    pub entities: usize,
    pub fields: usize,
    pub actions: usize,
    pub params: usize,
    pub preconditions: usize,
    pub postconditions: usize,
    pub properties: usize,
    pub invariants: usize,
    pub edge_guards: usize,
}

impl CoverageSummary {
    pub fn total(&self) -> usize {
        self.entities
            + self.fields
            + self.actions
            + self.params
            + self.preconditions
            + self.postconditions
            + self.properties
            + self.invariants
            + self.edge_guards
    }
}

/// Generate an audit report by walking the IR module and cross-referencing
/// with verification results.
pub fn generate_audit(
    source: &str,
    module: &Module,
    verify_errors: &[crate::verify::VerifyError],
    obligations: &[Obligation],
) -> AuditReport {
    let line_index = build_line_index(source);
    let mut entries = Vec::new();

    // Structs (from entities)
    for s in &module.structs {
        let mut parts = Vec::new();
        for f in &s.fields {
            parts.push(TracePart {
                label: format!("field:{}", f.name),
                ir_desc: format!("{}: {}", f.name, format_ir_type(&f.ty)),
                line: offset_to_line(&line_index, f.trace.span.start),
            });
        }
        entries.push(AuditEntry {
            kind: SpecItemKind::Entity,
            name: s.name.clone(),
            line: offset_to_line(&line_index, s.trace.span.start),
            parts,
            related_obligations: vec![],
        });
    }

    // Functions (from actions)
    for func in &module.functions {
        let mut parts = Vec::new();
        for p in &func.params {
            parts.push(TracePart {
                label: format!("param:{}", p.name),
                ir_desc: format!("{}: {}", p.name, format_ir_type(&p.ty)),
                line: offset_to_line(&line_index, p.trace.span.start),
            });
        }
        for (i, pre) in func.preconditions.iter().enumerate() {
            parts.push(TracePart {
                label: format!("requires[{}]", i),
                ir_desc: "precondition".into(),
                line: offset_to_line(&line_index, pre.trace.span.start),
            });
        }
        for (i, post) in func.postconditions.iter().enumerate() {
            let (trace, desc) = match post {
                Postcondition::Always { trace, .. } => (trace, "postcondition"),
                Postcondition::When { trace, .. } => (trace, "conditional postcondition"),
            };
            parts.push(TracePart {
                label: format!("ensures[{}]", i),
                ir_desc: desc.into(),
                line: offset_to_line(&line_index, trace.span.start),
            });
        }
        for prop in &func.properties {
            parts.push(TracePart {
                label: format!("property:{}", prop.key),
                ir_desc: format!("{}: {}", prop.key, format_prop_value(&prop.value)),
                line: offset_to_line(&line_index, prop.trace.span.start),
            });
        }

        let related: Vec<String> = obligations
            .iter()
            .filter(|o| o.action == func.name)
            .map(|o| o.to_string())
            .collect();

        entries.push(AuditEntry {
            kind: SpecItemKind::Action,
            name: func.name.clone(),
            line: offset_to_line(&line_index, func.trace.span.start),
            parts,
            related_obligations: related,
        });
    }

    // Invariants
    for inv in &module.invariants {
        let related: Vec<String> = obligations
            .iter()
            .filter(|o| o.invariant == inv.name)
            .map(|o| o.to_string())
            .collect();

        entries.push(AuditEntry {
            kind: SpecItemKind::Invariant,
            name: inv.name.clone(),
            line: offset_to_line(&line_index, inv.trace.span.start),
            parts: vec![],
            related_obligations: related,
        });
    }

    // Edge guards
    if !module.edge_guards.is_empty() {
        let first_line = offset_to_line(&line_index, module.edge_guards[0].trace.span.start);
        let mut parts = Vec::new();
        for (i, guard) in module.edge_guards.iter().enumerate() {
            parts.push(TracePart {
                label: format!("guard[{}]", i),
                ir_desc: format!("when ... => {}", guard.action),
                line: offset_to_line(&line_index, guard.trace.span.start),
            });
        }
        entries.push(AuditEntry {
            kind: SpecItemKind::EdgeCases,
            name: "edge_cases".into(),
            line: first_line,
            parts,
            related_obligations: vec![],
        });
    }

    let summary = CoverageSummary {
        entities: module.structs.len(),
        fields: module.structs.iter().map(|s| s.fields.len()).sum(),
        actions: module.functions.len(),
        params: module.functions.iter().map(|f| f.params.len()).sum(),
        preconditions: module.functions.iter().map(|f| f.preconditions.len()).sum(),
        postconditions: module
            .functions
            .iter()
            .map(|f| f.postconditions.len())
            .sum(),
        properties: module.functions.iter().map(|f| f.properties.len()).sum(),
        invariants: module.invariants.len(),
        edge_guards: module.edge_guards.len(),
    };

    AuditReport {
        module_name: module.name.clone(),
        entries,
        summary,
        verify_error_count: verify_errors.len(),
        obligations: obligations.to_vec(),
    }
}

// ── Display implementations ────────────────────────────────

impl fmt::Display for SpecItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecItemKind::Entity => write!(f, "Entity"),
            SpecItemKind::Action => write!(f, "Action"),
            SpecItemKind::Invariant => write!(f, "Invariant"),
            SpecItemKind::EdgeCases => write!(f, "EdgeCases"),
        }
    }
}

impl AuditReport {
    /// Format as a trace map (for `intent audit`).
    pub fn format_trace_map(&self) -> String {
        let mut out = format!("{} — Audit Trace Map\n", self.module_name);

        for entry in &self.entries {
            out.push('\n');
            let ir_kind = match entry.kind {
                SpecItemKind::Entity => "Struct",
                SpecItemKind::Action => "Function",
                SpecItemKind::Invariant => "Invariant",
                SpecItemKind::EdgeCases => "Guards",
            };

            if entry.kind == SpecItemKind::EdgeCases {
                out.push_str(&format!("  EdgeCases -> {ir_kind}\n"));
            } else {
                out.push_str(&format!("  {} {} -> {ir_kind}", entry.kind, entry.name,));
                out.push_str(&format!("  [L{}]\n", entry.line));
            }

            let total = entry.parts.len() + entry.related_obligations.len();
            for (i, part) in entry.parts.iter().enumerate() {
                let is_last = i == total - 1;
                let prefix = if is_last { "└──" } else { "├──" };
                out.push_str(&format!(
                    "    {prefix} {}: {}  [L{}]\n",
                    part.label, part.ir_desc, part.line,
                ));
            }

            for (i, ob) in entry.related_obligations.iter().enumerate() {
                let is_last = entry.parts.len() + i == total - 1;
                let prefix = if is_last { "└──" } else { "├──" };
                out.push_str(&format!("    {prefix} obligation: {ob}\n"));
            }
        }

        out.push_str(&format!(
            "\nCoverage: {total} spec items compiled",
            total = self.summary.total(),
        ));
        if self.verify_error_count == 0 {
            out.push_str(", verification clean\n");
        } else {
            out.push_str(&format!(
                ", {} verification error(s)\n",
                self.verify_error_count
            ));
        }
        if !self.obligations.is_empty() {
            out.push_str(&format!(
                "Obligations: {} identified\n",
                self.obligations.len()
            ));
        }

        out
    }

    /// Format as a coverage summary (for `intent coverage`).
    pub fn format_coverage(&self) -> String {
        let s = &self.summary;
        let mut out = format!("{} — Coverage Summary\n\n", self.module_name);

        out.push_str(&format!(
            "  Entities:       {:>3} ({} fields)\n",
            s.entities, s.fields,
        ));
        out.push_str(&format!(
            "  Actions:        {:>3} ({} params, {} requires, {} ensures, {} properties)\n",
            s.actions, s.params, s.preconditions, s.postconditions, s.properties,
        ));
        out.push_str(&format!("  Invariants:     {:>3}\n", s.invariants));
        out.push_str(&format!("  Edge guards:    {:>3}\n", s.edge_guards));
        out.push_str("  ─────────────────────────────\n");
        out.push_str(&format!(
            "  Total:          {:>3} spec items compiled\n",
            s.total(),
        ));

        out.push('\n');
        if self.verify_error_count == 0 {
            out.push_str("  Verification:   clean (0 errors)\n");
        } else {
            out.push_str(&format!(
                "  Verification:   {} error(s)\n",
                self.verify_error_count,
            ));
        }

        if self.obligations.is_empty() {
            out.push_str("  Obligations:    none\n");
        } else {
            out.push_str(&format!(
                "  Obligations:    {} identified\n",
                self.obligations.len(),
            ));
            for ob in &self.obligations {
                out.push_str(&format!("    - {ob}\n"));
            }
        }

        out
    }
}

// ── Helpers ────────────────────────────────────────────────

/// Build a line-start index: `line_starts[i]` is the byte offset where line `i+1` begins.
fn build_line_index(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, ch) in source.char_indices() {
        if ch == '\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Convert a byte offset to a 1-based line number.
fn offset_to_line(line_starts: &[usize], offset: usize) -> usize {
    match line_starts.binary_search(&offset) {
        Ok(i) => i + 1,
        Err(i) => i,
    }
}

fn format_ir_type(ty: &IrType) -> String {
    match ty {
        IrType::Named(n) => n.clone(),
        IrType::Struct(n) => n.clone(),
        IrType::List(inner) => format!("List<{}>", format_ir_type(inner)),
        IrType::Set(inner) => format!("Set<{}>", format_ir_type(inner)),
        IrType::Map(k, v) => {
            format!("Map<{}, {}>", format_ir_type(k), format_ir_type(v))
        }
        IrType::Optional(inner) => format!("{}?", format_ir_type(inner)),
        IrType::Union(variants) => variants.join(" | "),
        IrType::Decimal(p) => format!("Decimal({p})"),
    }
}

fn format_prop_value(v: &PropertyValue) -> String {
    match v {
        PropertyValue::Bool(b) => b.to_string(),
        PropertyValue::Int(n) => n.to_string(),
        PropertyValue::String(s) => format!("\"{s}\""),
        PropertyValue::Ident(i) => i.clone(),
    }
}
