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
            Language::Go => format!("*{base}"),
            Language::Java => map_java_optional(&base),
            Language::CSharp => format!("{base}?"),
            Language::Swift => format!("{base}?"),
        }
    } else {
        base
    }
}

/// Java optional types: box primitives, leave reference types as-is.
fn map_java_optional(base: &str) -> String {
    match base {
        "long" => "Long".to_string(),
        "boolean" => "Boolean".to_string(),
        other => other.to_string(),
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
                Language::Go => format!("[]{inner_ty}"),
                Language::Java => format!("List<{}>", box_java_primitive(&inner_ty)),
                Language::CSharp => format!("List<{inner_ty}>"),
                Language::Swift => format!("[{inner_ty}]"),
            }
        }
        TypeKind::Set(inner) => {
            let inner_ty = map_type(inner, lang);
            match lang {
                Language::Rust => format!("HashSet<{inner_ty}>"),
                Language::TypeScript => format!("Set<{inner_ty}>"),
                Language::Python => format!("set[{inner_ty}]"),
                Language::Go => format!("map[{inner_ty}]struct{{}}"),
                Language::Java => format!("Set<{}>", box_java_primitive(&inner_ty)),
                Language::CSharp => format!("HashSet<{inner_ty}>"),
                Language::Swift => format!("Set<{inner_ty}>"),
            }
        }
        TypeKind::Map(k, v) => {
            let key_ty = map_type(k, lang);
            let val_ty = map_type(v, lang);
            match lang {
                Language::Rust => format!("HashMap<{key_ty}, {val_ty}>"),
                Language::TypeScript => format!("Map<{key_ty}, {val_ty}>"),
                Language::Python => format!("dict[{key_ty}, {val_ty}]"),
                Language::Go => format!("map[{key_ty}]{val_ty}"),
                Language::Java => format!(
                    "Map<{}, {}>",
                    box_java_primitive(&key_ty),
                    box_java_primitive(&val_ty)
                ),
                Language::CSharp => format!("Dictionary<{key_ty}, {val_ty}>"),
                Language::Swift => format!("[{key_ty}: {val_ty}]"),
            }
        }
        TypeKind::Parameterized { name, .. } => map_simple(name, lang),
    }
}

/// Box Java primitive types for use in generics (List<Long> not List<long>).
fn box_java_primitive(ty: &str) -> String {
    match ty {
        "long" => "Long".to_string(),
        "boolean" => "Boolean".to_string(),
        "int" => "Integer".to_string(),
        other => other.to_string(),
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
        Language::Go => match name {
            "UUID" => "uuid.UUID",
            "String" | "CurrencyCode" | "Email" | "URL" => "string",
            "Int" => "int64",
            "Decimal" => "decimal.Decimal",
            "Bool" => "bool",
            "DateTime" => "time.Time",
            other => other,
        },
        Language::Java => match name {
            "UUID" => "UUID",
            "String" | "CurrencyCode" | "Email" | "URL" => "String",
            "Int" => "long",
            "Decimal" => "BigDecimal",
            "Bool" => "boolean",
            "DateTime" => "Instant",
            other => other,
        },
        Language::CSharp => match name {
            "UUID" => "Guid",
            "String" | "CurrencyCode" | "Email" | "URL" => "string",
            "Int" => "long",
            "Decimal" => "decimal",
            "Bool" => "bool",
            "DateTime" => "DateTimeOffset",
            other => other,
        },
        Language::Swift => match name {
            "UUID" => "UUID",
            "String" | "CurrencyCode" | "Email" | "URL" => "String",
            "Int" => "Int",
            "Decimal" => "Decimal",
            "Bool" => "Bool",
            "DateTime" => "Date",
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
        Language::Go => names.join(" | "), // placeholder, actual const block generated separately
        Language::Java => names.join(" | "), // placeholder, actual enum generated separately
        Language::CSharp => names.join(" | "), // placeholder, actual enum generated separately
        Language::Swift => names.join(" | "), // placeholder, actual enum generated separately
    }
}
