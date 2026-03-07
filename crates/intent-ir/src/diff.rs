//! Spec-level diff reporting.
//!
//! Compares two audit reports (old vs. new) and produces a structured diff
//! at the intent-specification level: which entities, actions, invariants,
//! and edge cases were added, removed, or modified, and what changed within them.

use std::collections::HashMap;
use std::fmt;

use serde::Serialize;

use crate::audit::{AuditEntry, AuditReport, CoverageSummary, SpecItemKind, TracePart};

/// A complete diff report between two versions of a spec.
#[derive(Debug, Clone, Serialize)]
pub struct DiffReport {
    pub module_name: String,
    pub changes: Vec<DiffEntry>,
    pub summary: DiffSummary,
}

/// A single spec-item change.
#[derive(Debug, Clone, Serialize)]
pub struct DiffEntry {
    pub kind: SpecItemKind,
    pub name: String,
    pub change: ChangeKind,
    /// Sub-item details (only present for Modified entries).
    pub details: Vec<DiffDetail>,
}

/// What happened to a spec item or sub-item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
}

/// A change to a sub-item within a spec entry.
#[derive(Debug, Clone, Serialize)]
pub struct DiffDetail {
    pub label: String,
    pub change: ChangeKind,
    pub old_desc: Option<String>,
    pub new_desc: Option<String>,
}

/// High-level summary of changes.
#[derive(Debug, Clone, Serialize)]
pub struct DiffSummary {
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
    pub old_coverage: CoverageSummary,
    pub new_coverage: CoverageSummary,
    pub old_verify_errors: usize,
    pub new_verify_errors: usize,
    pub old_obligations: usize,
    pub new_obligations: usize,
}

/// Compare two audit reports and produce a structured diff.
pub fn diff_reports(old: &AuditReport, new: &AuditReport) -> DiffReport {
    // Index old entries by (kind, name) for lookup.
    let old_map: HashMap<(SpecItemKind, &str), &AuditEntry> = old
        .entries
        .iter()
        .map(|e| ((e.kind, e.name.as_str()), e))
        .collect();

    let new_map: HashMap<(SpecItemKind, &str), &AuditEntry> = new
        .entries
        .iter()
        .map(|e| ((e.kind, e.name.as_str()), e))
        .collect();

    let mut changes = Vec::new();

    // Find removed entries (in old, not in new).
    for entry in &old.entries {
        let key = (entry.kind, entry.name.as_str());
        if !new_map.contains_key(&key) {
            changes.push(DiffEntry {
                kind: entry.kind,
                name: entry.name.clone(),
                change: ChangeKind::Removed,
                details: vec![],
            });
        }
    }

    // Find added and modified entries.
    for entry in &new.entries {
        let key = (entry.kind, entry.name.as_str());
        match old_map.get(&key) {
            None => {
                changes.push(DiffEntry {
                    kind: entry.kind,
                    name: entry.name.clone(),
                    change: ChangeKind::Added,
                    details: vec![],
                });
            }
            Some(old_entry) => {
                let details = diff_parts(&old_entry.parts, &entry.parts);
                if !details.is_empty() {
                    changes.push(DiffEntry {
                        kind: entry.kind,
                        name: entry.name.clone(),
                        change: ChangeKind::Modified,
                        details,
                    });
                }
            }
        }
    }

    let added = changes
        .iter()
        .filter(|c| c.change == ChangeKind::Added)
        .count();
    let removed = changes
        .iter()
        .filter(|c| c.change == ChangeKind::Removed)
        .count();
    let modified = changes
        .iter()
        .filter(|c| c.change == ChangeKind::Modified)
        .count();

    let summary = DiffSummary {
        added,
        removed,
        modified,
        old_coverage: old.summary.clone(),
        new_coverage: new.summary.clone(),
        old_verify_errors: old.verify_error_count,
        new_verify_errors: new.verify_error_count,
        old_obligations: old.obligations.len(),
        new_obligations: new.obligations.len(),
    };

    DiffReport {
        module_name: new.module_name.clone(),
        changes,
        summary,
    }
}

/// Compare the parts (sub-items) of two matched entries.
fn diff_parts(old_parts: &[TracePart], new_parts: &[TracePart]) -> Vec<DiffDetail> {
    let old_map: HashMap<&str, &TracePart> =
        old_parts.iter().map(|p| (p.label.as_str(), p)).collect();
    let new_map: HashMap<&str, &TracePart> =
        new_parts.iter().map(|p| (p.label.as_str(), p)).collect();

    let mut details = Vec::new();

    // Removed parts.
    for part in old_parts {
        if !new_map.contains_key(part.label.as_str()) {
            details.push(DiffDetail {
                label: part.label.clone(),
                change: ChangeKind::Removed,
                old_desc: Some(part.ir_desc.clone()),
                new_desc: None,
            });
        }
    }

    // Added and modified parts.
    for part in new_parts {
        match old_map.get(part.label.as_str()) {
            None => {
                details.push(DiffDetail {
                    label: part.label.clone(),
                    change: ChangeKind::Added,
                    old_desc: None,
                    new_desc: Some(part.ir_desc.clone()),
                });
            }
            Some(old_part) => {
                if old_part.ir_desc != part.ir_desc {
                    details.push(DiffDetail {
                        label: part.label.clone(),
                        change: ChangeKind::Modified,
                        old_desc: Some(old_part.ir_desc.clone()),
                        new_desc: Some(part.ir_desc.clone()),
                    });
                }
            }
        }
    }

    details
}

// ── Display implementations ────────────────────────────────

impl fmt::Display for ChangeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangeKind::Added => write!(f, "+"),
            ChangeKind::Removed => write!(f, "-"),
            ChangeKind::Modified => write!(f, "~"),
        }
    }
}

impl DiffReport {
    /// Format as a human-readable diff report.
    pub fn format(&self) -> String {
        let mut out = format!("{} — Spec Diff Report\n", self.module_name);

        if self.changes.is_empty() {
            out.push_str("\n  No spec-level changes detected.\n");
            return out;
        }

        for entry in &self.changes {
            out.push('\n');
            let symbol = entry.change;
            out.push_str(&format!("  [{symbol}] {} {}\n", entry.kind, entry.name,));

            for detail in &entry.details {
                let dsym = detail.change;
                match detail.change {
                    ChangeKind::Added => {
                        out.push_str(&format!(
                            "      [{dsym}] {}: {}\n",
                            detail.label,
                            detail.new_desc.as_deref().unwrap_or(""),
                        ));
                    }
                    ChangeKind::Removed => {
                        out.push_str(&format!(
                            "      [{dsym}] {}: {}\n",
                            detail.label,
                            detail.old_desc.as_deref().unwrap_or(""),
                        ));
                    }
                    ChangeKind::Modified => {
                        out.push_str(&format!(
                            "      [{dsym}] {}: {} -> {}\n",
                            detail.label,
                            detail.old_desc.as_deref().unwrap_or(""),
                            detail.new_desc.as_deref().unwrap_or(""),
                        ));
                    }
                }
            }
        }

        // Summary section.
        let s = &self.summary;
        out.push_str(&format!(
            "\nSummary: {} added, {} removed, {} modified\n",
            s.added, s.removed, s.modified,
        ));

        // Coverage delta.
        let old_total = s.old_coverage.total();
        let new_total = s.new_coverage.total();
        if new_total != old_total {
            let delta = new_total as isize - old_total as isize;
            let sign = if delta > 0 { "+" } else { "" };
            out.push_str(&format!(
                "Coverage: {} -> {} spec items ({sign}{})\n",
                old_total, new_total, delta,
            ));
        } else {
            out.push_str(&format!("Coverage: {} spec items (unchanged)\n", new_total));
        }

        // Verification status.
        match (s.old_verify_errors, s.new_verify_errors) {
            (0, 0) => out.push_str("Verification: clean -> clean\n"),
            (0, n) => out.push_str(&format!("Verification: clean -> {} error(s)\n", n)),
            (o, 0) => out.push_str(&format!("Verification: {} error(s) -> clean\n", o)),
            (o, n) if o == n => {
                out.push_str(&format!("Verification: {} error(s) (unchanged)\n", n))
            }
            (o, n) => out.push_str(&format!("Verification: {} -> {} error(s)\n", o, n)),
        }

        // Obligations delta.
        if s.new_obligations != s.old_obligations {
            out.push_str(&format!(
                "Obligations: {} -> {}\n",
                s.old_obligations, s.new_obligations,
            ));
        }

        out
    }
}
