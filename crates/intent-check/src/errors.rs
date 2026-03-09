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
    #[diagnostic(
        code(intent::check::duplicate_entity),
        help("rename one of the entities")
    )]
    DuplicateEntity {
        name: String,
        #[label("first defined here")]
        first: SourceSpan,
        #[label("redefined here")]
        second: SourceSpan,
    },

    #[error("duplicate action `{name}`")]
    #[diagnostic(
        code(intent::check::duplicate_action),
        help("rename one of the actions")
    )]
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
    #[diagnostic(code(intent::check::undefined_type), help("{help_text}"))]
    UndefinedType {
        name: String,
        #[label("used here")]
        span: SourceSpan,
        /// Dynamic help text — includes "did you mean?" suggestion when available.
        help_text: String,
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
    #[diagnostic(code(intent::check::unknown_field), help("{help_text}"))]
    UnknownField {
        field: String,
        entity: String,
        #[label("accessed here")]
        span: SourceSpan,
        /// Dynamic help text — includes "did you mean?" suggestion when available.
        help_text: String,
    },

    #[error("unresolved import `{name}` — item not found in module `{module}`")]
    #[diagnostic(
        code(intent::check::unresolved_import),
        help("check that `{name}` is defined in module `{module}`")
    )]
    UnresolvedImport {
        name: String,
        module: String,
        #[label("imported here")]
        span: SourceSpan,
    },

    #[error("`old()` cannot be used in a `requires` block")]
    #[diagnostic(
        code(intent::check::old_in_requires),
        help(
            "in requires blocks, values are pre-state by default — `old()` is only needed in ensures blocks to reference pre-state values"
        )
    )]
    OldInRequires {
        #[label("used here")]
        span: SourceSpan,
    },

    #[error("comparing `{expr}` to itself is always {result}")]
    #[diagnostic(code(intent::check::tautological_comparison), help("{help_text}"))]
    TautologicalComparison {
        expr: String,
        result: String,
        #[label("both sides are identical")]
        span: SourceSpan,
        /// Dynamic help text with suggestions for alternative expressions.
        help_text: String,
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

    pub fn undefined_type(name: &str, span: Span, suggestion: Option<&str>) -> Self {
        let help_text = match suggestion {
            Some(s) => format!("did you mean `{s}`?"),
            None => format!("define an entity named `{name}`, or use a built-in type"),
        };
        Self::UndefinedType {
            name: name.to_string(),
            span: to_source_span(span),
            help_text,
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

    pub fn unknown_field(field: &str, entity: &str, span: Span, suggestion: Option<&str>) -> Self {
        let help_text = match suggestion {
            Some(s) => format!("did you mean `{s}`?"),
            None => format!("`{entity}` has no field named `{field}`"),
        };
        Self::UnknownField {
            field: field.to_string(),
            entity: entity.to_string(),
            span: to_source_span(span),
            help_text,
        }
    }

    pub fn unresolved_import(name: &str, module: &str, span: Span) -> Self {
        Self::UnresolvedImport {
            name: name.to_string(),
            module: module.to_string(),
            span: to_source_span(span),
        }
    }

    pub fn old_in_requires(span: Span) -> Self {
        Self::OldInRequires {
            span: to_source_span(span),
        }
    }

    pub fn tautological_comparison(expr: &str, result: &str, span: Span) -> Self {
        let help_text = build_tautological_help(expr);
        Self::TautologicalComparison {
            expr: expr.to_string(),
            result: result.to_string(),
            span: to_source_span(span),
            help_text,
        }
    }
}

/// Build help text for a tautological comparison, suggesting alternatives.
///
/// For `a.balance`, suggests `old(a.balance)` and notes that both sides are identical.
/// For `x`, suggests `old(x)`.
fn build_tautological_help(expr: &str) -> String {
    if expr.contains('.') {
        // Field access: suggest old() variant
        format!(
            "both sides of this comparison are identical — did you mean `{expr} == old({expr})`?"
        )
    } else {
        "both sides of this comparison are identical — did you mean to compare different values?"
            .to_string()
    }
}
