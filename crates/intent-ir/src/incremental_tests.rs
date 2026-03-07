use crate::incremental::incremental_verify;
use crate::lower::lower_file;

fn lower(src: &str) -> crate::types::Module {
    let ast = intent_parser::parse_file(src).unwrap();
    lower_file(&ast)
}

#[test]
fn no_cache_does_full_verify() {
    let module = lower("module M entity X { v: Int }");
    let result = incremental_verify(&module, None);
    assert!(result.errors.is_empty());
    assert!(result.stats.full_reverify);
    assert_eq!(result.stats.cached, 0);
}

#[test]
fn identical_module_uses_cache() {
    let module = lower("module M entity X { v: Int } action A { x: X requires { x.v > 0 } }");
    let first = incremental_verify(&module, None);
    assert!(first.stats.full_reverify);
    assert_eq!(first.stats.reverified, 1); // 1 function

    // Re-verify with same module — should use cache.
    let second = incremental_verify(&module, Some(&first.cache));
    assert!(!second.stats.full_reverify);
    assert_eq!(second.stats.cached, 1);
    assert_eq!(second.stats.reverified, 0);
}

#[test]
fn changed_function_is_reverified() {
    let module_v1 = lower("module M entity X { v: Int } action A { x: X requires { x.v > 0 } }");
    let first = incremental_verify(&module_v1, None);

    // Change the precondition.
    let module_v2 = lower("module M entity X { v: Int } action A { x: X requires { x.v >= 0 } }");
    let second = incremental_verify(&module_v2, Some(&first.cache));
    assert_eq!(second.stats.reverified, 1);
    assert_eq!(second.stats.cached, 0);
}

#[test]
fn added_function_triggers_context_change() {
    let module_v1 = lower("module M entity X { v: Int } action A { x: X }");
    let first = incremental_verify(&module_v1, None);

    // Add a new action — context changes (new function name).
    let module_v2 = lower("module M entity X { v: Int } action A { x: X } action B { x: X }");
    let second = incremental_verify(&module_v2, Some(&first.cache));
    // Context changed, so full re-verify.
    assert!(second.stats.full_reverify);
}

#[test]
fn struct_change_invalidates_dependent_functions() {
    let module_v1 = lower("module M entity X { v: Int } action A { x: X requires { x.v > 0 } }");
    let first = incremental_verify(&module_v1, None);

    // Change the struct (add a field).
    let module_v2 =
        lower("module M entity X { v: Int w: Bool } action A { x: X requires { x.v > 0 } }");
    let second = incremental_verify(&module_v2, Some(&first.cache));
    // Struct changed, so full re-verify even though function is same.
    assert!(second.stats.full_reverify);
    assert_eq!(second.stats.reverified, 1);
}

#[test]
fn invariant_caching() {
    let src = "module M entity X { v: Int } invariant Pos { forall x: X => x.v >= 0 }";
    let module = lower(src);
    let first = incremental_verify(&module, None);
    assert_eq!(first.stats.reverified, 1); // invariant

    let second = incremental_verify(&module, Some(&first.cache));
    assert_eq!(second.stats.cached, 1);
    assert_eq!(second.stats.reverified, 0);
}

#[test]
fn obligations_always_recomputed() {
    let src = "module M entity X { v: Int } action A { x: X ensures { x.v == old(x.v) + 1 } } invariant Pos { forall x: X => x.v >= 0 }";
    let module = lower(src);
    let first = incremental_verify(&module, None);
    assert_eq!(first.obligations.len(), 1);

    // Even with cache, obligations are present.
    let second = incremental_verify(&module, Some(&first.cache));
    assert_eq!(second.obligations.len(), 1);
}

#[test]
fn errors_preserved_in_cache() {
    // This has a verification error: postcondition without params.
    let src = "module M entity X { v: Int } action Bad { ensures { true } }";
    let module = lower(src);
    let first = incremental_verify(&module, None);
    assert!(!first.errors.is_empty());

    // Cached result should also have errors.
    let second = incremental_verify(&module, Some(&first.cache));
    assert_eq!(second.errors.len(), first.errors.len());
    assert_eq!(second.stats.cached, 1);
}

#[test]
fn edge_guard_caching() {
    let src = "module M edge_cases { when x == y => reject(\"no\") }";
    let module = lower(src);
    let first = incremental_verify(&module, None);
    assert_eq!(first.stats.reverified, 1); // edge guards

    let second = incremental_verify(&module, Some(&first.cache));
    assert_eq!(second.stats.cached, 1);
    assert_eq!(second.stats.reverified, 0);
}

#[test]
fn cache_serialization_roundtrip() {
    let src = "module M entity X { v: Int } action A { x: X requires { x.v > 0 } }";
    let module = lower(src);
    let result = incremental_verify(&module, None);

    // Serialize and deserialize the cache.
    let json = serde_json::to_string(&result.cache).unwrap();
    let restored: crate::incremental::VerifyCache = serde_json::from_str(&json).unwrap();

    // Re-verify with the restored cache — should use it.
    let second = incremental_verify(&module, Some(&restored));
    assert_eq!(second.stats.cached, 1);
    assert_eq!(second.stats.reverified, 0);
}

#[test]
fn module_name_mismatch_invalidates_cache() {
    let module_a = lower("module A entity X { v: Int }");
    let cache_a = incremental_verify(&module_a, None).cache;

    let module_b = lower("module B entity X { v: Int }");
    let result = incremental_verify(&module_b, Some(&cache_a));
    // Different module name → full re-verify.
    assert!(result.stats.full_reverify);
}
