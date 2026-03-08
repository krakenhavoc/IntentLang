//! Module resolver: loads, parses, and caches imported `.intent` files.
//!
//! Resolution is file-system based: `use Foo` looks for `Foo.intent` in the
//! same directory as the importing file.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::ast;
use crate::parser::ParseError;

/// A resolved module graph: the root file plus all transitively imported modules.
#[derive(Debug)]
pub struct ModuleGraph {
    /// The root file that was resolved.
    pub root: PathBuf,
    /// All parsed modules keyed by canonical path.
    pub modules: HashMap<PathBuf, ast::File>,
    /// Import order (topological — dependencies before dependents).
    pub order: Vec<PathBuf>,
}

/// Errors from module resolution.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("parse error in {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: ParseError,
    },
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("import cycle detected: {cycle}")]
    Cycle { cycle: String },
    #[error("module not found: `use {module_name}` — expected file {expected_path}")]
    ModuleNotFound {
        module_name: String,
        expected_path: PathBuf,
    },
}

/// Resolve a module and all its transitive imports.
///
/// Starting from `root_path`, parses the file, follows `use` declarations,
/// and builds a `ModuleGraph` with all modules in dependency order.
pub fn resolve(root_path: &Path) -> Result<ModuleGraph, ResolveError> {
    let root_path = std::fs::canonicalize(root_path).map_err(|e| ResolveError::Io {
        path: root_path.to_path_buf(),
        source: e,
    })?;

    let mut modules: HashMap<PathBuf, ast::File> = HashMap::new();
    let mut order: Vec<PathBuf> = Vec::new();
    let mut visiting: HashSet<PathBuf> = HashSet::new();

    resolve_recursive(&root_path, &mut modules, &mut order, &mut visiting)?;

    Ok(ModuleGraph {
        root: root_path,
        modules,
        order,
    })
}

fn resolve_recursive(
    path: &Path,
    modules: &mut HashMap<PathBuf, ast::File>,
    order: &mut Vec<PathBuf>,
    visiting: &mut HashSet<PathBuf>,
) -> Result<(), ResolveError> {
    // Already fully resolved.
    if modules.contains_key(path) {
        return Ok(());
    }

    // Cycle detection.
    if !visiting.insert(path.to_path_buf()) {
        let cycle = path.display().to_string();
        return Err(ResolveError::Cycle { cycle });
    }

    // Read and parse.
    let source = std::fs::read_to_string(path).map_err(|e| ResolveError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    let file = crate::parse_file(&source).map_err(|e| ResolveError::Parse {
        path: path.to_path_buf(),
        source: e,
    })?;

    // Resolve each import.
    let dir = path.parent().unwrap_or(Path::new("."));
    for use_decl in &file.imports {
        let import_filename = format!("{}.intent", use_decl.module_name);
        let import_path = dir.join(&import_filename);

        let canonical =
            std::fs::canonicalize(&import_path).map_err(|_| ResolveError::ModuleNotFound {
                module_name: use_decl.module_name.clone(),
                expected_path: import_path.clone(),
            })?;

        resolve_recursive(&canonical, modules, order, visiting)?;
    }

    // Done visiting — add to resolved set in topological order.
    visiting.remove(path);
    order.push(path.to_path_buf());
    modules.insert(path.to_path_buf(), file);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn resolve_single_file_no_imports() {
        let dir = setup_temp_dir();
        let root = dir.path().join("main.intent");
        fs::write(&root, "module Main\n\nentity Foo {\n  id: UUID\n}\n").unwrap();

        let graph = resolve(&root).unwrap();
        assert_eq!(graph.modules.len(), 1);
        assert_eq!(graph.order.len(), 1);
    }

    #[test]
    fn resolve_two_modules() {
        let dir = setup_temp_dir();

        fs::write(
            dir.path().join("Types.intent"),
            "module Types\n\nentity Account {\n  id: UUID\n  balance: Int\n}\n",
        )
        .unwrap();

        fs::write(
            dir.path().join("Main.intent"),
            "module Main\n\nuse Types\n\naction Transfer {\n  from: Account\n  to: Account\n}\n",
        )
        .unwrap();

        let graph = resolve(&dir.path().join("Main.intent")).unwrap();
        assert_eq!(graph.modules.len(), 2);
        // Types should come before Main in topological order
        let names: Vec<&str> = graph
            .order
            .iter()
            .map(|p| graph.modules[p].module.name.as_str())
            .collect();
        assert_eq!(names, vec!["Types", "Main"]);
    }

    #[test]
    fn resolve_selective_import() {
        let dir = setup_temp_dir();

        fs::write(
            dir.path().join("Types.intent"),
            "module Types\n\nentity Account {\n  id: UUID\n}\n\nentity User {\n  name: String\n}\n",
        )
        .unwrap();

        fs::write(
            dir.path().join("Main.intent"),
            "module Main\n\nuse Types.Account\n\naction Foo {\n  a: Account\n}\n",
        )
        .unwrap();

        let graph = resolve(&dir.path().join("Main.intent")).unwrap();
        assert_eq!(graph.modules.len(), 2);
        // The import is selective — Main only imports Account
        let main_file =
            &graph.modules[&std::fs::canonicalize(dir.path().join("Main.intent")).unwrap()];
        assert_eq!(main_file.imports[0].module_name, "Types");
        assert_eq!(main_file.imports[0].item.as_deref(), Some("Account"));
    }

    #[test]
    fn resolve_cycle_detected() {
        let dir = setup_temp_dir();

        fs::write(dir.path().join("A.intent"), "module A\n\nuse B\n").unwrap();

        fs::write(dir.path().join("B.intent"), "module B\n\nuse A\n").unwrap();

        let err = resolve(&dir.path().join("A.intent")).unwrap_err();
        assert!(matches!(err, ResolveError::Cycle { .. }));
    }

    #[test]
    fn resolve_module_not_found() {
        let dir = setup_temp_dir();

        fs::write(
            dir.path().join("Main.intent"),
            "module Main\n\nuse NonExistent\n",
        )
        .unwrap();

        let err = resolve(&dir.path().join("Main.intent")).unwrap_err();
        assert!(matches!(err, ResolveError::ModuleNotFound { .. }));
    }

    #[test]
    fn resolve_transitive_imports() {
        let dir = setup_temp_dir();

        fs::write(
            dir.path().join("Base.intent"),
            "module Base\n\nentity Id {\n  value: UUID\n}\n",
        )
        .unwrap();

        fs::write(
            dir.path().join("Types.intent"),
            "module Types\n\nuse Base\n\nentity Account {\n  id: UUID\n}\n",
        )
        .unwrap();

        fs::write(
            dir.path().join("Main.intent"),
            "module Main\n\nuse Types\n\naction Foo {\n  a: Account\n}\n",
        )
        .unwrap();

        let graph = resolve(&dir.path().join("Main.intent")).unwrap();
        assert_eq!(graph.modules.len(), 3);
        let names: Vec<&str> = graph
            .order
            .iter()
            .map(|p| graph.modules[p].module.name.as_str())
            .collect();
        assert_eq!(names, vec!["Base", "Types", "Main"]);
    }

    #[test]
    fn resolve_diamond_dependency() {
        let dir = setup_temp_dir();

        fs::write(
            dir.path().join("Base.intent"),
            "module Base\n\nentity Id {\n  value: UUID\n}\n",
        )
        .unwrap();

        fs::write(
            dir.path().join("Left.intent"),
            "module Left\n\nuse Base\n\nentity Foo {\n  id: UUID\n}\n",
        )
        .unwrap();

        fs::write(
            dir.path().join("Right.intent"),
            "module Right\n\nuse Base\n\nentity Bar {\n  id: UUID\n}\n",
        )
        .unwrap();

        fs::write(
            dir.path().join("Main.intent"),
            "module Main\n\nuse Left\nuse Right\n",
        )
        .unwrap();

        let graph = resolve(&dir.path().join("Main.intent")).unwrap();
        assert_eq!(graph.modules.len(), 4);
        // Base should appear only once and before Left/Right
        let names: Vec<&str> = graph
            .order
            .iter()
            .map(|p| graph.modules[p].module.name.as_str())
            .collect();
        assert_eq!(names[0], "Base");
        assert!(names.contains(&"Left"));
        assert!(names.contains(&"Right"));
        assert_eq!(names[3], "Main");
    }
}
