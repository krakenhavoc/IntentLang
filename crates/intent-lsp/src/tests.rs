//! Integration tests for LSP features: diagnostics, hover, navigation, completion.

use tower_lsp::lsp_types::*;

use crate::completion;
use crate::diagnostics;
use crate::document::Document;
use crate::hover;
use crate::navigation;

fn test_uri() -> Url {
    Url::parse("file:///test.intent").unwrap()
}

fn make_doc(source: &str) -> Document {
    Document::new(source.to_string(), None)
}

// ── Diagnostics ────────────────────────────────────────────

#[test]
fn diagnostics_clean_file() {
    let doc = make_doc("module Test\n\nentity Foo {\n  id: UUID\n}\n");
    let diags = diagnostics::compute_diagnostics(&doc, &test_uri());
    assert!(diags.is_empty(), "clean file should have no diagnostics");
}

#[test]
fn diagnostics_parse_error() {
    let doc = make_doc("this is not valid intent");
    let diags = diagnostics::compute_diagnostics(&doc, &test_uri());
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, Some(DiagnosticSeverity::ERROR));
    assert!(diags[0].source.as_deref() == Some("intentlang"));
}

#[test]
fn diagnostics_undefined_type() {
    let doc = make_doc("module Test\n\nentity Foo {\n  bar: NonExistent\n}\n");
    let diags = diagnostics::compute_diagnostics(&doc, &test_uri());
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("NonExistent"));
    assert_eq!(diags[0].severity, Some(DiagnosticSeverity::ERROR));
}

#[test]
fn diagnostics_duplicate_entity() {
    let doc =
        make_doc("module Test\n\nentity Foo {\n  id: UUID\n}\n\nentity Foo {\n  name: String\n}\n");
    let diags = diagnostics::compute_diagnostics(&doc, &test_uri());
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("duplicate"));
    // Should have related info pointing to the first definition.
    assert!(diags[0].related_information.is_some());
    assert_eq!(diags[0].related_information.as_ref().unwrap().len(), 1);
}

#[test]
fn diagnostics_tautological_is_warning() {
    let doc = make_doc(
        "module Test\n\nentity Foo {\n  x: Int\n}\n\naction Bar {\n  f: Foo\n\n  requires {\n    f.x == f.x\n  }\n}\n",
    );
    let diags = diagnostics::compute_diagnostics(&doc, &test_uri());
    let warnings: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::WARNING))
        .collect();
    assert!(
        !warnings.is_empty(),
        "tautological comparison should be a warning"
    );
}

// ── Hover ──────────────────────────────────────────────────

#[test]
fn hover_keyword_entity() {
    let src = "module Test\n\nentity Foo {\n  id: UUID\n}\n";
    let doc = make_doc(src);
    let offset = src.find("entity").unwrap();
    let result = hover::hover_at(&doc, offset);
    assert!(result.is_some());
    let hover = result.unwrap();
    if let HoverContents::Markup(markup) = hover.contents {
        assert!(markup.value.contains("entity"));
    } else {
        panic!("expected markup hover");
    }
}

#[test]
fn hover_entity_name() {
    let src = "module Test\n\nentity Account {\n  id: UUID\n  balance: Int\n}\n";
    let doc = make_doc(src);
    let offset = src.find("Account").unwrap();
    let result = hover::hover_at(&doc, offset);
    assert!(result.is_some());
    let hover = result.unwrap();
    if let HoverContents::Markup(markup) = hover.contents {
        assert!(markup.value.contains("Account"));
        assert!(markup.value.contains("id"));
        assert!(markup.value.contains("balance"));
    } else {
        panic!("expected markup hover");
    }
}

#[test]
fn hover_builtin_type() {
    let src = "module Test\n\nentity Foo {\n  id: UUID\n}\n";
    let doc = make_doc(src);
    let offset = src.find("UUID").unwrap();
    let result = hover::hover_at(&doc, offset);
    assert!(result.is_some());
    if let HoverContents::Markup(markup) = result.unwrap().contents {
        assert!(markup.value.contains("Built-in type"));
    }
}

#[test]
fn hover_no_result_on_whitespace() {
    let src = "module Test\n\n\nentity Foo {\n  id: UUID\n}\n";
    let doc = make_doc(src);
    // Offset in the blank line between module and entity.
    let offset = src.find("\n\n").unwrap() + 1;
    let result = hover::hover_at(&doc, offset);
    assert!(result.is_none());
}

// ── Navigation (go-to-definition) ──────────────────────────

#[test]
fn goto_def_entity_type_ref() {
    let src =
        "module Test\n\nentity Account {\n  id: UUID\n}\n\naction Transfer {\n  from: Account\n}\n";
    let doc = make_doc(src);
    let uri = test_uri();

    // Find "Account" in the action's parameter type (second occurrence).
    let first_account = src.find("Account").unwrap();
    let second_account = src[first_account + 1..].find("Account").unwrap() + first_account + 1;
    let result = navigation::goto_definition(&doc, second_account, &uri);

    assert!(result.is_some(), "should find definition");
    if let Some(GotoDefinitionResponse::Scalar(loc)) = result {
        // Should point to the entity declaration.
        assert_eq!(loc.uri, uri);
        assert_eq!(loc.range.start.line, 2); // line of `entity Account {`
    }
}

#[test]
fn goto_def_no_result_for_builtin() {
    let src = "module Test\n\nentity Foo {\n  id: UUID\n}\n";
    let doc = make_doc(src);
    let uri = test_uri();

    // UUID is a built-in type — no user definition to jump to.
    let offset = src.find("UUID").unwrap();
    let result = navigation::goto_definition(&doc, offset, &uri);
    assert!(result.is_none());
}

#[test]
fn goto_def_action_in_edge_case() {
    let src = "module Test\n\nentity Foo {\n  id: UUID\n}\n\naction HandleError {\n  f: Foo\n}\n\nedge_cases {\n  when true => HandleError(f: Foo)\n}\n";
    let doc = make_doc(src);
    let uri = test_uri();

    // Find HandleError in the edge_cases block.
    let first = src.find("HandleError").unwrap();
    let second = src[first + 1..].find("HandleError").unwrap() + first + 1;
    let result = navigation::goto_definition(&doc, second, &uri);

    assert!(result.is_some(), "should find action definition");
}

// ── Completion ─────────────────────────────────────────────

#[test]
fn completion_top_level() {
    let src = "module Test\n\n";
    let doc = make_doc(src);
    let offset = src.len();
    let result = completion::completions(&doc, offset);

    assert!(result.is_some());
    if let Some(CompletionResponse::Array(items)) = result {
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(labels.contains(&"entity".to_string()));
        assert!(labels.contains(&"action".to_string()));
        assert!(labels.contains(&"invariant".to_string()));
        assert!(labels.contains(&"use".to_string()));
    }
}

#[test]
fn completion_type_position() {
    let src = "module Test\n\nentity Foo {\n  id: ";
    let doc = make_doc(src);
    // After `: ` — should suggest types.
    let offset = src.len();
    let result = completion::completions(&doc, offset);

    assert!(result.is_some());
    if let Some(CompletionResponse::Array(items)) = result {
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(labels.contains(&"UUID".to_string()));
        assert!(labels.contains(&"String".to_string()));
        assert!(labels.contains(&"Int".to_string()));
        assert!(labels.contains(&"List".to_string()));
    }
}

#[test]
fn completion_inside_action_suggests_params() {
    // Use valid syntax so the parser produces an AST for completion context.
    let src = "module Test\n\nentity Foo {\n  id: UUID\n}\n\naction Bar {\n  myParam: Foo\n\n  requires {\n    myParam.id != null\n  }\n}\n";
    let doc = make_doc(src);
    // Place cursor inside the requires block.
    let offset = src.find("myParam.id").unwrap();
    let result = completion::completions(&doc, offset);

    assert!(result.is_some());
    if let Some(CompletionResponse::Array(items)) = result {
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(
            labels.contains(&"myParam".to_string()),
            "should suggest action parameter names, got: {:?}",
            labels
        );
    }
}
