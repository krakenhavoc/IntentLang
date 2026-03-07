//! Type checking for intent specifications.
//!
//! Validates:
//! - All referenced types exist (built-in or user-defined entities)
//! - Field accesses resolve to valid fields
//! - Operator type compatibility (comparison, arithmetic, logical)

use intent_parser::ast;

/// Collected type information from a parsed file.
#[derive(Debug, Default)]
pub struct TypeEnv {
    /// Entity names and their field maps.
    pub entities: Vec<(String, Vec<(String, ast::TypeExpr)>)>,
}

impl TypeEnv {
    /// Build a type environment from a parsed AST file.
    pub fn from_file(file: &ast::File) -> Self {
        let mut env = TypeEnv::default();
        for item in &file.items {
            if let ast::TopLevelItem::Entity(entity) = item {
                let fields = entity
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), f.ty.clone()))
                    .collect();
                env.entities.push((entity.name.clone(), fields));
            }
        }
        env
    }
}
