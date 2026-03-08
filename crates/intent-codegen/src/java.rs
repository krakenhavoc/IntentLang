//! Java skeleton code generator.
//!
//! Generates Java 16+ records for entities, enums for union types,
//! and static methods for actions inside a module-level final class.

use intent_parser::ast;

use crate::types::map_type;
use crate::{Language, doc_text, format_ensures_item, format_expr, to_camel_case};

/// Java reserved keywords that cannot be used as identifiers.
const JAVA_KEYWORDS: &[&str] = &[
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "try",
    "void",
    "volatile",
    "while",
];

/// Escape a Java identifier if it collides with a reserved keyword.
fn safe_ident(name: &str) -> String {
    let camel = to_camel_case(name);
    if JAVA_KEYWORDS.contains(&camel.as_str()) {
        format!("{camel}_")
    } else {
        camel
    }
}

/// Generate Java skeleton code from a parsed intent file.
pub fn generate(file: &ast::File) -> String {
    let lang = Language::Java;
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "// Generated from {}.intent. DO NOT EDIT.\n",
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
    out.push_str(&format!("package {pkg_name};\n\n"));

    // Imports
    let imports = generate_imports(file);
    if !imports.is_empty() {
        out.push_str(&imports);
        out.push('\n');
    }

    // Module class wrapper
    out.push_str(&format!("public final class {} {{\n\n", file.module.name));
    out.push_str(&format!(
        "    private {}() {{}} // Prevent instantiation\n\n",
        file.module.name
    ));

    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(e) => generate_entity(&mut out, e, &lang),
            ast::TopLevelItem::Action(a) => generate_action(&mut out, a, &lang),
            ast::TopLevelItem::Invariant(inv) => generate_invariant(&mut out, inv),
            ast::TopLevelItem::EdgeCases(ec) => generate_edge_cases(&mut out, ec),
            ast::TopLevelItem::Test(_) => {}
        }
    }

    // Close module class
    out.push_str("}\n");
    out
}

fn generate_imports(file: &ast::File) -> String {
    let source = collect_type_names(file);
    let has_action = file
        .items
        .iter()
        .any(|item| matches!(item, ast::TopLevelItem::Action(_)));

    let mut imports = Vec::new();

    if source.contains("UUID") {
        imports.push("import java.util.UUID;");
    }
    if source.contains("Decimal") {
        imports.push("import java.math.BigDecimal;");
    }
    if source.contains("DateTime") {
        imports.push("import java.time.Instant;");
    }
    if source.contains("List<") {
        imports.push("import java.util.List;");
    }
    if source.contains("Set<") {
        imports.push("import java.util.Set;");
    }
    if source.contains("Map<") {
        imports.push("import java.util.Map;");
    }
    if has_action {
        // UnsupportedOperationException is in java.lang, no import needed
    }

    if imports.is_empty() {
        return String::new();
    }

    imports.join("\n") + "\n"
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
        ast::TypeKind::List(inner) => {
            out.push_str("List<");
            collect_type_name(inner, out);
        }
        ast::TypeKind::Set(inner) => {
            out.push_str("Set<");
            collect_type_name(inner, out);
        }
        ast::TypeKind::Map(k, v) => {
            out.push_str("Map<");
            collect_type_name(k, out);
            collect_type_name(v, out);
        }
        ast::TypeKind::Union(_) => {}
    }
}

fn generate_entity(out: &mut String, entity: &ast::EntityDecl, lang: &Language) {
    // Emit enum types for union-typed fields
    for field in &entity.fields {
        if let ast::TypeKind::Union(variants) = &field.ty.ty {
            let enum_name = format!("{}{}", entity.name, capitalize(&field.name));
            generate_union_enum(out, &enum_name, variants);
        }
    }

    // Javadoc
    if let Some(doc) = &entity.doc {
        out.push_str("    /**\n");
        for line in doc_text(doc).lines() {
            out.push_str(&format!("     * {line}\n"));
        }
        out.push_str("     */\n");
    }

    // Record declaration
    let params: Vec<String> = entity
        .fields
        .iter()
        .map(|f| {
            let ty = if let ast::TypeKind::Union(_) = &f.ty.ty {
                format!("{}{}", entity.name, capitalize(&f.name))
            } else {
                map_type(&f.ty, lang)
            };
            format!("{ty} {}", safe_ident(&f.name))
        })
        .collect();

    out.push_str(&format!("    public record {}(\n", entity.name));
    for (i, param) in params.iter().enumerate() {
        let comma = if i < params.len() - 1 { "," } else { "" };
        out.push_str(&format!("        {param}{comma}\n"));
    }
    out.push_str("    ) {}\n\n");
}

fn generate_union_enum(out: &mut String, name: &str, variants: &[ast::TypeKind]) {
    let names: Vec<&str> = variants
        .iter()
        .filter_map(|v| match v {
            ast::TypeKind::Simple(n) => Some(n.as_str()),
            _ => None,
        })
        .collect();

    out.push_str(&format!("    public enum {name} {{\n"));
    for (i, n) in names.iter().enumerate() {
        let comma = if i < names.len() - 1 { "," } else { "" };
        out.push_str(&format!("        {n}{comma}\n"));
    }
    out.push_str("    }\n\n");
}

fn generate_action(out: &mut String, action: &ast::ActionDecl, lang: &Language) {
    let fn_name = to_camel_case(&action.name);

    // Javadoc
    out.push_str("    /**\n");
    if let Some(doc) = &action.doc {
        for line in doc_text(doc).lines() {
            out.push_str(&format!("     * {line}\n"));
        }
    }

    // Requires
    if let Some(req) = &action.requires {
        out.push_str("     *\n     * <p>Requires:\n     * <ul>\n");
        for cond in &req.conditions {
            out.push_str(&format!(
                "     *   <li>{{@code {}}}</li>\n",
                format_expr(cond)
            ));
        }
        out.push_str("     * </ul>\n");
    }

    // Ensures
    if let Some(ens) = &action.ensures {
        out.push_str("     *\n     * <p>Ensures:\n     * <ul>\n");
        for item in &ens.items {
            out.push_str(&format!(
                "     *   <li>{{@code {}}}</li>\n",
                format_ensures_item(item)
            ));
        }
        out.push_str("     * </ul>\n");
    }

    // Properties
    if let Some(props) = &action.properties {
        out.push_str("     *\n     * <p>Properties:\n     * <ul>\n");
        for entry in &props.entries {
            out.push_str(&format!(
                "     *   <li>{}: {}</li>\n",
                entry.key,
                crate::format_prop_value(&entry.value)
            ));
        }
        out.push_str("     * </ul>\n");
    }

    out.push_str("     */\n");

    // Method signature
    let params: Vec<String> = action
        .params
        .iter()
        .map(|p| {
            let ty = map_type(&p.ty, lang);
            format!("{ty} {}", safe_ident(&p.name))
        })
        .collect();

    out.push_str(&format!(
        "    public static void {fn_name}({}) {{\n",
        params.join(", ")
    ));
    out.push_str(&format!(
        "        throw new UnsupportedOperationException(\"TODO: implement {fn_name}\");\n"
    ));
    out.push_str("    }\n\n");
}

fn generate_invariant(out: &mut String, inv: &ast::InvariantDecl) {
    out.push_str(&format!("    // Invariant: {}\n", inv.name));
    if let Some(doc) = &inv.doc {
        for line in doc_text(doc).lines() {
            out.push_str(&format!("    // {line}\n"));
        }
    }
    out.push_str(&format!("    // {}\n\n", format_expr(&inv.body)));
}

fn generate_edge_cases(out: &mut String, ec: &ast::EdgeCasesDecl) {
    out.push_str("    // Edge cases:\n");
    for rule in &ec.rules {
        out.push_str(&format!(
            "    // when {} => {}()\n",
            format_expr(&rule.condition),
            rule.action.name,
        ));
    }
    out.push('\n');
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
