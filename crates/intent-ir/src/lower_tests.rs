use crate::lower::lower_file;
use crate::types::*;

fn parse(src: &str) -> intent_parser::ast::File {
    intent_parser::parse_file(src).unwrap()
}

#[test]
fn lower_entity_to_struct() {
    let ast = parse("module M entity Account { id: UUID balance: Decimal(precision: 2) }");
    let ir = lower_file(&ast);

    assert_eq!(ir.name, "M");
    assert_eq!(ir.structs.len(), 1);
    let s = &ir.structs[0];
    assert_eq!(s.name, "Account");
    assert_eq!(s.fields.len(), 2);
    assert_eq!(s.fields[0].name, "id");
    assert_eq!(s.fields[0].ty, IrType::Named("UUID".into()));
    assert_eq!(s.fields[1].name, "balance");
    assert_eq!(s.fields[1].ty, IrType::Decimal(2));
}

#[test]
fn lower_action_to_function() {
    let ast = parse(
        "module M entity X { v: Int } action A { x: X requires { x.v > 0 } ensures { x.v == old(x.v) + 1 } }",
    );
    let ir = lower_file(&ast);

    assert_eq!(ir.functions.len(), 1);
    let f = &ir.functions[0];
    assert_eq!(f.name, "A");
    assert_eq!(f.params.len(), 1);
    assert_eq!(f.params[0].name, "x");
    assert_eq!(f.preconditions.len(), 1);
    assert_eq!(f.postconditions.len(), 1);
}

#[test]
fn lower_invariant() {
    let ast = parse("module M entity X { v: Int } invariant Pos { forall x: X => x.v >= 0 }");
    let ir = lower_file(&ast);

    assert_eq!(ir.invariants.len(), 1);
    assert_eq!(ir.invariants[0].name, "Pos");
    // The body should be a Forall
    assert!(matches!(ir.invariants[0].expr, IrExpr::Forall { .. }));
}

#[test]
fn lower_edge_cases() {
    let ast = parse(
        "module M action A { x: Int } edge_cases { when x > 100 => reject(\"too big\") }",
    );
    let ir = lower_file(&ast);

    assert_eq!(ir.edge_guards.len(), 1);
    assert_eq!(ir.edge_guards[0].action, "reject");
}

#[test]
fn lower_optional_type() {
    let ast = parse("module M entity X { v: Int? }");
    let ir = lower_file(&ast);

    assert_eq!(ir.structs[0].fields[0].ty, IrType::Optional(Box::new(IrType::Named("Int".into()))));
}

#[test]
fn lower_union_type() {
    let ast = parse("module M entity X { status: Active | Frozen | Closed }");
    let ir = lower_file(&ast);

    assert_eq!(
        ir.structs[0].fields[0].ty,
        IrType::Union(vec!["Active".into(), "Frozen".into(), "Closed".into()])
    );
}

#[test]
fn lower_collection_types() {
    let ast = parse("module M entity X { items: List<Int> tags: Set<String> meta: Map<String, Int> }");
    let ir = lower_file(&ast);

    assert_eq!(ir.structs[0].fields[0].ty, IrType::List(Box::new(IrType::Named("Int".into()))));
    assert_eq!(ir.structs[0].fields[1].ty, IrType::Set(Box::new(IrType::Named("String".into()))));
    assert_eq!(
        ir.structs[0].fields[2].ty,
        IrType::Map(
            Box::new(IrType::Named("String".into())),
            Box::new(IrType::Named("Int".into()))
        )
    );
}

#[test]
fn lower_when_postcondition() {
    let ast = parse(
        "module M entity X { v: Int } action A { x: X ensures { when x.v > 0 => x.v == old(x.v) } }",
    );
    let ir = lower_file(&ast);

    assert_eq!(ir.functions[0].postconditions.len(), 1);
    assert!(matches!(
        ir.functions[0].postconditions[0],
        Postcondition::When { .. }
    ));
}

#[test]
fn lower_properties() {
    let ast = parse(
        "module M action A { x: Int properties { idempotent: true atomic: true max_latency_ms: 500 } }",
    );
    let ir = lower_file(&ast);

    let props = &ir.functions[0].properties;
    assert_eq!(props.len(), 3);
    assert_eq!(props[0].key, "idempotent");
    assert!(matches!(props[0].value, PropertyValue::Bool(true)));
    assert_eq!(props[2].key, "max_latency_ms");
    assert!(matches!(props[2].value, PropertyValue::Int(500)));
}

#[test]
fn source_traces_populated() {
    let ast = parse("module Mod entity Ent { f: Int }");
    let ir = lower_file(&ast);

    let trace = &ir.structs[0].trace;
    assert_eq!(trace.module, "Mod");
    assert_eq!(trace.item, "Ent");
    assert_eq!(trace.part, "entity");

    let field_trace = &ir.structs[0].fields[0].trace;
    assert_eq!(field_trace.part, "field:f");
}

#[test]
fn lower_zero_arg_call() {
    let ast = parse("module M action A { x: Int requires { x > now() } }");
    let ir = lower_file(&ast);

    let pre = &ir.functions[0].preconditions[0].expr;
    if let IrExpr::Compare { right, .. } = pre {
        assert!(matches!(right.as_ref(), IrExpr::Call { name, args } if name == "now" && args.is_empty()));
    } else {
        panic!("expected Compare");
    }
}
