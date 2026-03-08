//! Per-file document state: source text, line index, cached AST and errors.

use intent_check::CheckError;
use intent_parser::ParseError;
use intent_parser::ast;
use tower_lsp::lsp_types::{Position, Range};

/// Byte-offset to LSP position converter.
///
/// Pre-computes line start offsets so lookups are O(log n) via binary search.
pub struct LineIndex {
    /// Byte offset of the start of each line (line 0 starts at offset 0).
    line_starts: Vec<usize>,
}

impl LineIndex {
    /// Build a line index from source text.
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, byte) in source.as_bytes().iter().enumerate() {
            if *byte == b'\n' {
                line_starts.push(i + 1);
            }
        }
        LineIndex { line_starts }
    }

    /// Convert a byte offset to an LSP `Position` (line + UTF-16 character).
    pub fn offset_to_position(&self, offset: usize, source: &str) -> Position {
        let line = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);

        let line_start = self.line_starts[line];
        let line_text = &source[line_start..offset.min(source.len())];

        // Count UTF-16 code units for the character offset.
        let character: u32 = line_text.chars().map(|c| c.len_utf16() as u32).sum();

        Position {
            line: line as u32,
            character,
        }
    }

    /// Convert an LSP `Position` to a byte offset.
    pub fn position_to_offset(&self, pos: Position, source: &str) -> usize {
        let line = pos.line as usize;
        if line >= self.line_starts.len() {
            return source.len();
        }
        let line_start = self.line_starts[line];
        let line_text = &source[line_start..];

        let mut utf16_count: u32 = 0;
        let mut byte_offset = 0;
        for ch in line_text.chars() {
            if ch == '\n' || utf16_count >= pos.character {
                break;
            }
            utf16_count += ch.len_utf16() as u32;
            byte_offset += ch.len_utf8();
        }

        line_start + byte_offset
    }

    /// Convert an `ast::Span` (byte offsets) to an LSP `Range`.
    pub fn span_to_range(&self, span: ast::Span, source: &str) -> Range {
        Range {
            start: self.offset_to_position(span.start, source),
            end: self.offset_to_position(span.end, source),
        }
    }

    /// Convert a `miette::SourceSpan` (offset, length) to an LSP `Range`.
    pub fn source_span_to_range(&self, span: miette::SourceSpan, source: &str) -> Range {
        let start = span.offset();
        let end = start + span.len();
        Range {
            start: self.offset_to_position(start, source),
            end: self.offset_to_position(end, source),
        }
    }
}

/// Cached state for an open document.
pub struct Document {
    pub source: String,
    pub line_index: LineIndex,
    pub ast: Option<ast::File>,
    pub parse_error: Option<ParseError>,
    pub check_errors: Vec<CheckError>,
}

impl Document {
    /// Parse and check the given source text, caching all results.
    pub fn new(source: String, file_path: Option<&std::path::Path>) -> Self {
        let line_index = LineIndex::new(&source);

        match intent_parser::parse_file(&source) {
            Err(parse_error) => Document {
                source,
                line_index,
                ast: None,
                parse_error: Some(parse_error),
                check_errors: Vec::new(),
            },
            Ok(ast) => {
                let check_errors = run_checks(&ast, file_path);
                Document {
                    source,
                    line_index,
                    ast: Some(ast),
                    parse_error: None,
                    check_errors,
                }
            }
        }
    }
}

/// Run semantic checks, resolving imports if the file is on disk.
fn run_checks(ast: &ast::File, file_path: Option<&std::path::Path>) -> Vec<CheckError> {
    if ast.imports.is_empty() || file_path.is_none() {
        return intent_check::check_file(ast);
    }

    let path = file_path.unwrap();
    match intent_parser::resolve(path) {
        Ok(graph) => {
            let root_file = &graph.modules[&graph.root];
            let imported: Vec<&ast::File> = graph
                .order
                .iter()
                .filter(|p| **p != graph.root)
                .filter_map(|p| graph.modules.get(p))
                .collect();
            intent_check::check_file_with_imports(root_file, &imported)
        }
        Err(_) => {
            // Fall back to single-file check if resolution fails.
            intent_check::check_file(ast)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_index_single_line() {
        let src = "module Foo";
        let idx = LineIndex::new(src);
        assert_eq!(
            idx.offset_to_position(0, src),
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            idx.offset_to_position(7, src),
            Position {
                line: 0,
                character: 7
            }
        );
    }

    #[test]
    fn line_index_multi_line() {
        let src = "module Foo\n\nentity Bar {\n  id: UUID\n}\n";
        let idx = LineIndex::new(src);

        // "entity" starts at line 2, col 0
        let entity_offset = src.find("entity").unwrap();
        assert_eq!(
            idx.offset_to_position(entity_offset, src),
            Position {
                line: 2,
                character: 0
            }
        );

        // "id" starts at line 3, col 2
        let id_offset = src.find("id:").unwrap();
        assert_eq!(
            idx.offset_to_position(id_offset, src),
            Position {
                line: 3,
                character: 2
            }
        );
    }

    #[test]
    fn position_to_offset_roundtrip() {
        let src = "module Foo\n\nentity Bar {\n  id: UUID\n}\n";
        let idx = LineIndex::new(src);

        let pos = Position {
            line: 2,
            character: 7,
        };
        let offset = idx.position_to_offset(pos, src);
        let back = idx.offset_to_position(offset, src);
        assert_eq!(back, pos);
    }

    #[test]
    fn span_to_range_conversion() {
        let src = "module Foo\nentity Bar {\n}\n";
        let idx = LineIndex::new(src);
        let span = ast::Span { start: 11, end: 21 };
        let range = idx.span_to_range(span, src);
        assert_eq!(range.start.line, 1);
        assert_eq!(range.start.character, 0);
    }

    #[test]
    fn source_span_to_range_conversion() {
        let src = "module Foo\nentity Bar {\n}\n";
        let idx = LineIndex::new(src);
        let span: miette::SourceSpan = (11, 10).into();
        let range = idx.source_span_to_range(span, src);
        assert_eq!(range.start.line, 1);
        assert_eq!(range.start.character, 0);
    }

    #[test]
    fn document_parse_success() {
        let src = "module Test\n\nentity Foo {\n  id: UUID\n}\n";
        let doc = Document::new(src.to_string(), None);
        assert!(doc.ast.is_some());
        assert!(doc.parse_error.is_none());
        assert!(doc.check_errors.is_empty());
    }

    #[test]
    fn document_parse_failure() {
        let src = "this is not valid intent";
        let doc = Document::new(src.to_string(), None);
        assert!(doc.ast.is_none());
        assert!(doc.parse_error.is_some());
    }

    #[test]
    fn document_check_errors() {
        let src = "module Test\n\nentity Foo {\n  id: NonExistent\n}\n";
        let doc = Document::new(src.to_string(), None);
        assert!(doc.ast.is_some());
        assert!(!doc.check_errors.is_empty());
    }
}
