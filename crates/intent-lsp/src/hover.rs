//! Hover provider: show doc blocks, type info, and keyword help on hover.

use intent_parser::ast::{self, ExprKind, TopLevelItem, TypeKind};
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use crate::document::Document;

/// Provide hover information for the symbol at the given byte offset.
pub fn hover_at(doc: &Document, offset: usize) -> Option<Hover> {
    let ast = doc.ast.as_ref()?;

    // 1. Check if cursor is on a keyword.
    if let Some(hover) = keyword_hover(&doc.source, offset) {
        return Some(hover);
    }

    // 2. Check if cursor is on a declaration name or reference.
    if let Some(name) = find_symbol_name(ast, offset, &doc.source) {
        return symbol_hover(ast, &name);
    }

    // 3. Check if cursor is on a field name.
    if let Some(hover) = field_hover(ast, offset, &doc.source, doc) {
        return Some(hover);
    }

    None
}

/// Check if the cursor is on a known keyword and provide help text.
fn keyword_hover(source: &str, offset: usize) -> Option<Hover> {
    let keywords: &[(&str, &str)] = &[
        (
            "module",
            "**module** — Declares the module name for this file.\n\n```intent\nmodule ModuleName\n```",
        ),
        (
            "entity",
            "**entity** — Defines a data structure with typed fields.\n\n```intent\nentity Account {\n  id: UUID\n  balance: Decimal(precision: 2)\n}\n```",
        ),
        (
            "action",
            "**action** — Defines an operation with parameters and contracts.\n\n```intent\naction Transfer {\n  amount: Decimal(precision: 2)\n  requires { amount > 0 }\n  ensures { ... }\n}\n```",
        ),
        (
            "invariant",
            "**invariant** — Defines a universal constraint that must always hold.\n\n```intent\ninvariant NonNegativeBalance {\n  forall a: Account => a.balance >= 0\n}\n```",
        ),
        (
            "edge_cases",
            "**edge_cases** — Defines handlers for exceptional conditions.\n\n```intent\nedge_cases {\n  when condition => HandleAction()\n}\n```",
        ),
        (
            "requires",
            "**requires** — Preconditions that must be true before an action executes.",
        ),
        (
            "ensures",
            "**ensures** — Postconditions that must be true after an action executes.\n\nUse `old(expr)` to reference pre-state values.",
        ),
        (
            "properties",
            "**properties** — Key-value metadata for an action.",
        ),
        (
            "forall",
            "**forall** — Universal quantifier: the predicate must hold for all values of the bound type.\n\n```intent\nforall x: Type => predicate\n```",
        ),
        (
            "exists",
            "**exists** — Existential quantifier: the predicate must hold for at least one value.\n\n```intent\nexists x: Type => predicate\n```",
        ),
        (
            "old",
            "**old(expr)** — References the value of an expression before the action executed.\n\nOnly valid in `ensures` blocks.",
        ),
        (
            "use",
            "**use** — Import definitions from another module.\n\n```intent\nuse OtherModule          // import all\nuse OtherModule.Account  // import specific item\n```",
        ),
    ];

    for &(kw, help) in keywords {
        if let Some(start) = find_word_at(source, offset, kw)
            && start <= offset
            && offset <= start + kw.len()
        {
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: help.to_string(),
                }),
                range: None,
            });
        }
    }

    None
}

/// Find the start of a word in source at the given offset.
fn find_word_at(source: &str, offset: usize, word: &str) -> Option<usize> {
    // Find the word boundary containing offset.
    let start = source[..offset.min(source.len())]
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| i + 1)
        .unwrap_or(0);

    let end = source[offset.min(source.len())..]
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| i + offset)
        .unwrap_or(source.len());

    let found = &source[start..end];
    if found == word { Some(start) } else { None }
}

/// Find the name of the symbol at the given offset.
fn find_symbol_name(ast: &ast::File, offset: usize, source: &str) -> Option<String> {
    // Check declaration names.
    for item in &ast.items {
        let (name, span) = match item {
            TopLevelItem::Entity(e) => (&e.name, e.span),
            TopLevelItem::Action(a) => (&a.name, a.span),
            TopLevelItem::Invariant(i) => (&i.name, i.span),
            TopLevelItem::EdgeCases(_) | TopLevelItem::Test(_) => continue,
        };
        if let Some(rel) = source[span.start..span.end.min(source.len())].find(name.as_str()) {
            let abs_start = span.start + rel;
            if offset >= abs_start && offset <= abs_start + name.len() {
                return Some(name.clone());
            }
        }
    }

    // Check type references in fields/params.
    for item in &ast.items {
        match item {
            TopLevelItem::Entity(e) => {
                for field in &e.fields {
                    if let Some(name) = find_type_name_at(&field.ty, offset) {
                        return Some(name);
                    }
                }
            }
            TopLevelItem::Action(a) => {
                for param in &a.params {
                    if let Some(name) = find_type_name_at(&param.ty, offset) {
                        return Some(name);
                    }
                }
            }
            _ => {}
        }
    }

    // Check identifiers in expressions.
    for item in &ast.items {
        match item {
            TopLevelItem::Action(a) => {
                if let Some(ref req) = a.requires {
                    for expr in &req.conditions {
                        if let Some(name) = find_uppercase_ident(expr, offset) {
                            return Some(name);
                        }
                    }
                }
                if let Some(ref ens) = a.ensures {
                    for item in &ens.items {
                        let exprs: Vec<&ast::Expr> = match item {
                            ast::EnsuresItem::Expr(e) => vec![e],
                            ast::EnsuresItem::When(w) => vec![&w.condition, &w.consequence],
                        };
                        for expr in exprs {
                            if let Some(name) = find_uppercase_ident(expr, offset) {
                                return Some(name);
                            }
                        }
                    }
                }
            }
            TopLevelItem::Invariant(inv) => {
                if let Some(name) = find_uppercase_ident(&inv.body, offset) {
                    return Some(name);
                }
            }
            _ => {}
        }
    }

    None
}

fn find_type_name_at(ty: &ast::TypeExpr, offset: usize) -> Option<String> {
    if offset < ty.span.start || offset >= ty.span.end {
        return None;
    }
    match &ty.ty {
        TypeKind::Simple(name) => Some(name.clone()),
        TypeKind::List(inner) | TypeKind::Set(inner) => find_type_name_at(inner, offset),
        TypeKind::Map(k, v) => {
            find_type_name_at(k, offset).or_else(|| find_type_name_at(v, offset))
        }
        TypeKind::Parameterized { name, .. } => Some(name.clone()),
        TypeKind::Union(variants) => {
            for v in variants {
                if let TypeKind::Simple(name) = v {
                    return Some(name.clone());
                }
            }
            None
        }
    }
}

fn find_uppercase_ident(expr: &ast::Expr, offset: usize) -> Option<String> {
    if offset < expr.span.start || offset >= expr.span.end {
        return None;
    }
    match &expr.kind {
        ExprKind::Ident(name) if name.starts_with(char::is_uppercase) => Some(name.clone()),
        ExprKind::Quantifier { ty, .. } if ty.starts_with(char::is_uppercase) => Some(ty.clone()),
        _ => {
            let mut result = None;
            expr.for_each_child(|child| {
                if result.is_none() {
                    result = find_uppercase_ident(child, offset);
                }
            });
            result
        }
    }
}

/// Build hover content for a symbol name.
fn symbol_hover(ast: &ast::File, name: &str) -> Option<Hover> {
    // Check built-in types.
    let builtins: &[(&str, &str)] = &[
        ("UUID", "Built-in type: universally unique identifier"),
        ("String", "Built-in type: text string"),
        ("Int", "Built-in type: integer number"),
        (
            "Decimal",
            "Built-in type: decimal number with configurable precision\n\n```intent\nDecimal(precision: 2)\n```",
        ),
        ("Bool", "Built-in type: boolean (true/false)"),
        ("DateTime", "Built-in type: date and time"),
        (
            "CurrencyCode",
            "Built-in domain type: ISO 4217 currency code",
        ),
        ("Email", "Built-in domain type: email address"),
        ("URL", "Built-in domain type: URL"),
    ];

    for &(builtin, desc) in builtins {
        if name == builtin {
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: desc.to_string(),
                }),
                range: None,
            });
        }
    }

    // Check user-defined entities, actions, invariants.
    for item in &ast.items {
        match item {
            TopLevelItem::Entity(e) if e.name == name => {
                let mut text = format!("**entity** `{}`", name);
                if let Some(ref doc) = e.doc {
                    text.push_str("\n\n");
                    text.push_str(&doc.lines.join("\n"));
                }
                text.push_str("\n\n```intent\n");
                text.push_str(&format!("entity {} {{\n", name));
                for field in &e.fields {
                    text.push_str(&format!("  {}: {}\n", field.name, format_type(&field.ty)));
                }
                text.push_str("}\n```");
                return Some(make_hover(text));
            }
            TopLevelItem::Action(a) if a.name == name => {
                let mut text = format!("**action** `{}`", name);
                if let Some(ref doc) = a.doc {
                    text.push_str("\n\n");
                    text.push_str(&doc.lines.join("\n"));
                }
                text.push_str("\n\nParameters:\n");
                for param in &a.params {
                    text.push_str(&format!(
                        "- `{}`: `{}`\n",
                        param.name,
                        format_type(&param.ty)
                    ));
                }
                return Some(make_hover(text));
            }
            TopLevelItem::Invariant(i) if i.name == name => {
                let mut text = format!("**invariant** `{}`", name);
                if let Some(ref doc) = i.doc {
                    text.push_str("\n\n");
                    text.push_str(&doc.lines.join("\n"));
                }
                return Some(make_hover(text));
            }
            _ => {}
        }
    }

    None
}

/// Format a type expression for display.
pub fn format_type(ty: &ast::TypeExpr) -> String {
    let base = match &ty.ty {
        TypeKind::Simple(name) => name.clone(),
        TypeKind::List(inner) => format!("List<{}>", format_type(inner)),
        TypeKind::Set(inner) => format!("Set<{}>", format_type(inner)),
        TypeKind::Map(k, v) => format!("Map<{}, {}>", format_type(k), format_type(v)),
        TypeKind::Union(variants) => variants
            .iter()
            .map(|v| match v {
                TypeKind::Simple(name) => name.clone(),
                _ => "...".to_string(),
            })
            .collect::<Vec<_>>()
            .join(" | "),
        TypeKind::Parameterized { name, params } => {
            let p: Vec<String> = params
                .iter()
                .map(|p| format!("{}: {}", p.name, format_literal(&p.value)))
                .collect();
            format!("{}({})", name, p.join(", "))
        }
    };
    if ty.optional {
        format!("{}?", base)
    } else {
        base
    }
}

fn format_literal(lit: &ast::Literal) -> String {
    match lit {
        ast::Literal::Null => "null".to_string(),
        ast::Literal::Bool(b) => b.to_string(),
        ast::Literal::Int(i) => i.to_string(),
        ast::Literal::Decimal(d) => d.clone(),
        ast::Literal::String(s) => format!("\"{}\"", s),
    }
}

/// Check if cursor is on a field name and provide its type info.
fn field_hover(ast: &ast::File, offset: usize, source: &str, _doc: &Document) -> Option<Hover> {
    for item in &ast.items {
        if let TopLevelItem::Entity(e) = item {
            for field in &e.fields {
                if offset >= field.span.start && offset < field.span.end {
                    // Check if cursor is on the field name part (before the colon).
                    let field_text = &source[field.span.start..field.span.end.min(source.len())];
                    if let Some(colon_pos) = field_text.find(':') {
                        let name_end = field.span.start + colon_pos;
                        if offset < name_end {
                            let text = format!(
                                "`{}.{}`: `{}`",
                                e.name,
                                field.name,
                                format_type(&field.ty)
                            );
                            return Some(make_hover(text));
                        }
                    }
                }
            }
        }
        if let TopLevelItem::Action(a) = item {
            for param in &a.params {
                if offset >= param.span.start && offset < param.span.end {
                    let param_text = &source[param.span.start..param.span.end.min(source.len())];
                    if let Some(colon_pos) = param_text.find(':') {
                        let name_end = param.span.start + colon_pos;
                        if offset < name_end {
                            let text = format!(
                                "`{}.{}`: `{}`",
                                a.name,
                                param.name,
                                format_type(&param.ty)
                            );
                            return Some(make_hover(text));
                        }
                    }
                }
            }
        }
    }
    None
}

fn make_hover(text: String) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: text,
        }),
        range: None,
    }
}
