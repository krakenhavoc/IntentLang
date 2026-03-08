pub mod ast;
pub mod parser;
pub mod resolve;

#[cfg(test)]
mod snapshot_tests;

pub use ast::*;
pub use parser::{ParseError, parse_file};
pub use resolve::{ModuleGraph, ResolveError, resolve};
