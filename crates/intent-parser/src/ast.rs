//! Typed AST for the IntentLang specification language.
//!
//! Every node carries a `Span` with byte offsets into the source text,
//! supporting Phase 3 (Audit Bridge) source-location tracing.

use serde::{Deserialize, Serialize};

/// Byte-offset span in the source text: `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

// ── Top-level ────────────────────────────────────────────────

/// A complete `.intent` file.
#[derive(Debug, Clone, Serialize)]
pub struct File {
    pub module: ModuleDecl,
    pub doc: Option<DocBlock>,
    pub imports: Vec<UseDecl>,
    pub items: Vec<TopLevelItem>,
    pub span: Span,
}

/// `use ModuleName` or `use ModuleName.ItemName`
#[derive(Debug, Clone, Serialize)]
pub struct UseDecl {
    pub module_name: String,
    pub item: Option<String>,
    pub span: Span,
}

/// `module ModuleName`
#[derive(Debug, Clone, Serialize)]
pub struct ModuleDecl {
    pub name: String,
    pub span: Span,
}

/// A sequence of `---` documentation lines.
#[derive(Debug, Clone, Serialize)]
pub struct DocBlock {
    pub lines: Vec<String>,
    pub span: Span,
}

/// Any top-level declaration.
#[derive(Debug, Clone, Serialize)]
pub enum TopLevelItem {
    Entity(EntityDecl),
    Action(ActionDecl),
    Invariant(InvariantDecl),
    EdgeCases(EdgeCasesDecl),
    Test(TestDecl),
    StateMachine(StateMachineDecl),
}

// ── State Machine ─────────────────────────────────────────────

/// `state StateName { chain* }` — State machine with named variants and transitions.
///
/// Syntactic sugar that auto-generates a union type with valid transitions.
/// `chains` preserves the original declaration for formatting round-trips.
#[derive(Debug, Clone, Serialize)]
pub struct StateMachineDecl {
    pub doc: Option<DocBlock>,
    pub name: String,
    /// All unique state variants (in first-seen order).
    pub states: Vec<String>,
    /// Valid transitions: `(from_state, to_state)`.
    pub transitions: Vec<(String, String)>,
    /// Original transition chains for formatter round-tripping.
    pub chains: Vec<Vec<String>>,
    pub span: Span,
}

// ── Test ────────────────────────────────────────────────────

/// `test "name" { given { ... } when Action { ... } then { ... } }`
#[derive(Debug, Clone, Serialize)]
pub struct TestDecl {
    pub name: String,
    pub given: Vec<GivenBinding>,
    pub when_action: WhenAction,
    pub then: ThenClause,
    pub span: Span,
}

/// `name = TypeName { ... }` or `name = expr` — a concrete binding in a test.
#[derive(Debug, Clone, Serialize)]
pub struct GivenBinding {
    pub name: String,
    pub value: GivenValue,
    pub span: Span,
}

/// The right-hand side of a given binding.
#[derive(Debug, Clone, Serialize)]
pub enum GivenValue {
    /// `TypeName { field: value, ... }` — an entity instance.
    EntityConstructor {
        type_name: String,
        fields: Vec<ConstructorField>,
    },
    /// A plain expression (number, string, identifier, etc.).
    Expr(Expr),
}

/// `name: expr` inside an entity constructor or when block.
#[derive(Debug, Clone, Serialize)]
pub struct ConstructorField {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

/// `when ActionName { param: value, ... }` — the action invocation in a test.
#[derive(Debug, Clone, Serialize)]
pub struct WhenAction {
    pub action_name: String,
    pub args: Vec<ConstructorField>,
    pub span: Span,
}

/// The expected outcome of a test.
#[derive(Debug, Clone, Serialize)]
pub enum ThenClause {
    /// `then { expr* }` — assertions to check against new state.
    Asserts(Vec<Expr>, Span),
    /// `then fails` with optional violation kind filter.
    Fails(Option<String>, Span),
}

// ── Entity ───────────────────────────────────────────────────

/// `entity EntityName { field* }`
#[derive(Debug, Clone, Serialize)]
pub struct EntityDecl {
    pub doc: Option<DocBlock>,
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

/// `name: Type` — a field or parameter declaration.
#[derive(Debug, Clone, Serialize)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

// ── Action ───────────────────────────────────────────────────

/// `action ActionName { ... }`
#[derive(Debug, Clone, Serialize)]
pub struct ActionDecl {
    pub doc: Option<DocBlock>,
    pub name: String,
    pub params: Vec<FieldDecl>,
    pub requires: Option<RequiresBlock>,
    pub ensures: Option<EnsuresBlock>,
    pub properties: Option<PropertiesBlock>,
    pub span: Span,
}

/// `requires { expr* }`
#[derive(Debug, Clone, Serialize)]
pub struct RequiresBlock {
    pub conditions: Vec<Expr>,
    pub span: Span,
}

/// `ensures { item* }`
#[derive(Debug, Clone, Serialize)]
pub struct EnsuresBlock {
    pub items: Vec<EnsuresItem>,
    pub span: Span,
}

/// A postcondition — either a bare expression or a `when` clause.
#[derive(Debug, Clone, Serialize)]
pub enum EnsuresItem {
    Expr(Expr),
    When(WhenClause),
}

/// `when condition => consequence`
#[derive(Debug, Clone, Serialize)]
pub struct WhenClause {
    pub condition: Expr,
    pub consequence: Expr,
    pub span: Span,
}

/// `properties { entry* }`
#[derive(Debug, Clone, Serialize)]
pub struct PropertiesBlock {
    pub entries: Vec<PropEntry>,
    pub span: Span,
}

/// `key: value` inside a properties block.
#[derive(Debug, Clone, Serialize)]
pub struct PropEntry {
    pub key: String,
    pub value: PropValue,
    pub span: Span,
}

/// The right-hand side of a property entry.
#[derive(Debug, Clone, Serialize)]
pub enum PropValue {
    Literal(Literal),
    Ident(String),
    List(Vec<PropValue>),
    Object(Vec<(String, PropValue)>),
}

// ── Invariant ────────────────────────────────────────────────

/// `invariant InvariantName { doc? expr }`
#[derive(Debug, Clone, Serialize)]
pub struct InvariantDecl {
    pub doc: Option<DocBlock>,
    pub name: String,
    pub body: Expr,
    pub span: Span,
}

// ── Edge cases ───────────────────────────────────────────────

/// `edge_cases { rule* }`
#[derive(Debug, Clone, Serialize)]
pub struct EdgeCasesDecl {
    pub rules: Vec<EdgeRule>,
    pub span: Span,
}

/// `when condition => action(args)`
#[derive(Debug, Clone, Serialize)]
pub struct EdgeRule {
    pub condition: Expr,
    pub action: ActionCall,
    pub span: Span,
}

/// A function-call-style action on the RHS of an edge rule.
#[derive(Debug, Clone, Serialize)]
pub struct ActionCall {
    pub name: String,
    pub args: Vec<CallArg>,
    pub span: Span,
}

/// A call argument — either named (`key: value`) or positional.
#[derive(Debug, Clone, Serialize)]
pub enum CallArg {
    Named {
        key: String,
        value: Expr,
        span: Span,
    },
    Positional(Expr),
}

// ── Types ────────────────────────────────────────────────────

/// A full type expression: a union type optionally marked optional with `?`.
#[derive(Debug, Clone, Serialize)]
pub struct TypeExpr {
    pub ty: TypeKind,
    pub optional: bool,
    pub span: Span,
}

/// The shape of a type.
#[derive(Debug, Clone, Serialize)]
pub enum TypeKind {
    /// A single named type: `UUID`, `String`, `Active`.
    Simple(String),
    /// A union of two or more types: `Active | Frozen | Closed`.
    Union(Vec<TypeKind>),
    /// `List<T>`
    List(Box<TypeExpr>),
    /// `Set<T>`
    Set(Box<TypeExpr>),
    /// `Map<K, V>`
    Map(Box<TypeExpr>, Box<TypeExpr>),
    /// `Decimal(precision: 2)` — a type with parameters.
    Parameterized {
        name: String,
        params: Vec<TypeParam>,
    },
}

/// `name: value` inside a parameterized type like `Decimal(precision: 2)`.
#[derive(Debug, Clone, Serialize)]
pub struct TypeParam {
    pub name: String,
    pub value: Literal,
    pub span: Span,
}

// ── Expressions ──────────────────────────────────────────────

/// An expression node with its source span.
#[derive(Debug, Clone, Serialize)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// Expression variants.
#[derive(Debug, Clone, Serialize)]
pub enum ExprKind {
    /// `a => b` — logical implication.
    Implies(Box<Expr>, Box<Expr>),
    /// `a || b`
    Or(Box<Expr>, Box<Expr>),
    /// `a && b`
    And(Box<Expr>, Box<Expr>),
    /// `!a`
    Not(Box<Expr>),
    /// `a == b`, `a > b`, etc.
    Compare {
        left: Box<Expr>,
        op: CmpOp,
        right: Box<Expr>,
    },
    /// `a + b`, `a - b`
    Arithmetic {
        left: Box<Expr>,
        op: ArithOp,
        right: Box<Expr>,
    },
    /// `old(expr)` — pre-state reference.
    Old(Box<Expr>),
    /// `forall x: T => body` or `exists x: T => body`
    Quantifier {
        kind: QuantifierKind,
        binding: String,
        ty: String,
        body: Box<Expr>,
    },
    /// `name(args)` — function call.
    Call { name: String, args: Vec<CallArg> },
    /// Field access chain: `a.b.c` or `f(x).y.z`.
    /// `root` is the base expression, `fields` are the `.`-accessed names.
    FieldAccess {
        root: Box<Expr>,
        fields: Vec<String>,
    },
    /// List literal: `[a, b, c]`.
    List(Vec<Expr>),
    /// A plain identifier: `amount`, `Active`, `email`.
    Ident(String),
    /// A literal value.
    Literal(Literal),
}

impl Expr {
    /// Call `f` for each immediate child expression.
    ///
    /// Handles all `ExprKind` variants so callers don't need to duplicate
    /// the recursive descent boilerplate.
    pub fn for_each_child(&self, mut f: impl FnMut(&Expr)) {
        match &self.kind {
            ExprKind::Implies(a, b)
            | ExprKind::Or(a, b)
            | ExprKind::And(a, b)
            | ExprKind::Compare {
                left: a, right: b, ..
            }
            | ExprKind::Arithmetic {
                left: a, right: b, ..
            } => {
                f(a);
                f(b);
            }
            ExprKind::Not(inner) | ExprKind::Old(inner) => f(inner),
            ExprKind::Call { args, .. } => {
                for arg in args {
                    match arg {
                        CallArg::Named { value, .. } => f(value),
                        CallArg::Positional(e) => f(e),
                    }
                }
            }
            ExprKind::FieldAccess { root, .. } => f(root),
            ExprKind::Quantifier { body, .. } => f(body),
            ExprKind::List(items) => {
                for item in items {
                    f(item);
                }
            }
            ExprKind::Ident(_) | ExprKind::Literal(_) => {}
        }
    }
}

/// Comparison operators.
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

/// Quantifier kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum QuantifierKind {
    Forall,
    Exists,
}

/// Literal values.
#[derive(Debug, Clone, Serialize)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Decimal(String),
    String(String),
}
