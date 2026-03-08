//! Convert parse/check errors into LSP diagnostics.

use intent_check::CheckError;
use intent_parser::ParseError;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, Url,
};

use crate::document::Document;

/// Build LSP diagnostics from a document's parse/check errors.
pub fn compute_diagnostics(doc: &Document, uri: &Url) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if let Some(ref parse_error) = doc.parse_error {
        diagnostics.push(parse_error_to_diagnostic(parse_error, &doc.source, doc));
    }

    for error in &doc.check_errors {
        diagnostics.push(check_error_to_diagnostic(error, &doc.source, doc, uri));
    }

    diagnostics
}

fn parse_error_to_diagnostic(err: &ParseError, source: &str, doc: &Document) -> Diagnostic {
    let range = doc.line_index.source_span_to_range(err.span, source);

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(tower_lsp::lsp_types::NumberOrString::String(
            "intent::parse::syntax_error".to_string(),
        )),
        source: Some("intentlang".to_string()),
        message: err.message.clone(),
        ..Diagnostic::default()
    }
}

fn check_error_to_diagnostic(
    err: &CheckError,
    source: &str,
    doc: &Document,
    uri: &Url,
) -> Diagnostic {
    let (primary_span, severity, related) = extract_error_info(err, source, doc, uri);

    let range = doc.line_index.source_span_to_range(primary_span, source);

    Diagnostic {
        range,
        severity: Some(severity),
        code: Some(tower_lsp::lsp_types::NumberOrString::String(
            error_code(err).to_string(),
        )),
        source: Some("intentlang".to_string()),
        message: err.to_string(),
        related_information: if related.is_empty() {
            None
        } else {
            Some(related)
        },
        ..Diagnostic::default()
    }
}

/// Extract the primary span, severity, and any related information from a check error.
fn extract_error_info(
    err: &CheckError,
    source: &str,
    doc: &Document,
    uri: &Url,
) -> (
    miette::SourceSpan,
    DiagnosticSeverity,
    Vec<DiagnosticRelatedInformation>,
) {
    match err {
        // Duplicate errors: primary at redefinition, related at original.
        CheckError::DuplicateEntity { first, second, .. }
        | CheckError::DuplicateAction { first, second, .. }
        | CheckError::DuplicateInvariant { first, second, .. }
        | CheckError::DuplicateField { first, second, .. } => {
            let related = vec![DiagnosticRelatedInformation {
                location: Location {
                    uri: uri.clone(),
                    range: doc.line_index.source_span_to_range(*first, source),
                },
                message: "first defined here".to_string(),
            }];
            (*second, DiagnosticSeverity::ERROR, related)
        }

        // Single-span errors.
        CheckError::UndefinedType { span, .. }
        | CheckError::UndefinedQuantifierType { span, .. }
        | CheckError::UndefinedEdgeAction { span, .. }
        | CheckError::UnknownField { span, .. }
        | CheckError::UnresolvedImport { span, .. }
        | CheckError::OldInRequires { span, .. } => (*span, DiagnosticSeverity::ERROR, Vec::new()),

        // Tautological comparison is a warning.
        CheckError::TautologicalComparison { span, .. } => {
            (*span, DiagnosticSeverity::WARNING, Vec::new())
        }
    }
}

pub fn error_code(err: &CheckError) -> &'static str {
    match err {
        CheckError::DuplicateEntity { .. } => "intent::check::duplicate_entity",
        CheckError::DuplicateAction { .. } => "intent::check::duplicate_action",
        CheckError::DuplicateInvariant { .. } => "intent::check::duplicate_invariant",
        CheckError::DuplicateField { .. } => "intent::check::duplicate_field",
        CheckError::UndefinedType { .. } => "intent::check::undefined_type",
        CheckError::UndefinedQuantifierType { .. } => "intent::check::undefined_quantifier_type",
        CheckError::UndefinedEdgeAction { .. } => "intent::check::undefined_edge_action",
        CheckError::UnknownField { .. } => "intent::check::unknown_field",
        CheckError::UnresolvedImport { .. } => "intent::check::unresolved_import",
        CheckError::OldInRequires { .. } => "intent::check::old_in_requires",
        CheckError::TautologicalComparison { .. } => "intent::check::tautological_comparison",
    }
}
