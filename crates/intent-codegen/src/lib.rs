//! Skeleton code generator for IntentLang specifications.
//!
//! Generates typed stubs in Rust, TypeScript, Python, Go, Java, C#, or Swift
//! from a parsed `.intent` AST. Entities become structs/classes/dataclasses/records,
//! actions become function signatures with contract documentation.

pub mod csharp;
pub mod go;
pub mod java;
pub mod openapi;
pub mod python;
pub mod rust;
pub mod rust_tests;
pub mod swift;
pub mod test_harness;
mod types;
pub mod typescript;

#[cfg(test)]
mod codegen_tests;
#[cfg(test)]
mod openapi_tests;

use intent_parser::ast::{
    self, ArithOp, CallArg, CmpOp, EnsuresItem, ExprKind, Literal, PropValue, QuantifierKind,
};

/// Target language for code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Go,
    Java,
    CSharp,
    Swift,
}

/// Generate skeleton code from a parsed intent file.
pub fn generate(file: &ast::File, lang: Language) -> String {
    match lang {
        Language::Rust => rust::generate(file),
        Language::TypeScript => typescript::generate(file),
        Language::Python => python::generate(file),
        Language::Go => go::generate(file),
        Language::Java => java::generate(file),
        Language::CSharp => csharp::generate(file),
        Language::Swift => swift::generate(file),
    }
}

/// Convert a PascalCase name to snake_case.
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

/// Convert a PascalCase name to camelCase.
pub fn to_camel_case(s: &str) -> String {
    let snake = to_snake_case(s);
    let mut result = String::new();
    let mut capitalize_next = false;
    for (i, ch) in snake.chars().enumerate() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else if i == 0 {
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

/// Format an expression as a human-readable comment string.
pub fn format_expr(expr: &ast::Expr) -> String {
    match &expr.kind {
        ExprKind::Implies(l, r) => format!("{} => {}", format_expr(l), format_expr(r)),
        ExprKind::Or(l, r) => format!("{} || {}", format_expr(l), format_expr(r)),
        ExprKind::And(l, r) => format!("{} && {}", format_expr(l), format_expr(r)),
        ExprKind::Not(e) => format!("!{}", format_expr(e)),
        ExprKind::Compare { left, op, right } => {
            let op_str = match op {
                CmpOp::Eq => "==",
                CmpOp::Ne => "!=",
                CmpOp::Lt => "<",
                CmpOp::Gt => ">",
                CmpOp::Le => "<=",
                CmpOp::Ge => ">=",
            };
            format!("{} {} {}", format_expr(left), op_str, format_expr(right))
        }
        ExprKind::Arithmetic { left, op, right } => {
            let op_str = match op {
                ArithOp::Add => "+",
                ArithOp::Sub => "-",
            };
            format!("{} {} {}", format_expr(left), op_str, format_expr(right))
        }
        ExprKind::Old(e) => format!("old({})", format_expr(e)),
        ExprKind::Quantifier {
            kind,
            binding,
            ty,
            body,
        } => {
            let kw = match kind {
                QuantifierKind::Forall => "forall",
                QuantifierKind::Exists => "exists",
            };
            format!("{kw} {binding}: {ty} => {}", format_expr(body))
        }
        ExprKind::Call { name, args } => {
            let args_str = format_call_args(args);
            format!("{name}({args_str})")
        }
        ExprKind::FieldAccess { root, fields } => {
            format!("{}.{}", format_expr(root), fields.join("."))
        }
        ExprKind::List(items) => {
            let inner: Vec<_> = items.iter().map(format_expr).collect();
            format!("[{}]", inner.join(", "))
        }
        ExprKind::Ident(name) => name.clone(),
        ExprKind::Literal(lit) => format_literal(lit),
    }
}

fn format_call_args(args: &[CallArg]) -> String {
    args.iter()
        .map(|a| match a {
            CallArg::Named { key, value, .. } => format!("{key}: {}", format_expr(value)),
            CallArg::Positional(e) => format_expr(e),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_literal(lit: &Literal) -> String {
    match lit {
        Literal::Null => "null".to_string(),
        Literal::Bool(b) => b.to_string(),
        Literal::Int(n) => n.to_string(),
        Literal::Decimal(s) => s.clone(),
        Literal::String(s) => format!("\"{s}\""),
    }
}

/// Format an ensures item as a comment string.
pub fn format_ensures_item(item: &EnsuresItem) -> String {
    match item {
        EnsuresItem::Expr(e) => format_expr(e),
        EnsuresItem::When(w) => {
            format!(
                "when {} => {}",
                format_expr(&w.condition),
                format_expr(&w.consequence)
            )
        }
    }
}

/// Format a property value as a human-readable string.
pub fn format_prop_value(val: &PropValue) -> String {
    match val {
        PropValue::Literal(lit) => format_literal(lit),
        PropValue::Ident(name) => name.clone(),
        PropValue::List(items) => {
            let inner: Vec<_> = items.iter().map(format_prop_value).collect();
            format!("[{}]", inner.join(", "))
        }
        PropValue::Object(entries) => {
            let inner: Vec<_> = entries
                .iter()
                .map(|(k, v)| format!("{k}: {}", format_prop_value(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
    }
}

/// Render doc block lines as a joined string.
pub fn doc_text(doc: &ast::DocBlock) -> String {
    doc.lines.join("\n")
}

/// File extension for a language.
pub fn file_extension(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "rs",
        Language::TypeScript => "ts",
        Language::Python => "py",
        Language::Go => "go",
        Language::Java => "java",
        Language::CSharp => "cs",
        Language::Swift => "swift",
    }
}

/// Output file name for a module in the target language.
pub fn output_filename(module_name: &str, lang: Language) -> String {
    match lang {
        Language::Rust | Language::Python | Language::Go => {
            format!("{}.{}", to_snake_case(module_name), file_extension(lang))
        }
        Language::TypeScript => {
            format!("{}.{}", to_camel_case(module_name), file_extension(lang))
        }
        Language::Java | Language::CSharp | Language::Swift => {
            format!("{}.{}", module_name, file_extension(lang))
        }
    }
}
