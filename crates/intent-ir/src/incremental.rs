//! Incremental verification.
//!
//! Caches per-item content hashes and verification results so that
//! only changed items (and their dependents) are re-verified.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::types::*;
use crate::verify::{
    Obligation, VerifyError, analyze_obligations, collect_module_call_names, verify_edge_guard,
    verify_function, verify_invariant,
};

/// Cached verification state for a module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyCache {
    /// Module name.
    pub module_name: String,
    /// Hash of the global context (sorted struct + function names).
    /// If this changes, the entire module must be re-verified.
    pub context_hash: u64,
    /// Per-struct content hashes (structs don't have direct errors but
    /// changes invalidate dependent functions/invariants).
    pub structs: HashMap<String, u64>,
    /// Per-function content hashes and cached errors.
    pub functions: HashMap<String, CachedItem>,
    /// Per-invariant content hashes and cached errors.
    pub invariants: HashMap<String, CachedItem>,
    /// Combined hash and errors for all edge guards.
    pub edge_guards: CachedItem,
}

/// Cached verification result for a single item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedItem {
    pub content_hash: u64,
    pub errors: Vec<VerifyError>,
}

/// Result of incremental verification.
#[derive(Debug, Clone, Serialize)]
pub struct IncrementalResult {
    /// All verification errors (merged from cached + fresh).
    pub errors: Vec<VerifyError>,
    /// All obligations (always recomputed).
    pub obligations: Vec<Obligation>,
    /// Updated cache to persist.
    pub cache: VerifyCache,
    /// Statistics about what was re-verified vs cached.
    pub stats: IncrementalStats,
}

/// Statistics for incremental verification.
#[derive(Debug, Clone, Serialize)]
pub struct IncrementalStats {
    pub total_items: usize,
    pub reverified: usize,
    pub cached: usize,
    /// True if the global context changed, forcing full re-verification.
    pub full_reverify: bool,
}

/// Perform incremental verification of an IR module.
///
/// If `cache` is `None`, performs a full verification and builds a new cache.
/// If `cache` is `Some`, compares content hashes to determine which items
/// changed and only re-verifies those.
///
/// Obligations are always recomputed since they are cross-cutting.
pub fn incremental_verify(module: &Module, cache: Option<&VerifyCache>) -> IncrementalResult {
    let current_ctx = context_hash(module);
    let total_items = module.functions.len()
        + module.invariants.len()
        + if module.edge_guards.is_empty() { 0 } else { 1 };

    // Build the global context needed by per-item verifiers.
    let known_types: HashSet<&str> = module
        .structs
        .iter()
        .map(|s| s.name.as_str())
        .chain(module.functions.iter().map(|f| f.name.as_str()))
        .collect();
    let mut call_names = HashSet::new();
    collect_module_call_names(module, &mut call_names);

    // Check if we can use the cache.
    let cache_valid = cache
        .map(|c| c.module_name == module.name && c.context_hash == current_ctx)
        .unwrap_or(false);

    let mut errors = Vec::new();
    let mut reverified = 0;
    let mut cached_count = 0;

    // -- Per-item hashes --
    let mut new_struct_hashes = HashMap::new();
    let mut new_func_items = HashMap::new();
    let mut new_inv_items = HashMap::new();

    // Compute struct hashes and check for changes.
    let mut struct_changed = false;
    for s in &module.structs {
        let h = content_hash_struct(s);
        if cache_valid {
            if let Some(old_h) = cache.and_then(|c| c.structs.get(&s.name)) {
                if *old_h != h {
                    struct_changed = true;
                }
            } else {
                struct_changed = true;
            }
        }
        new_struct_hashes.insert(s.name.clone(), h);
    }
    // If structs were removed, that's also a change.
    if cache_valid && let Some(c) = cache {
        for name in c.structs.keys() {
            if !new_struct_hashes.contains_key(name) {
                struct_changed = true;
            }
        }
    }

    // Verify functions.
    for func in &module.functions {
        let h = content_hash_function(func);
        let use_cached = cache_valid
            && !struct_changed
            && cache
                .and_then(|c| c.functions.get(&func.name))
                .map(|ci| ci.content_hash == h)
                .unwrap_or(false);

        if use_cached {
            let cached_errors = &cache.unwrap().functions[&func.name].errors;
            errors.extend(cached_errors.iter().cloned());
            cached_count += 1;
            new_func_items.insert(
                func.name.clone(),
                CachedItem {
                    content_hash: h,
                    errors: cached_errors.clone(),
                },
            );
        } else {
            let mut item_errors = Vec::new();
            verify_function(func, &known_types, &call_names, &mut item_errors);
            errors.extend(item_errors.iter().cloned());
            reverified += 1;
            new_func_items.insert(
                func.name.clone(),
                CachedItem {
                    content_hash: h,
                    errors: item_errors,
                },
            );
        }
    }

    // Verify invariants.
    for inv in &module.invariants {
        let h = content_hash_invariant(inv);
        let use_cached = cache_valid
            && !struct_changed
            && cache
                .and_then(|c| c.invariants.get(&inv.name))
                .map(|ci| ci.content_hash == h)
                .unwrap_or(false);

        if use_cached {
            let cached_errors = &cache.unwrap().invariants[&inv.name].errors;
            errors.extend(cached_errors.iter().cloned());
            cached_count += 1;
            new_inv_items.insert(
                inv.name.clone(),
                CachedItem {
                    content_hash: h,
                    errors: cached_errors.clone(),
                },
            );
        } else {
            let mut item_errors = Vec::new();
            verify_invariant(inv, &known_types, &call_names, &mut item_errors);
            errors.extend(item_errors.iter().cloned());
            reverified += 1;
            new_inv_items.insert(
                inv.name.clone(),
                CachedItem {
                    content_hash: h,
                    errors: item_errors,
                },
            );
        }
    }

    // Verify edge guards (treated as a single unit).
    let eg_hash = content_hash_edge_guards(&module.edge_guards);
    let eg_use_cached = cache_valid
        && !struct_changed
        && cache
            .map(|c| c.edge_guards.content_hash == eg_hash)
            .unwrap_or(false);
    let new_eg = if !module.edge_guards.is_empty() {
        if eg_use_cached {
            let cached_errors = &cache.unwrap().edge_guards.errors;
            errors.extend(cached_errors.iter().cloned());
            cached_count += 1;
            CachedItem {
                content_hash: eg_hash,
                errors: cached_errors.clone(),
            }
        } else {
            let mut item_errors = Vec::new();
            for guard in &module.edge_guards {
                verify_edge_guard(guard, &known_types, &mut item_errors);
            }
            errors.extend(item_errors.iter().cloned());
            if total_items > 0 {
                reverified += 1;
            }
            CachedItem {
                content_hash: eg_hash,
                errors: item_errors,
            }
        }
    } else {
        CachedItem {
            content_hash: 0,
            errors: vec![],
        }
    };

    // Always recompute obligations (cheap, cross-cutting).
    let obligations = analyze_obligations(module);

    let new_cache = VerifyCache {
        module_name: module.name.clone(),
        context_hash: current_ctx,
        structs: new_struct_hashes,
        functions: new_func_items,
        invariants: new_inv_items,
        edge_guards: new_eg,
    };

    IncrementalResult {
        errors,
        obligations,
        cache: new_cache,
        stats: IncrementalStats {
            total_items,
            reverified,
            cached: cached_count,
            full_reverify: !cache_valid || struct_changed,
        },
    }
}

// ── Hashing helpers ────────────────────────────────────────

/// Hash of the global context: sorted struct + function names.
/// If items are added or removed, this changes.
fn context_hash(module: &Module) -> u64 {
    let mut names: Vec<&str> = module
        .structs
        .iter()
        .map(|s| s.name.as_str())
        .chain(module.functions.iter().map(|f| f.name.as_str()))
        .chain(module.invariants.iter().map(|i| i.name.as_str()))
        .collect();
    names.sort();
    hash_parts(&names)
}

/// Content hash for a struct (name + fields), excluding source traces.
fn content_hash_struct(s: &Struct) -> u64 {
    let fields_json: Vec<String> = s
        .fields
        .iter()
        .map(|f| format!("{}:{}", f.name, serde_json::to_string(&f.ty).unwrap()))
        .collect();
    let mut parts = vec![s.name.clone()];
    parts.extend(fields_json);
    let refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
    hash_parts(&refs)
}

/// Content hash for a function (name + params + pre/postconditions + properties),
/// excluding source traces.
fn content_hash_function(func: &Function) -> u64 {
    let mut parts = vec![func.name.clone()];
    for p in &func.params {
        parts.push(format!(
            "param:{}:{}",
            p.name,
            serde_json::to_string(&p.ty).unwrap()
        ));
    }
    for pre in &func.preconditions {
        parts.push(format!("pre:{}", expr_signature(&pre.expr)));
    }
    for post in &func.postconditions {
        match post {
            Postcondition::Always { expr, .. } => {
                parts.push(format!("post:{}", expr_signature(expr)));
            }
            Postcondition::When { guard, expr, .. } => {
                parts.push(format!(
                    "post-when:{}:{}",
                    expr_signature(guard),
                    expr_signature(expr)
                ));
            }
        }
    }
    for prop in &func.properties {
        parts.push(format!(
            "prop:{}:{}",
            prop.key,
            serde_json::to_string(&prop.value).unwrap()
        ));
    }
    let refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
    hash_parts(&refs)
}

/// Content hash for an invariant (name + expression), excluding source traces.
fn content_hash_invariant(inv: &Invariant) -> u64 {
    let parts = [inv.name.as_str(), &expr_signature(&inv.expr)];
    hash_parts(&parts)
}

/// Content hash for all edge guards combined.
fn content_hash_edge_guards(guards: &[EdgeGuard]) -> u64 {
    let mut parts = Vec::new();
    for g in guards {
        parts.push(format!(
            "guard:{}:{}",
            expr_signature(&g.condition),
            g.action
        ));
        for (k, v) in &g.args {
            parts.push(format!("arg:{}:{}", k, expr_signature(v)));
        }
    }
    let refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
    hash_parts(&refs)
}

/// Produce a canonical string representation of an IR expression (no traces).
fn expr_signature(expr: &IrExpr) -> String {
    serde_json::to_string(expr).unwrap()
}

/// Hash a slice of string parts using DefaultHasher.
fn hash_parts(parts: &[&str]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for part in parts {
        part.hash(&mut hasher);
    }
    hasher.finish()
}
