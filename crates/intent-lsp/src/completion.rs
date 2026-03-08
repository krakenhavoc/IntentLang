//! Completion provider: keywords, types, and entity/action names.

use intent_parser::ast::{self, TopLevelItem};
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, InsertTextFormat,
};

use crate::document::Document;

/// Built-in types available for completion.
const BUILTIN_TYPES: &[&str] = &[
    "UUID",
    "String",
    "Int",
    "Decimal",
    "Bool",
    "DateTime",
    "CurrencyCode",
    "Email",
    "URL",
];

/// Provide completion items for the given byte offset.
pub fn completions(doc: &Document, offset: usize) -> Option<CompletionResponse> {
    let mut items = Vec::new();

    let context = determine_context(&doc.source, offset);

    match context {
        Context::TopLevel => {
            add_top_level_keywords(&mut items);
        }
        Context::InsideBlock => {
            add_block_keywords(&mut items);
        }
        Context::TypePosition => {
            add_type_completions(&mut items, doc);
        }
        Context::Expression => {
            add_expression_completions(&mut items, doc, offset);
        }
        Context::Unknown => {
            // Provide everything as a fallback.
            add_top_level_keywords(&mut items);
            add_type_completions(&mut items, doc);
        }
    }

    if items.is_empty() {
        None
    } else {
        Some(CompletionResponse::Array(items))
    }
}

/// Broad context categories for completion.
enum Context {
    TopLevel,
    InsideBlock,
    TypePosition,
    Expression,
    Unknown,
}

/// Determine the cursor context from the source text.
fn determine_context(source: &str, offset: usize) -> Context {
    let before = &source[..offset.min(source.len())];

    // Count open/close braces to determine nesting.
    let open_braces = before.chars().filter(|&c| c == '{').count();
    let close_braces = before.chars().filter(|&c| c == '}').count();
    let nesting = open_braces.saturating_sub(close_braces);

    if nesting == 0 {
        return Context::TopLevel;
    }

    // Look at the last non-whitespace character before offset.
    let trimmed = before.trim_end();

    // After a colon → type position.
    if trimmed.ends_with(':') {
        return Context::TypePosition;
    }

    // Inside requires/ensures → expression.
    let last_keyword = find_last_keyword(before);
    match last_keyword.as_deref() {
        Some("requires" | "ensures") if nesting >= 2 => Context::Expression,
        _ if nesting == 1 => Context::InsideBlock,
        _ if nesting >= 2 => Context::Expression,
        _ => Context::Unknown,
    }
}

/// Find the last IntentLang keyword before the cursor.
fn find_last_keyword(text: &str) -> Option<String> {
    let keywords = [
        "requires",
        "ensures",
        "properties",
        "entity",
        "action",
        "invariant",
        "edge_cases",
    ];

    let mut best: Option<(usize, &str)> = None;
    for kw in keywords {
        if let Some(pos) = text.rfind(kw)
            && (best.is_none() || pos > best.unwrap().0)
        {
            best = Some((pos, kw));
        }
    }

    best.map(|(_, kw)| kw.to_string())
}

fn add_top_level_keywords(items: &mut Vec<CompletionItem>) {
    let keywords = [
        (
            "entity",
            "entity ${1:Name} {\n  ${2:field}: ${3:Type}\n}",
            "Define an entity",
        ),
        (
            "action",
            "action ${1:Name} {\n  ${2:param}: ${3:Type}\n\n  requires {\n    ${4}\n  }\n\n  ensures {\n    ${5}\n  }\n}",
            "Define an action",
        ),
        (
            "invariant",
            "invariant ${1:Name} {\n  forall ${2:x}: ${3:Type} => ${4:predicate}\n}",
            "Define an invariant",
        ),
        (
            "edge_cases",
            "edge_cases {\n  when ${1:condition} => ${2:Action}()\n}",
            "Define edge cases",
        ),
        ("use", "use ${1:ModuleName}", "Import a module"),
    ];

    for (label, snippet, detail) in keywords {
        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..CompletionItem::default()
        });
    }
}

fn add_block_keywords(items: &mut Vec<CompletionItem>) {
    let keywords = [
        ("requires", "requires {\n  ${1}\n}", "Precondition block"),
        ("ensures", "ensures {\n  ${1}\n}", "Postcondition block"),
        (
            "properties",
            "properties {\n  ${1:key}: ${2:value}\n}",
            "Properties block",
        ),
    ];

    for (label, snippet, detail) in keywords {
        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..CompletionItem::default()
        });
    }
}

fn add_type_completions(items: &mut Vec<CompletionItem>, doc: &Document) {
    // Built-in types.
    for &ty in BUILTIN_TYPES {
        items.push(CompletionItem {
            label: ty.to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some("Built-in type".to_string()),
            ..CompletionItem::default()
        });
    }

    // Collection types.
    let collections = [
        ("List", "List<${1:Type}>", "Ordered collection"),
        ("Set", "Set<${1:Type}>", "Unique collection"),
        (
            "Map",
            "Map<${1:KeyType}, ${2:ValueType}>",
            "Key-value mapping",
        ),
    ];
    for (label, snippet, detail) in collections {
        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some(detail.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..CompletionItem::default()
        });
    }

    // User-defined entity names as types.
    if let Some(ref ast) = doc.ast {
        for item in &ast.items {
            if let TopLevelItem::Entity(e) = item {
                items.push(CompletionItem {
                    label: e.name.clone(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some("Entity".to_string()),
                    ..CompletionItem::default()
                });
            }
        }
    }
}

fn add_expression_completions(items: &mut Vec<CompletionItem>, doc: &Document, offset: usize) {
    // Expression keywords.
    let expr_keywords = [
        (
            "forall",
            "forall ${1:x}: ${2:Type} => ${3:predicate}",
            "Universal quantifier",
        ),
        (
            "exists",
            "exists ${1:x}: ${2:Type} => ${3:predicate}",
            "Existential quantifier",
        ),
        ("old", "old(${1:expr})", "Pre-state reference"),
        ("true", "true", "Boolean true"),
        ("false", "false", "Boolean false"),
        ("null", "null", "Null value"),
    ];

    for (label, snippet, detail) in expr_keywords {
        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..CompletionItem::default()
        });
    }

    // Add parameter names if we're inside an action.
    if let Some(ref ast) = doc.ast {
        if let Some(action) = find_enclosing_action(ast, offset) {
            for param in &action.params {
                items.push(CompletionItem {
                    label: param.name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(super::hover::format_type(&param.ty)),
                    ..CompletionItem::default()
                });
            }
        }

        // Add entity and action names for references.
        for item in &ast.items {
            match item {
                TopLevelItem::Entity(e) => {
                    items.push(CompletionItem {
                        label: e.name.clone(),
                        kind: Some(CompletionItemKind::CLASS),
                        detail: Some("Entity".to_string()),
                        ..CompletionItem::default()
                    });
                }
                TopLevelItem::Action(a) => {
                    items.push(CompletionItem {
                        label: a.name.clone(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some("Action".to_string()),
                        ..CompletionItem::default()
                    });
                }
                _ => {}
            }
        }
    }
}

/// Find the action declaration that contains the given byte offset.
fn find_enclosing_action(ast: &ast::File, offset: usize) -> Option<&ast::ActionDecl> {
    for item in &ast.items {
        if let TopLevelItem::Action(a) = item
            && offset >= a.span.start
            && offset < a.span.end
        {
            return Some(a);
        }
    }
    None
}
