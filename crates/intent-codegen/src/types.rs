//! Type mapping from IntentLang types to target language type strings.

use intent_parser::ast::{TypeExpr, TypeKind};

use crate::Language;

/// Map an IntentLang type expression to a target language type string.
pub fn map_type(ty: &TypeExpr, lang: &Language) -> String {
    let base = map_type_kind(&ty.ty, lang);
    if ty.optional {
        match lang {
            Language::Rust => format!("Option<{base}>"),
            Language::TypeScript => format!("{base} | null"),
            Language::Python => format!("{base} | None"),
        }
    } else {
        base
    }
}

fn map_type_kind(kind: &TypeKind, lang: &Language) -> String {
    match kind {
        TypeKind::Simple(name) => map_simple(name, lang),
        TypeKind::Union(variants) => map_union(variants, lang),
        TypeKind::List(inner) => {
            let inner_ty = map_type(inner, lang);
            match lang {
                Language::Rust => format!("Vec<{inner_ty}>"),
                Language::TypeScript => format!("{inner_ty}[]"),
                Language::Python => format!("list[{inner_ty}]"),
            }
        }
        TypeKind::Set(inner) => {
            let inner_ty = map_type(inner, lang);
            match lang {
                Language::Rust => format!("HashSet<{inner_ty}>"),
                Language::TypeScript => format!("Set<{inner_ty}>"),
                Language::Python => format!("set[{inner_ty}]"),
            }
        }
        TypeKind::Map(k, v) => {
            let key_ty = map_type(k, lang);
            let val_ty = map_type(v, lang);
            match lang {
                Language::Rust => format!("HashMap<{key_ty}, {val_ty}>"),
                Language::TypeScript => format!("Map<{key_ty}, {val_ty}>"),
                Language::Python => format!("dict[{key_ty}, {val_ty}]"),
            }
        }
        TypeKind::Parameterized { name, .. } => map_simple(name, lang),
    }
}

fn map_simple(name: &str, lang: &Language) -> String {
    match lang {
        Language::Rust => match name {
            "UUID" => "Uuid",
            "String" => "String",
            "Int" => "i64",
            "Decimal" => "Decimal",
            "Bool" => "bool",
            "DateTime" => "DateTime<Utc>",
            "CurrencyCode" | "Email" | "URL" => "String",
            other => other, // entity references stay as-is
        },
        Language::TypeScript => match name {
            "UUID" | "String" | "CurrencyCode" | "Email" | "URL" => "string",
            "Int" | "Decimal" => "number",
            "Bool" => "boolean",
            "DateTime" => "Date",
            other => other,
        },
        Language::Python => match name {
            "UUID" => "uuid.UUID",
            "String" | "CurrencyCode" | "Email" | "URL" => "str",
            "Int" => "int",
            "Decimal" => "Decimal",
            "Bool" => "bool",
            "DateTime" => "datetime",
            other => other,
        },
    }
    .to_string()
}

fn map_union(variants: &[TypeKind], lang: &Language) -> String {
    let names: Vec<&str> = variants
        .iter()
        .filter_map(|v| match v {
            TypeKind::Simple(n) => Some(n.as_str()),
            _ => None,
        })
        .collect();
    match lang {
        Language::Rust => names.join(" | "), // placeholder, actual enum generated separately
        Language::TypeScript => names
            .iter()
            .map(|n| format!("\"{n}\""))
            .collect::<Vec<_>>()
            .join(" | "),
        Language::Python => {
            let inner = names
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Literal[{inner}]")
        }
    }
}
