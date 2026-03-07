//! Core IR type definitions for IntentLang.
//!
//! The IR is a typed, formally verifiable representation generated from
//! intent specs. Every IR node carries a `SourceTrace` linking it back
//! to the originating spec element (for Phase 3 Audit Bridge).

use intent_parser::ast::Span;
use serde::{Deserialize, Serialize};

// ── Source tracing ──────────────────────────────────────────

/// Links an IR node back to its originating spec element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceTrace {
    /// Module name from the intent file.
    pub module: String,
    /// Which spec item produced this IR node (e.g., "Transfer", "NoNegativeBalances").
    pub item: String,
    /// Which part of the spec item (e.g., "requires", "ensures", "field:balance").
    pub part: String,
    /// Byte span in the original source.
    pub span: Span,
}

// ── Module ──────────────────────────────────────────────────

/// A compiled IR module — the output of lowering a single `.intent` file.
#[derive(Debug, Clone, Serialize)]
pub struct Module {
    pub name: String,
    pub structs: Vec<Struct>,
    pub functions: Vec<Function>,
    pub invariants: Vec<Invariant>,
    pub edge_guards: Vec<EdgeGuard>,
}

// ── Types ───────────────────────────────────────────────────

/// An IR type — resolved from the intent language's type expressions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum IrType {
    /// Named primitive or domain type (UUID, String, Int, Bool, etc.)
    Named(String),
    /// Reference to a struct defined in this module.
    Struct(String),
    /// `List<T>`
    List(Box<IrType>),
    /// `Set<T>`
    Set(Box<IrType>),
    /// `Map<K, V>`
    Map(Box<IrType>, Box<IrType>),
    /// Optional wrapper: `T?`
    Optional(Box<IrType>),
    /// Union of variants (enum-like): `Active | Frozen | Closed`
    Union(Vec<String>),
    /// Decimal with precision: `Decimal(2)`
    Decimal(u32),
}

// ── Struct (from entity) ────────────────────────────────────

/// A struct type — lowered from an `entity` declaration.
#[derive(Debug, Clone, Serialize)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<Field>,
    pub trace: SourceTrace,
}

/// A typed field within a struct.
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub name: String,
    pub ty: IrType,
    pub trace: SourceTrace,
}

// ── Function (from action) ──────────────────────────────────

/// A function — lowered from an `action` declaration.
/// Contains typed parameters, pre/postconditions, and effect/property annotations.
#[derive(Debug, Clone, Serialize)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub preconditions: Vec<Condition>,
    pub postconditions: Vec<Postcondition>,
    pub properties: Vec<Property>,
    pub trace: SourceTrace,
}

/// A typed function parameter.
#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: String,
    pub ty: IrType,
    pub trace: SourceTrace,
}

/// A precondition (from `requires`).
#[derive(Debug, Clone, Serialize)]
pub struct Condition {
    pub expr: IrExpr,
    pub trace: SourceTrace,
}

/// A postcondition (from `ensures`) — may be unconditional or guarded by `when`.
#[derive(Debug, Clone, Serialize)]
pub enum Postcondition {
    /// Always holds after execution.
    Always { expr: IrExpr, trace: SourceTrace },
    /// Holds only when `guard` is true.
    When {
        guard: IrExpr,
        expr: IrExpr,
        trace: SourceTrace,
    },
}

/// A declarative property annotation (idempotent, atomic, etc.)
#[derive(Debug, Clone, Serialize)]
pub struct Property {
    pub key: String,
    pub value: PropertyValue,
    pub trace: SourceTrace,
}

/// Property value — mirrors the AST's PropValue but in IR form.
#[derive(Debug, Clone, Serialize)]
pub enum PropertyValue {
    Bool(bool),
    Int(i64),
    String(String),
    Ident(String),
}

// ── Invariant ───────────────────────────────────────────────

/// A module-level invariant — a proof obligation that must always hold.
#[derive(Debug, Clone, Serialize)]
pub struct Invariant {
    pub name: String,
    pub expr: IrExpr,
    pub trace: SourceTrace,
}

// ── Edge guards ─────────────────────────────────────────────

/// An edge-case guard — lowered from `edge_cases { when cond => action }`.
#[derive(Debug, Clone, Serialize)]
pub struct EdgeGuard {
    pub condition: IrExpr,
    pub action: String,
    pub args: Vec<(String, IrExpr)>,
    pub trace: SourceTrace,
}

// ── IR Expressions ──────────────────────────────────────────

/// An IR expression — a simplified, typed expression tree.
#[derive(Debug, Clone, Serialize)]
pub enum IrExpr {
    /// Variable reference (parameter name or quantifier binding).
    Var(String),
    /// Literal value.
    Literal(IrLiteral),
    /// Field access: `obj.field`
    FieldAccess { root: Box<IrExpr>, field: String },
    /// Binary comparison.
    Compare {
        left: Box<IrExpr>,
        op: CmpOp,
        right: Box<IrExpr>,
    },
    /// Arithmetic operation.
    Arithmetic {
        left: Box<IrExpr>,
        op: ArithOp,
        right: Box<IrExpr>,
    },
    /// Logical AND.
    And(Box<IrExpr>, Box<IrExpr>),
    /// Logical OR.
    Or(Box<IrExpr>, Box<IrExpr>),
    /// Logical NOT.
    Not(Box<IrExpr>),
    /// Logical implication: `a => b` ≡ `!a || b`.
    Implies(Box<IrExpr>, Box<IrExpr>),
    /// Pre-state reference: `old(expr)`.
    Old(Box<IrExpr>),
    /// Universal quantifier: `forall x: T => body`.
    Forall {
        binding: String,
        ty: String,
        body: Box<IrExpr>,
    },
    /// Existential quantifier: `exists x: T => body`.
    Exists {
        binding: String,
        ty: String,
        body: Box<IrExpr>,
    },
    /// Function call.
    Call { name: String, args: Vec<IrExpr> },
}

/// Comparison operators (shared with AST but owned by IR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

/// Arithmetic operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ArithOp {
    Add,
    Sub,
}

/// IR literal values.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum IrLiteral {
    Null,
    Bool(bool),
    Int(i64),
    Decimal(String),
    String(String),
}
