//! Pest-based parser that converts `.intent` source text into a typed AST.
//!
//! The grammar is defined in `grammar/intent.pest`. This module wraps the
//! generated pest parser and transforms pest `Pairs` into [`ast`] nodes.

use pest::Parser;
use pest_derive::Parser;

use crate::ast::*;

/// The pest-generated parser. Grammar is loaded at compile time from the
/// workspace-relative path.
#[derive(Parser)]
#[grammar = "src/intent.pest"]
pub struct IntentParser;

/// Parse error with human-readable message and source location.
#[derive(Debug, thiserror::Error, miette::Diagnostic, Clone)]
#[error("{message}")]
#[diagnostic(code(intent::parse::syntax_error))]
pub struct ParseError {
    pub message: String,
    #[label("{label}")]
    pub span: miette::SourceSpan,
    pub label: String,
    #[help]
    pub help: Option<String>,
}

impl From<pest::error::Error<Rule>> for ParseError {
    fn from(err: pest::error::Error<Rule>) -> Self {
        humanize_pest_error(err)
    }
}

/// Convert a pest error into a human-readable ParseError with helpful messages.
fn humanize_pest_error(err: pest::error::Error<Rule>) -> ParseError {
    let (offset, len) = match err.location {
        pest::error::InputLocation::Pos(p) => (p, 1),
        pest::error::InputLocation::Span((s, e)) => (s, e - s),
    };
    let span: miette::SourceSpan = (offset, len).into();

    // Extract the expected rules from the pest error variant
    let (message, label, help) = match &err.variant {
        pest::error::ErrorVariant::ParsingError { positives, .. } => {
            humanize_expected_rules(positives)
        }
        pest::error::ErrorVariant::CustomError { message } => {
            (message.clone(), "here".to_string(), None)
        }
    };

    ParseError {
        message,
        span,
        label,
        help,
    }
}

/// Map pest rule names to human-readable error messages.
fn humanize_expected_rules(rules: &[Rule]) -> (String, String, Option<String>) {
    // Check for common patterns in what was expected
    let rule_set: std::collections::HashSet<&Rule> = rules.iter().collect();

    if rule_set.contains(&Rule::module_decl) {
        return (
            "missing module declaration".to_string(),
            "expected `module ModuleName`".to_string(),
            Some("every .intent file must start with `module ModuleName`".to_string()),
        );
    }

    if rule_set.contains(&Rule::union_type) || rule_set.contains(&Rule::simple_type) {
        return (
            "invalid type".to_string(),
            "expected a type".to_string(),
            Some(
                "types must start with an uppercase letter (e.g., String, UUID, MyEntity)"
                    .to_string(),
            ),
        );
    }

    if rule_set.contains(&Rule::optional_marker) && rule_set.contains(&Rule::ident) {
        return (
            "unexpected end of block".to_string(),
            "expected a field declaration or `}`".to_string(),
            Some("check for unclosed braces or missing field declarations".to_string()),
        );
    }

    if rule_set.contains(&Rule::field_decl) || rule_set.contains(&Rule::param_decl) {
        return (
            "expected a field or parameter declaration".to_string(),
            "expected `name: Type`".to_string(),
            Some("fields are declared as `name: Type` (e.g., `email: String`)".to_string()),
        );
    }

    if rule_set.contains(&Rule::EOI) {
        return (
            "unexpected content after end of file".to_string(),
            "unexpected token".to_string(),
            Some("check for extra text or unclosed blocks".to_string()),
        );
    }

    // Fallback: format the rule names
    let names: Vec<String> = rules
        .iter()
        .filter(|r| !matches!(r, Rule::WHITESPACE | Rule::COMMENT | Rule::EOI))
        .map(|r| format!("`{:?}`", r))
        .collect();

    let msg = if names.is_empty() {
        "syntax error".to_string()
    } else {
        format!("expected {}", names.join(" or "))
    };

    ("syntax error".to_string(), msg, None)
}

/// Parse a complete `.intent` source string into an AST [`File`].
pub fn parse_file(source: &str) -> Result<File, ParseError> {
    let pairs = IntentParser::parse(Rule::file, source)?;
    let pair = pairs.into_iter().next().unwrap();
    Ok(build_file(pair))
}

// ── Builders ─────────────────────────────────────────────────
// Each `build_*` function consumes a pest `Pair` and returns an AST node.

fn span_of(pair: &pest::iterators::Pair<'_, Rule>) -> Span {
    let s = pair.as_span();
    Span {
        start: s.start(),
        end: s.end(),
    }
}

fn build_file(pair: pest::iterators::Pair<'_, Rule>) -> File {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();

    let module = build_module_decl(inner.next().unwrap());

    let mut doc = None;
    let mut items = Vec::new();

    for p in inner {
        match p.as_rule() {
            Rule::doc_block => doc = Some(build_doc_block(p)),
            Rule::entity_decl => items.push(TopLevelItem::Entity(build_entity_decl(p))),
            Rule::action_decl => items.push(TopLevelItem::Action(build_action_decl(p))),
            Rule::invariant_decl => items.push(TopLevelItem::Invariant(build_invariant_decl(p))),
            Rule::edge_cases_decl => items.push(TopLevelItem::EdgeCases(build_edge_cases_decl(p))),
            Rule::EOI => {}
            _ => {}
        }
    }

    File {
        module,
        doc,
        items,
        span,
    }
}

fn build_module_decl(pair: pest::iterators::Pair<'_, Rule>) -> ModuleDecl {
    let span = span_of(&pair);
    let name = pair.into_inner().next().unwrap().as_str().to_string();
    ModuleDecl { name, span }
}

fn build_doc_block(pair: pest::iterators::Pair<'_, Rule>) -> DocBlock {
    let span = span_of(&pair);
    let lines = pair
        .into_inner()
        .map(|p| {
            let text = p.as_str();
            let content = text
                .strip_prefix("---")
                .unwrap_or(text)
                .trim_end_matches('\n');
            content.strip_prefix(' ').unwrap_or(content).to_string()
        })
        .collect();
    DocBlock { lines, span }
}

fn build_entity_decl(pair: pest::iterators::Pair<'_, Rule>) -> EntityDecl {
    let span = span_of(&pair);
    let mut doc = None;
    let mut name = String::new();
    let mut fields = Vec::new();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::doc_block => doc = Some(build_doc_block(p)),
            Rule::type_ident => name = p.as_str().to_string(),
            Rule::field_decl => fields.push(build_field_decl(p)),
            _ => {}
        }
    }

    EntityDecl {
        doc,
        name,
        fields,
        span,
    }
}

fn build_field_decl(pair: pest::iterators::Pair<'_, Rule>) -> FieldDecl {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let ty = build_type_expr(inner.next().unwrap());
    FieldDecl { name, ty, span }
}

fn build_action_decl(pair: pest::iterators::Pair<'_, Rule>) -> ActionDecl {
    let span = span_of(&pair);
    let mut doc = None;
    let mut name = String::new();
    let mut params = Vec::new();
    let mut requires = None;
    let mut ensures = None;
    let mut properties = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::doc_block => doc = Some(build_doc_block(p)),
            Rule::type_ident => name = p.as_str().to_string(),
            Rule::param_decl => params.push(build_field_decl(p)),
            Rule::requires_block => requires = Some(build_requires_block(p)),
            Rule::ensures_block => ensures = Some(build_ensures_block(p)),
            Rule::properties_block => properties = Some(build_properties_block(p)),
            _ => {}
        }
    }

    ActionDecl {
        doc,
        name,
        params,
        requires,
        ensures,
        properties,
        span,
    }
}

fn build_requires_block(pair: pest::iterators::Pair<'_, Rule>) -> RequiresBlock {
    let span = span_of(&pair);
    let conditions = pair.into_inner().map(build_expr).collect();
    RequiresBlock { conditions, span }
}

fn build_ensures_block(pair: pest::iterators::Pair<'_, Rule>) -> EnsuresBlock {
    let span = span_of(&pair);
    let items = pair
        .into_inner()
        .map(|p| match p.as_rule() {
            Rule::when_clause => EnsuresItem::When(build_when_clause(p)),
            _ => EnsuresItem::Expr(build_expr(p)),
        })
        .collect();
    EnsuresBlock { items, span }
}

fn build_when_clause(pair: pest::iterators::Pair<'_, Rule>) -> WhenClause {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let condition = build_or_expr(inner.next().unwrap());
    let consequence = build_expr(inner.next().unwrap());
    WhenClause {
        condition,
        consequence,
        span,
    }
}

fn build_properties_block(pair: pest::iterators::Pair<'_, Rule>) -> PropertiesBlock {
    let span = span_of(&pair);
    let entries = pair.into_inner().map(build_prop_entry).collect();
    PropertiesBlock { entries, span }
}

fn build_prop_entry(pair: pest::iterators::Pair<'_, Rule>) -> PropEntry {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let key = inner.next().unwrap().as_str().to_string();
    let value = build_prop_value(inner.next().unwrap());
    PropEntry { key, value, span }
}

fn build_prop_value(pair: pest::iterators::Pair<'_, Rule>) -> PropValue {
    match pair.as_rule() {
        Rule::obj_literal => {
            let fields = pair
                .into_inner()
                .map(|f| {
                    let mut inner = f.into_inner();
                    let key = inner.next().unwrap().as_str().to_string();
                    let value = build_prop_value(inner.next().unwrap());
                    (key, value)
                })
                .collect();
            PropValue::Object(fields)
        }
        Rule::list_literal => {
            let items = pair.into_inner().map(build_prop_value).collect();
            PropValue::List(items)
        }
        Rule::string_literal => {
            let s = extract_string(pair);
            PropValue::Literal(Literal::String(s))
        }
        Rule::number_literal => PropValue::Literal(parse_number_literal(pair.as_str())),
        Rule::bool_literal => PropValue::Literal(Literal::Bool(pair.as_str() == "true")),
        Rule::ident => PropValue::Ident(pair.as_str().to_string()),
        // For expressions nested in prop value contexts, try to extract
        Rule::expr | Rule::implies_expr => {
            // Recurse into inner pairs
            let inner = pair.into_inner().next().unwrap();
            build_prop_value(inner)
        }
        _ => PropValue::Ident(pair.as_str().to_string()),
    }
}

fn build_invariant_decl(pair: pest::iterators::Pair<'_, Rule>) -> InvariantDecl {
    let span = span_of(&pair);
    let mut doc = None;
    let mut name = String::new();
    let mut body = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::doc_block => doc = Some(build_doc_block(p)),
            Rule::type_ident => name = p.as_str().to_string(),
            Rule::expr => body = Some(build_expr(p)),
            _ => {}
        }
    }

    InvariantDecl {
        doc,
        name,
        body: body.expect("invariant must have a body expression"),
        span,
    }
}

fn build_edge_cases_decl(pair: pest::iterators::Pair<'_, Rule>) -> EdgeCasesDecl {
    let span = span_of(&pair);
    let rules = pair.into_inner().map(build_edge_rule).collect();
    EdgeCasesDecl { rules, span }
}

fn build_edge_rule(pair: pest::iterators::Pair<'_, Rule>) -> EdgeRule {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let condition = build_or_expr(inner.next().unwrap());
    let action = build_action_call(inner.next().unwrap());
    EdgeRule {
        condition,
        action,
        span,
    }
}

fn build_action_call(pair: pest::iterators::Pair<'_, Rule>) -> ActionCall {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let args = inner
        .next()
        .map(|p| p.into_inner().map(build_call_arg).collect())
        .unwrap_or_default();
    ActionCall { name, args, span }
}

// ── Type expression builders ─────────────────────────────────

fn build_type_expr(pair: pest::iterators::Pair<'_, Rule>) -> TypeExpr {
    let span = span_of(&pair);
    let mut optional = false;
    let mut ty_kind = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::union_type => ty_kind = Some(build_union_type(p)),
            Rule::optional_marker => optional = true,
            _ => {}
        }
    }

    TypeExpr {
        ty: ty_kind.unwrap(),
        optional,
        span,
    }
}

fn build_union_type(pair: pest::iterators::Pair<'_, Rule>) -> TypeKind {
    let variants: Vec<TypeKind> = pair.into_inner().map(build_base_type).collect();
    if variants.len() == 1 {
        variants.into_iter().next().unwrap()
    } else {
        TypeKind::Union(variants)
    }
}

fn build_base_type(pair: pest::iterators::Pair<'_, Rule>) -> TypeKind {
    match pair.as_rule() {
        Rule::list_type => {
            let inner = pair.into_inner().next().unwrap();
            TypeKind::List(Box::new(build_type_expr(inner)))
        }
        Rule::set_type => {
            let inner = pair.into_inner().next().unwrap();
            TypeKind::Set(Box::new(build_type_expr(inner)))
        }
        Rule::map_type => {
            let mut inner = pair.into_inner();
            let key = build_type_expr(inner.next().unwrap());
            let value = build_type_expr(inner.next().unwrap());
            TypeKind::Map(Box::new(key), Box::new(value))
        }
        Rule::parameterized_type => {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let params = inner.map(build_type_param).collect();
            TypeKind::Parameterized { name, params }
        }
        Rule::simple_type => {
            let name = pair.into_inner().next().unwrap().as_str().to_string();
            TypeKind::Simple(name)
        }
        _ => TypeKind::Simple(pair.as_str().to_string()),
    }
}

fn build_type_param(pair: pest::iterators::Pair<'_, Rule>) -> TypeParam {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let value = parse_number_literal(inner.next().unwrap().as_str());
    TypeParam { name, value, span }
}

// ── Expression builders ──────────────────────────────────────

fn build_expr(pair: pest::iterators::Pair<'_, Rule>) -> Expr {
    let span = span_of(&pair);
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::implies_expr => build_implies_expr(inner),
        _ => {
            let kind = build_expr_kind(inner);
            Expr { kind, span }
        }
    }
}

fn build_implies_expr(pair: pest::iterators::Pair<'_, Rule>) -> Expr {
    let mut parts: Vec<pest::iterators::Pair<'_, Rule>> = Vec::new();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::implies_op => {}
            _ => parts.push(p),
        }
    }

    let mut result = build_or_expr(parts.remove(0));
    for part in parts {
        let right = build_or_expr(part);
        let new_span = Span {
            start: result.span.start,
            end: right.span.end,
        };
        result = Expr {
            kind: ExprKind::Implies(Box::new(result), Box::new(right)),
            span: new_span,
        };
    }
    result
}

fn build_or_expr(pair: pest::iterators::Pair<'_, Rule>) -> Expr {
    let span = span_of(&pair);
    let mut parts: Vec<pest::iterators::Pair<'_, Rule>> = Vec::new();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::or_op => {}
            _ => parts.push(p),
        }
    }

    if parts.is_empty() {
        return Expr {
            kind: ExprKind::Literal(Literal::Null),
            span,
        };
    }

    let mut result = build_and_expr(parts.remove(0));
    for part in parts {
        let right = build_and_expr(part);
        let new_span = Span {
            start: result.span.start,
            end: right.span.end,
        };
        result = Expr {
            kind: ExprKind::Or(Box::new(result), Box::new(right)),
            span: new_span,
        };
    }
    result
}

fn build_and_expr(pair: pest::iterators::Pair<'_, Rule>) -> Expr {
    let span = span_of(&pair);
    let mut parts: Vec<pest::iterators::Pair<'_, Rule>> = Vec::new();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::and_op => {}
            _ => parts.push(p),
        }
    }

    if parts.is_empty() {
        return Expr {
            kind: ExprKind::Literal(Literal::Null),
            span,
        };
    }

    let mut result = build_not_expr(parts.remove(0));
    for part in parts {
        let right = build_not_expr(part);
        let new_span = Span {
            start: result.span.start,
            end: right.span.end,
        };
        result = Expr {
            kind: ExprKind::And(Box::new(result), Box::new(right)),
            span: new_span,
        };
    }
    result
}

fn build_not_expr(pair: pest::iterators::Pair<'_, Rule>) -> Expr {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();

    match first.as_rule() {
        Rule::not_op => {
            let operand = build_not_expr(inner.next().unwrap());
            Expr {
                kind: ExprKind::Not(Box::new(operand)),
                span,
            }
        }
        Rule::cmp_expr => build_cmp_expr(first),
        _ => {
            let kind = build_expr_kind(first);
            Expr { kind, span }
        }
    }
}

fn build_cmp_expr(pair: pest::iterators::Pair<'_, Rule>) -> Expr {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let left = build_add_expr(inner.next().unwrap());

    if let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "==" => CmpOp::Eq,
            "!=" => CmpOp::Ne,
            "<" => CmpOp::Lt,
            ">" => CmpOp::Gt,
            "<=" => CmpOp::Le,
            ">=" => CmpOp::Ge,
            _ => unreachable!("unknown cmp op: {}", op_pair.as_str()),
        };
        let right = build_add_expr(inner.next().unwrap());
        Expr {
            kind: ExprKind::Compare {
                left: Box::new(left),
                op,
                right: Box::new(right),
            },
            span,
        }
    } else {
        Expr {
            kind: left.kind,
            span,
        }
    }
}

fn build_add_expr(pair: pest::iterators::Pair<'_, Rule>) -> Expr {
    let mut children: Vec<pest::iterators::Pair<'_, Rule>> = pair.into_inner().collect();

    if children.len() == 1 {
        return build_primary(children.remove(0));
    }

    // Interleaved: primary, op, primary, op, primary, ...
    let mut iter = children.into_iter();
    let mut result = build_primary(iter.next().unwrap());

    while let Some(op_pair) = iter.next() {
        let op = match op_pair.as_str() {
            "+" => ArithOp::Add,
            "-" => ArithOp::Sub,
            _ => unreachable!("unknown add op"),
        };
        let right = build_primary(iter.next().unwrap());
        let new_span = Span {
            start: result.span.start,
            end: right.span.end,
        };
        result = Expr {
            kind: ExprKind::Arithmetic {
                left: Box::new(result),
                op,
                right: Box::new(right),
            },
            span: new_span,
        };
    }

    result
}

fn build_primary(pair: pest::iterators::Pair<'_, Rule>) -> Expr {
    let span = span_of(&pair);
    let mut inner: Vec<pest::iterators::Pair<'_, Rule>> = pair.into_inner().collect();

    // First child is the atom, rest are `.ident` field accesses
    let atom_pair = inner.remove(0);
    let base = build_atom(atom_pair);

    if inner.is_empty() {
        return Expr {
            kind: base.kind,
            span,
        };
    }

    let fields: Vec<String> = inner.into_iter().map(|p| p.as_str().to_string()).collect();

    Expr {
        kind: ExprKind::FieldAccess {
            root: Box::new(base),
            fields,
        },
        span,
    }
}

fn build_atom(pair: pest::iterators::Pair<'_, Rule>) -> Expr {
    let span = span_of(&pair);
    match pair.as_rule() {
        Rule::old_expr => {
            let inner = pair.into_inner().next().unwrap();
            let expr = build_expr(inner);
            Expr {
                kind: ExprKind::Old(Box::new(expr)),
                span,
            }
        }
        Rule::quantifier_expr => {
            let mut inner = pair.into_inner();
            let kw = inner.next().unwrap();
            let kind = match kw.as_str() {
                "forall" => QuantifierKind::Forall,
                "exists" => QuantifierKind::Exists,
                _ => unreachable!(),
            };
            let binding = inner.next().unwrap().as_str().to_string();
            let ty = inner.next().unwrap().as_str().to_string();
            let body = build_expr(inner.next().unwrap());
            Expr {
                kind: ExprKind::Quantifier {
                    kind,
                    binding,
                    ty,
                    body: Box::new(body),
                },
                span,
            }
        }
        Rule::null_literal => Expr {
            kind: ExprKind::Literal(Literal::Null),
            span,
        },
        Rule::bool_literal => Expr {
            kind: ExprKind::Literal(Literal::Bool(pair.as_str() == "true")),
            span,
        },
        Rule::number_literal => Expr {
            kind: ExprKind::Literal(parse_number_literal(pair.as_str())),
            span,
        },
        Rule::string_literal => Expr {
            kind: ExprKind::Literal(Literal::String(extract_string(pair))),
            span,
        },
        Rule::list_literal => {
            // TODO: build list expression
            Expr {
                kind: ExprKind::Literal(Literal::Null),
                span,
            }
        }
        Rule::paren_expr => {
            let inner = pair.into_inner().next().unwrap();
            build_expr(inner)
        }
        Rule::call_or_ident => {
            // When call_args is empty (e.g., `now()`), pest produces no inner
            // pairs for the parens. Check the raw text for `(` to distinguish
            // zero-arg calls from plain identifiers.
            let text = pair.as_str();
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            if text.contains('(') {
                let args = inner
                    .next()
                    .map(|args_pair| args_pair.into_inner().map(build_call_arg).collect())
                    .unwrap_or_default();
                Expr {
                    kind: ExprKind::Call { name, args },
                    span,
                }
            } else {
                Expr {
                    kind: ExprKind::Ident(name),
                    span,
                }
            }
        }
        _ => Expr {
            kind: ExprKind::Ident(pair.as_str().to_string()),
            span,
        },
    }
}

fn build_call_arg(pair: pest::iterators::Pair<'_, Rule>) -> CallArg {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();

    match first.as_rule() {
        Rule::named_arg => {
            let span = span_of(&first);
            let mut named_inner = first.into_inner();
            let key = named_inner.next().unwrap().as_str().to_string();
            let value = build_expr(named_inner.next().unwrap());
            CallArg::Named { key, value, span }
        }
        _ => CallArg::Positional(build_expr(first)),
    }
}

fn build_expr_kind(pair: pest::iterators::Pair<'_, Rule>) -> ExprKind {
    match pair.as_rule() {
        Rule::implies_expr => build_implies_expr(pair).kind,
        Rule::or_expr => build_or_expr(pair).kind,
        Rule::and_expr => build_and_expr(pair).kind,
        Rule::not_expr => build_not_expr(pair).kind,
        Rule::cmp_expr => build_cmp_expr(pair).kind,
        Rule::add_expr => build_add_expr(pair).kind,
        Rule::primary => build_primary(pair).kind,
        _ => build_atom(pair).kind,
    }
}

// ── Helpers ──────────────────────────────────────────────────

fn parse_number_literal(s: &str) -> Literal {
    if s.contains('.') {
        Literal::Decimal(s.to_string())
    } else {
        Literal::Int(s.parse().unwrap_or(0))
    }
}

fn extract_string(pair: pest::iterators::Pair<'_, Rule>) -> String {
    pair.into_inner()
        .next()
        .map(|p| p.as_str().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_module() {
        let src = "module Foo\n";
        let file = parse_file(src).unwrap();
        assert_eq!(file.module.name, "Foo");
        assert!(file.items.is_empty());
    }

    #[test]
    fn parse_entity() {
        let src = r#"module Test

entity Account {
  id: UUID
  balance: Decimal(precision: 2)
  status: Active | Frozen | Closed
  notes: String?
}
"#;
        let file = parse_file(src).unwrap();
        assert_eq!(file.items.len(), 1);
        if let TopLevelItem::Entity(e) = &file.items[0] {
            assert_eq!(e.name, "Account");
            assert_eq!(e.fields.len(), 4);
            assert_eq!(e.fields[0].name, "id");
            assert!(e.fields[2].ty.optional == false);
            assert!(e.fields[3].ty.optional == true);
        } else {
            panic!("expected entity");
        }
    }

    #[test]
    fn parse_action_with_requires_ensures() {
        let src = r#"module Test

action Transfer {
  from: Account
  amount: Decimal(precision: 2)

  requires {
    from.status == Active
    amount > 0
  }

  ensures {
    from.balance == old(from.balance) - amount
  }
}
"#;
        let file = parse_file(src).unwrap();
        assert_eq!(file.items.len(), 1);
        if let TopLevelItem::Action(a) = &file.items[0] {
            assert_eq!(a.name, "Transfer");
            assert_eq!(a.params.len(), 2);
            assert_eq!(a.requires.as_ref().unwrap().conditions.len(), 2);
            assert_eq!(a.ensures.as_ref().unwrap().items.len(), 1);
        } else {
            panic!("expected action");
        }
    }

    #[test]
    fn parse_invariant() {
        let src = r#"module Test

invariant NoNegativeBalances {
  forall a: Account => a.balance >= 0
}
"#;
        let file = parse_file(src).unwrap();
        if let TopLevelItem::Invariant(inv) = &file.items[0] {
            assert_eq!(inv.name, "NoNegativeBalances");
            assert!(matches!(inv.body.kind, ExprKind::Quantifier { .. }));
        } else {
            panic!("expected invariant");
        }
    }

    #[test]
    fn parse_edge_cases() {
        let src = r#"module Test

edge_cases {
  when amount > 10000.00 => require_approval(level: "manager")
  when from == to => reject("Cannot transfer to same account")
}
"#;
        let file = parse_file(src).unwrap();
        if let TopLevelItem::EdgeCases(ec) = &file.items[0] {
            assert_eq!(ec.rules.len(), 2);
            assert_eq!(ec.rules[0].action.name, "require_approval");
            assert_eq!(ec.rules[1].action.name, "reject");
        } else {
            panic!("expected edge_cases");
        }
    }

    #[test]
    fn parse_transfer_example() {
        let src = include_str!("../../../examples/transfer.intent");
        let file = parse_file(src).unwrap();
        assert_eq!(file.module.name, "TransferFunds");
        // 2 entities + 2 actions + 2 invariants + 1 edge_cases = 7 items
        assert_eq!(file.items.len(), 7);
    }

    #[test]
    fn parse_auth_example() {
        let src = include_str!("../../../examples/auth.intent");
        let file = parse_file(src).unwrap();
        assert_eq!(file.module.name, "Authentication");
        // 2 entities + 2 actions + 2 invariants + 1 edge_cases = 7 items
        assert_eq!(file.items.len(), 7);
    }
}
