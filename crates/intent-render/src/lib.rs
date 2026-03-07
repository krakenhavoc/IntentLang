pub mod format;
pub mod html;
pub mod markdown;

use intent_parser::ast;

/// Format a [`Literal`] as a human-readable string.
pub fn format_literal(lit: &ast::Literal) -> String {
    match lit {
        ast::Literal::Null => "null".to_string(),
        ast::Literal::Bool(b) => b.to_string(),
        ast::Literal::Int(n) => n.to_string(),
        ast::Literal::Decimal(s) => s.clone(),
        ast::Literal::String(s) => format!("\"{}\"", s),
    }
}

/// Format a [`TypeExpr`] as a human-readable string.
pub fn format_type(ty: &ast::TypeExpr) -> String {
    let base = match &ty.ty {
        ast::TypeKind::Simple(name) => name.clone(),
        ast::TypeKind::Union(variants) => variants
            .iter()
            .map(|v| match v {
                ast::TypeKind::Simple(n) => n.clone(),
                _ => "...".to_string(),
            })
            .collect::<Vec<_>>()
            .join(" | "),
        ast::TypeKind::List(inner) => format!("List<{}>", format_type(inner)),
        ast::TypeKind::Set(inner) => format!("Set<{}>", format_type(inner)),
        ast::TypeKind::Map(k, v) => format!("Map<{}, {}>", format_type(k), format_type(v)),
        ast::TypeKind::Parameterized { name, params } => {
            let ps: Vec<String> = params
                .iter()
                .map(|p| format!("{}: {}", p.name, format_literal(&p.value)))
                .collect();
            format!("{}({})", name, ps.join(", "))
        }
    };
    if ty.optional {
        format!("{}?", base)
    } else {
        base
    }
}
