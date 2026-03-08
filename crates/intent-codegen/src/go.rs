//! Go skeleton code generator.

use intent_parser::ast;

use crate::types::map_type;
use crate::{Language, doc_text, format_ensures_item, format_expr, to_snake_case};

/// Go reserved keywords that cannot be used as identifiers.
const GO_KEYWORDS: &[&str] = &[
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
];

/// Escape a Go identifier if it collides with a reserved keyword.
fn safe_ident(name: &str) -> String {
    let snake = to_snake_case(name);
    if GO_KEYWORDS.contains(&snake.as_str()) {
        format!("{snake}_")
    } else {
        snake
    }
}

/// Capitalize the first character of a string (for exported Go names).
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Convert a snake_case or lowercase name to PascalCase (exported Go identifier).
fn to_pascal_case(s: &str) -> String {
    s.split('_').map(capitalize).collect::<String>()
}

/// Convert a field name to a JSON struct tag value (camelCase).
fn to_json_tag(s: &str) -> String {
    crate::to_camel_case(s)
}

/// Generate Go skeleton code from a parsed intent file.
pub fn generate(file: &ast::File) -> String {
    let lang = Language::Go;
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "// Code generated from {}.intent. DO NOT EDIT.\n",
        file.module.name
    ));
    if let Some(doc) = &file.doc {
        out.push('\n');
        for line in &doc.lines {
            out.push_str(&format!("// {line}\n"));
        }
    }
    out.push('\n');

    // Package declaration (lowercase module name)
    let pkg_name = file.module.name.to_lowercase();
    out.push_str(&format!("package {pkg_name}\n\n"));

    // Imports
    let imports = generate_imports(file);
    if !imports.is_empty() {
        out.push_str(&imports);
        out.push('\n');
    }

    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => generate_entity(&mut out, e, &lang),
            ast::TopLevelItem::Action(a) => generate_action(&mut out, a, &lang),
            ast::TopLevelItem::Invariant(inv) => generate_invariant(&mut out, inv),
            ast::TopLevelItem::EdgeCases(ec) => generate_edge_cases(&mut out, ec),
            ast::TopLevelItem::Test(_) => {}
        }
    }

    out
}

fn generate_imports(file: &ast::File) -> String {
    let source = collect_type_names(file);
    let has_union = file.items.iter().any(|item| {
        if let ast::TopLevelItem::Entity(e) = item {
            e.fields
                .iter()
                .any(|f| matches!(f.ty.ty, ast::TypeKind::Union(_)))
        } else {
            false
        }
    });
    let has_action = file
        .items
        .iter()
        .any(|item| matches!(item, ast::TopLevelItem::Action(_)));

    let mut imports = Vec::new();

    if has_action || has_union {
        // errors and fmt are stdlib, always available
        if has_action {
            imports.push("\"errors\"");
        }
        if has_union {
            imports.push("\"fmt\"");
        }
    }
    if source.contains("DateTime") {
        imports.push("\"time\"");
    }
    // External packages
    if source.contains("Decimal") {
        imports.push("\"github.com/shopspring/decimal\"");
    }
    if source.contains("UUID") {
        imports.push("\"github.com/google/uuid\"");
    }

    if imports.is_empty() {
        return String::new();
    }

    if imports.len() == 1 {
        return format!("import {}\n", imports[0]);
    }

    let mut out = String::from("import (\n");
    for imp in &imports {
        out.push_str(&format!("\t{imp}\n"));
    }
    out.push_str(")\n");
    out
}

/// Collect all type names as a single string for import detection.
fn collect_type_names(file: &ast::File) -> String {
    let mut names = String::new();
    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => {
                for f in &e.fields {
                    collect_type_name(&f.ty, &mut names);
                }
            }
            ast::TopLevelItem::Action(a) => {
                for p in &a.params {
                    collect_type_name(&p.ty, &mut names);
                }
            }
            _ => {}
        }
    }
    names
}

fn collect_type_name(ty: &ast::TypeExpr, out: &mut String) {
    match &ty.ty {
        ast::TypeKind::Simple(n) => {
            out.push_str(n);
            out.push(' ');
        }
        ast::TypeKind::Parameterized { name, .. } => {
            out.push_str(name);
            out.push(' ');
        }
        ast::TypeKind::List(inner) | ast::TypeKind::Set(inner) => collect_type_name(inner, out),
        ast::TypeKind::Map(k, v) => {
            collect_type_name(k, out);
            collect_type_name(v, out);
        }
        ast::TypeKind::Union(_) => {}
    }
}

fn generate_entity(out: &mut String, entity: &ast::EntityDecl, lang: &Language) {
    // Emit union type aliases with const blocks for union-typed fields
    for field in &entity.fields {
        if let ast::TypeKind::Union(variants) = &field.ty.ty {
            let type_name = format!("{}{}", entity.name, capitalize(&field.name));
            generate_union_type(out, &type_name, variants);
        }
    }

    // Doc comment
    if let Some(doc) = &entity.doc {
        for line in doc_text(doc).lines() {
            out.push_str(&format!("// {line}\n"));
        }
    }

    out.push_str(&format!("type {} struct {{\n", entity.name));

    for field in &entity.fields {
        let field_name = to_pascal_case(&field.name);
        let json_tag = to_json_tag(&field.name);
        let ty = if let ast::TypeKind::Union(_) = &field.ty.ty {
            let type_name = format!("{}{}", entity.name, capitalize(&field.name));
            if field.ty.optional {
                format!("*{type_name}")
            } else {
                type_name
            }
        } else {
            map_type(&field.ty, lang)
        };
        out.push_str(&format!("\t{field_name} {ty} `json:\"{json_tag}\"`\n"));
    }

    out.push_str("}\n\n");
}

fn generate_union_type(out: &mut String, name: &str, variants: &[ast::TypeKind]) {
    let names: Vec<&str> = variants
        .iter()
        .filter_map(|v| match v {
            ast::TypeKind::Simple(n) => Some(n.as_str()),
            _ => None,
        })
        .collect();

    out.push_str(&format!(
        "// {name} represents the allowed values for this field.\n"
    ));
    out.push_str(&format!("type {name} string\n\n"));

    out.push_str("const (\n");
    for n in &names {
        let const_name = format!("{name}{n}");
        out.push_str(&format!("\t{const_name} {name} = \"{n}\"\n"));
    }
    out.push_str(")\n\n");

    // Validate method
    out.push_str(&format!(
        "// Valid returns true if v is a known {name} value.\n"
    ));
    out.push_str(&format!("func (v {name}) Valid() bool {{\n"));
    out.push_str("\tswitch v {\n");
    out.push_str(&format!(
        "\tcase {}:\n",
        names
            .iter()
            .map(|n| format!("{name}{n}"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push_str("\t\treturn true\n");
    out.push_str("\tdefault:\n");
    out.push_str("\t\treturn false\n");
    out.push_str("\t}\n");
    out.push_str("}\n\n");

    // String method
    out.push_str(&format!("func (v {name}) String() string {{\n"));
    out.push_str("\treturn string(v)\n");
    out.push_str("}\n\n");

    // UnmarshalText for safe JSON deserialization
    out.push_str(&format!(
        "// UnmarshalText implements encoding.TextUnmarshaler for {name}.\n"
    ));
    out.push_str(&format!(
        "func (v *{name}) UnmarshalText(data []byte) error {{\n"
    ));
    out.push_str(&format!("\ts := {name}(data)\n"));
    out.push_str("\tif !s.Valid() {\n");
    out.push_str(&format!(
        "\t\treturn fmt.Errorf(\"invalid {name}: %q\", string(data))\n"
    ));
    out.push_str("\t}\n");
    out.push_str("\t*v = s\n");
    out.push_str("\treturn nil\n");
    out.push_str("}\n\n");
}

fn generate_action(out: &mut String, action: &ast::ActionDecl, lang: &Language) {
    let fn_name = to_pascal_case(&to_snake_case(&action.name));

    // Doc comment
    if let Some(doc) = &action.doc {
        out.push_str(&format!("// {fn_name} — {}\n", doc_text(doc)));
    }

    // Requires
    if let Some(req) = &action.requires {
        out.push_str("//\n// Requires:\n");
        for cond in &req.conditions {
            out.push_str(&format!("//   - {}\n", format_expr(cond)));
        }
    }

    // Ensures
    if let Some(ens) = &action.ensures {
        out.push_str("//\n// Ensures:\n");
        for item in &ens.items {
            out.push_str(&format!("//   - {}\n", format_ensures_item(item)));
        }
    }

    // Properties
    if let Some(props) = &action.properties {
        out.push_str("//\n// Properties:\n");
        for entry in &props.entries {
            out.push_str(&format!(
                "//   - {}: {}\n",
                entry.key,
                crate::format_prop_value(&entry.value)
            ));
        }
    }

    // Function signature
    let params: Vec<String> = action
        .params
        .iter()
        .map(|p| {
            let ty = map_type(&p.ty, lang);
            format!("{} {ty}", safe_ident(&p.name))
        })
        .collect();

    out.push_str(&format!("func {fn_name}({}) error {{\n", params.join(", ")));
    out.push_str(&format!(
        "\treturn errors.New(\"TODO: implement {fn_name}\")\n"
    ));
    out.push_str("}\n\n");
}

fn generate_invariant(out: &mut String, inv: &ast::InvariantDecl) {
    out.push_str(&format!("// Invariant: {}\n", inv.name));
    if let Some(doc) = &inv.doc {
        for line in doc_text(doc).lines() {
            out.push_str(&format!("// {line}\n"));
        }
    }
    out.push_str(&format!("// {}\n\n", format_expr(&inv.body)));
}

fn generate_edge_cases(out: &mut String, ec: &ast::EdgeCasesDecl) {
    out.push_str("// Edge cases:\n");
    for rule in &ec.rules {
        out.push_str(&format!(
            "// when {} => {}()\n",
            format_expr(&rule.condition),
            rule.action.name,
        ));
    }
    out.push('\n');
}
