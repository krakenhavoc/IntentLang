//! Diagnostic error types for semantic analysis.

use intent_parser::ast::Span;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Convert our AST Span to miette's SourceSpan.
fn to_source_span(span: Span) -> SourceSpan {
    (span.start, span.end - span.start).into()
}

/// A semantic check diagnostic.
#[derive(Debug, Clone, Error, Diagnostic)]
pub enum CheckError {
    #[error("duplicate entity `{name}`")]
    #[diagnostic(code(intent::check::duplicate_entity), help("rename one of the entities"))]
    DuplicateEntity {
        name: String,
        #[label("first defined here")]
        first: SourceSpan,
        #[label("redefined here")]
        second: SourceSpan,
    },

    #[error("duplicate action `{name}`")]
    #[diagnostic(code(intent::check::duplicate_action), help("rename one of the actions"))]
    DuplicateAction {
        name: String,
        #[label("first defined here")]
        first: SourceSpan,
        #[label("redefined here")]
        second: SourceSpan,
    },

    #[error("duplicate invariant `{name}`")]
    #[diagnostic(
        code(intent::check::duplicate_invariant),
        help("rename one of the invariants")
    )]
    DuplicateInvariant {
        name: String,
        #[label("first defined here")]
        first: SourceSpan,
        #[label("redefined here")]
        second: SourceSpan,
    },

    #[error("duplicate field `{field}` in `{parent}`")]
    #[diagnostic(
        code(intent::check::duplicate_field),
        help("remove or rename the duplicate field")
    )]
    DuplicateField {
        field: String,
        parent: String,
        #[label("first defined here")]
        first: SourceSpan,
        #[label("redefined here")]
        second: SourceSpan,
    },

    #[error("undefined type `{name}`")]
    #[diagnostic(
        code(intent::check::undefined_type),
        help("define an entity named `{name}`, or use a built-in type")
    )]
    UndefinedType {
        name: String,
        #[label("used here")]
        span: SourceSpan,
    },

    #[error("undefined entity `{name}` in quantifier binding")]
    #[diagnostic(
        code(intent::check::undefined_quantifier_type),
        help("the type in `forall`/`exists` bindings must be a defined entity or action")
    )]
    UndefinedQuantifierType {
        name: String,
        #[label("used here")]
        span: SourceSpan,
    },

    #[error("undefined action `{name}` in edge case")]
    #[diagnostic(
        code(intent::check::undefined_edge_action),
        help("define an action named `{name}`, or use the name of an existing action")
    )]
    UndefinedEdgeAction {
        name: String,
        #[label("called here")]
        span: SourceSpan,
    },

    #[error("unknown field `{field}` on entity `{entity}`")]
    #[diagnostic(
        code(intent::check::unknown_field),
        help("`{entity}` has no field named `{field}`")
    )]
    UnknownField {
        field: String,
        entity: String,
        #[label("accessed here")]
        span: SourceSpan,
    },
}

impl CheckError {
    pub fn duplicate_entity(name: &str, first: Span, second: Span) -> Self {
        Self::DuplicateEntity {
            name: name.to_string(),
            first: to_source_span(first),
            second: to_source_span(second),
        }
    }

    pub fn duplicate_action(name: &str, first: Span, second: Span) -> Self {
        Self::DuplicateAction {
            name: name.to_string(),
            first: to_source_span(first),
            second: to_source_span(second),
        }
    }

    pub fn duplicate_invariant(name: &str, first: Span, second: Span) -> Self {
        Self::DuplicateInvariant {
            name: name.to_string(),
            first: to_source_span(first),
            second: to_source_span(second),
        }
    }

    pub fn duplicate_field(field: &str, parent: &str, first: Span, second: Span) -> Self {
        Self::DuplicateField {
            field: field.to_string(),
            parent: parent.to_string(),
            first: to_source_span(first),
            second: to_source_span(second),
        }
    }

    pub fn undefined_type(name: &str, span: Span) -> Self {
        Self::UndefinedType {
            name: name.to_string(),
            span: to_source_span(span),
        }
    }

    pub fn undefined_quantifier_type(name: &str, span: Span) -> Self {
        Self::UndefinedQuantifierType {
            name: name.to_string(),
            span: to_source_span(span),
        }
    }

    pub fn undefined_edge_action(name: &str, span: Span) -> Self {
        Self::UndefinedEdgeAction {
            name: name.to_string(),
            span: to_source_span(span),
        }
    }

    pub fn unknown_field(field: &str, entity: &str, span: Span) -> Self {
        Self::UnknownField {
            field: field.to_string(),
            entity: entity.to_string(),
            span: to_source_span(span),
        }
    }
}
