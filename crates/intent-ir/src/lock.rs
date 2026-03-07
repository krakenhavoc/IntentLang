//! Multi-agent collaboration via spec-section locking.
//!
//! Provides a simple file-based locking mechanism that allows multiple agents
//! to claim ownership of spec items (entities, actions, invariants) to avoid
//! conflicting edits.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::audit::SpecItemKind;

/// A lock file tracking which agent owns which spec items.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockFile {
    pub module: String,
    pub claims: HashMap<String, Claim>,
}

/// A single agent's claim on a spec item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub agent: String,
    pub kind: SpecItemKind,
    pub claimed_at: String,
}

/// Errors from lock operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    /// The item is already claimed by a different agent.
    AlreadyClaimed {
        item: String,
        owner: String,
        requester: String,
    },
    /// The item is not claimed (cannot unlock).
    NotClaimed { item: String },
    /// The item is claimed by a different agent (cannot unlock).
    NotOwner {
        item: String,
        owner: String,
        requester: String,
    },
    /// The requested item does not exist in the spec.
    UnknownItem { item: String },
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::AlreadyClaimed {
                item,
                owner,
                requester,
            } => write!(
                f,
                "'{item}' is already claimed by agent '{owner}' (requested by '{requester}')"
            ),
            LockError::NotClaimed { item } => write!(f, "'{item}' is not claimed"),
            LockError::NotOwner {
                item,
                owner,
                requester,
            } => write!(f, "'{item}' is claimed by '{owner}', not '{requester}'"),
            LockError::UnknownItem { item } => {
                write!(f, "no spec item named '{item}' found")
            }
        }
    }
}

/// Known items in a spec (name → kind), used to validate lock targets.
pub type SpecItems = HashMap<String, SpecItemKind>;

/// Extract the set of lockable item names from an audit report.
pub fn extract_spec_items(report: &crate::audit::AuditReport) -> SpecItems {
    report
        .entries
        .iter()
        .map(|e| (e.name.clone(), e.kind))
        .collect()
}

/// Attempt to claim a spec item for an agent.
pub fn lock_item(
    lockfile: &mut LockFile,
    items: &SpecItems,
    item: &str,
    agent: &str,
    timestamp: &str,
) -> Result<(), LockError> {
    // Validate item exists.
    let kind = items.get(item).ok_or_else(|| LockError::UnknownItem {
        item: item.to_string(),
    })?;

    // Check if already claimed by a different agent.
    if let Some(claim) = lockfile.claims.get(item)
        && claim.agent != agent
    {
        return Err(LockError::AlreadyClaimed {
            item: item.to_string(),
            owner: claim.agent.clone(),
            requester: agent.to_string(),
        });
    }

    lockfile.claims.insert(
        item.to_string(),
        Claim {
            agent: agent.to_string(),
            kind: *kind,
            claimed_at: timestamp.to_string(),
        },
    );
    Ok(())
}

/// Release a claim on a spec item.
pub fn unlock_item(lockfile: &mut LockFile, item: &str, agent: &str) -> Result<(), LockError> {
    match lockfile.claims.get(item) {
        None => Err(LockError::NotClaimed {
            item: item.to_string(),
        }),
        Some(claim) if claim.agent != agent => Err(LockError::NotOwner {
            item: item.to_string(),
            owner: claim.agent.clone(),
            requester: agent.to_string(),
        }),
        Some(_) => {
            lockfile.claims.remove(item);
            Ok(())
        }
    }
}

/// Format lock status for human display.
pub fn format_status(lockfile: &LockFile, items: &SpecItems) -> String {
    let mut out = format!("Lock status for module: {}\n\n", lockfile.module);

    if lockfile.claims.is_empty() {
        out.push_str("  No items are currently claimed.\n");
        return out;
    }

    // Sort by item name for stable output.
    let mut claims: Vec<_> = lockfile.claims.iter().collect();
    claims.sort_by_key(|(name, _)| (*name).clone());

    for (name, claim) in &claims {
        out.push_str(&format!(
            "  {} {} — claimed by '{}' at {}\n",
            claim.kind, name, claim.agent, claim.claimed_at,
        ));
    }

    // Show unclaimed items.
    let mut unclaimed: Vec<_> = items
        .iter()
        .filter(|(name, _)| !lockfile.claims.contains_key(*name))
        .collect();
    unclaimed.sort_by_key(|(name, _)| (*name).clone());

    if !unclaimed.is_empty() {
        out.push_str("\n  Unclaimed:\n");
        for (name, kind) in &unclaimed {
            out.push_str(&format!("    {} {}\n", kind, name));
        }
    }

    out
}
