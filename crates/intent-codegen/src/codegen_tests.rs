//! Tests for the codegen module.

use intent_parser::parse_file;

use crate::{Language, generate, to_camel_case, to_snake_case};

fn codegen(src: &str, lang: Language) -> String {
    let file = parse_file(src).expect("parse should succeed");
    generate(&file, lang)
}

// ── Naming helpers ──────────────────────────────────────────

#[test]
fn snake_case_conversion() {
    assert_eq!(to_snake_case("TransferFunds"), "transfer_funds");
    assert_eq!(to_snake_case("Account"), "account");
    assert_eq!(to_snake_case("FreezeAccount"), "freeze_account");
    assert_eq!(to_snake_case("HTTPServer"), "h_t_t_p_server");
}

#[test]
fn camel_case_conversion() {
    assert_eq!(to_camel_case("TransferFunds"), "transferFunds");
    assert_eq!(to_camel_case("Account"), "account");
    assert_eq!(to_camel_case("FreezeAccount"), "freezeAccount");
}

// ── Rust generation ─────────────────────────────────────────

#[test]
fn rust_entity_struct() {
    let src = "module Test\nentity Item {\n  id: UUID\n  name: String\n}";
    let out = codegen(src, Language::Rust);
    assert!(out.contains("pub struct Item {"));
    assert!(out.contains("pub id: Uuid,"));
    assert!(out.contains("pub name: String,"));
}

#[test]
fn rust_optional_field() {
    let src = "module Test\nentity Item {\n  label: String?\n}";
    let out = codegen(src, Language::Rust);
    assert!(out.contains("Option<String>"));
}

#[test]
fn rust_union_generates_enum() {
    let src = "module Test\nentity Account {\n  status: Active | Frozen\n}";
    let out = codegen(src, Language::Rust);
    assert!(out.contains("pub enum AccountStatus {"));
    assert!(out.contains("Active,"));
    assert!(out.contains("Frozen,"));
    assert!(out.contains("pub status: AccountStatus,"));
}

#[test]
fn rust_action_function() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n  requires {\n    x.id != x.id\n  }\n}";
    let out = codegen(src, Language::Rust);
    assert!(out.contains("pub fn do_thing("));
    assert!(out.contains("todo!(\"Implement do_thing\")"));
    assert!(out.contains("# Requires"));
}

#[test]
fn rust_imports_uuid() {
    let src = "module Test\nentity Item {\n  id: UUID\n}";
    let out = codegen(src, Language::Rust);
    assert!(out.contains("use uuid::Uuid;"));
}

#[test]
fn rust_imports_decimal() {
    let src = "module Test\nentity Item {\n  price: Decimal(precision: 2)\n}";
    let out = codegen(src, Language::Rust);
    assert!(out.contains("use rust_decimal::Decimal;"));
}

#[test]
fn rust_imports_datetime() {
    let src = "module Test\nentity Item {\n  ts: DateTime\n}";
    let out = codegen(src, Language::Rust);
    assert!(out.contains("use chrono::{DateTime, Utc};"));
}

#[test]
fn rust_collection_types() {
    let src = "module Test\nentity Item {\n  tags: List<String>\n  ids: Set<UUID>\n  meta: Map<String, Int>\n}";
    let out = codegen(src, Language::Rust);
    assert!(out.contains("Vec<String>"));
    assert!(out.contains("HashSet<Uuid>"));
    assert!(out.contains("HashMap<String, i64>"));
}

#[test]
fn rust_doc_block() {
    let src = "module Test\n--- A test module.\nentity X { id: UUID }";
    let out = codegen(src, Language::Rust);
    assert!(out.contains("//! A test module."));
}

#[test]
fn rust_invariant_comment() {
    let src =
        "module Test\nentity X { id: UUID }\ninvariant Unique {\n  forall a: X => a.id == a.id\n}";
    let out = codegen(src, Language::Rust);
    assert!(out.contains("// Invariant: Unique"));
}

// ── TypeScript generation ───────────────────────────────────

#[test]
fn ts_entity_interface() {
    let src = "module Test\nentity Item {\n  id: UUID\n  name: String\n}";
    let out = codegen(src, Language::TypeScript);
    assert!(out.contains("export interface Item {"));
    assert!(out.contains("id: string;"));
    assert!(out.contains("name: string;"));
}

#[test]
fn ts_union_inline() {
    let src = "module Test\nentity Account {\n  status: Active | Frozen\n}";
    let out = codegen(src, Language::TypeScript);
    assert!(out.contains("\"Active\" | \"Frozen\""));
}

#[test]
fn ts_optional_field() {
    let src = "module Test\nentity Item {\n  label: String?\n}";
    let out = codegen(src, Language::TypeScript);
    assert!(out.contains("string | null"));
}

#[test]
fn ts_action_function() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n}";
    let out = codegen(src, Language::TypeScript);
    assert!(out.contains("export function doThing("));
    assert!(out.contains("throw new Error(\"TODO: implement doThing\")"));
}

#[test]
fn ts_camel_case_fields() {
    let src = "module Test\nentity Item {\n  created_at: DateTime\n}";
    let out = codegen(src, Language::TypeScript);
    assert!(out.contains("createdAt: Date;"));
}

// ── Python generation ───────────────────────────────────────

#[test]
fn python_entity_dataclass() {
    let src = "module Test\nentity Item {\n  id: UUID\n  name: String\n}";
    let out = codegen(src, Language::Python);
    assert!(out.contains("@dataclass"));
    assert!(out.contains("class Item:"));
    assert!(out.contains("id: uuid.UUID"));
    assert!(out.contains("name: str"));
}

#[test]
fn python_union_literal() {
    let src = "module Test\nentity Account {\n  status: Active | Frozen\n}";
    let out = codegen(src, Language::Python);
    assert!(out.contains("Literal[\"Active\", \"Frozen\"]"));
    assert!(out.contains("from typing import Literal"));
}

#[test]
fn python_optional_field() {
    let src = "module Test\nentity Item {\n  label: String?\n}";
    let out = codegen(src, Language::Python);
    assert!(out.contains("str | None"));
}

#[test]
fn python_action_function() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n}";
    let out = codegen(src, Language::Python);
    assert!(out.contains("def do_thing("));
    assert!(out.contains("raise NotImplementedError(\"TODO: implement do_thing\")"));
}

#[test]
fn python_imports() {
    let src =
        "module Test\nentity Item {\n  id: UUID\n  ts: DateTime\n  price: Decimal(precision: 2)\n}";
    let out = codegen(src, Language::Python);
    assert!(out.contains("from __future__ import annotations"));
    assert!(out.contains("from dataclasses import dataclass"));
    assert!(out.contains("import uuid"));
    assert!(out.contains("from datetime import datetime"));
    assert!(out.contains("from decimal import Decimal"));
}

// ── Full example files ──────────────────────────────────────

#[test]
fn transfer_rust_compiles() {
    let src = include_str!("../../../examples/transfer.intent");
    let out = codegen(src, Language::Rust);
    // Smoke test: the output should have key elements
    assert!(out.contains("pub struct Account"));
    assert!(out.contains("pub struct TransferRecord"));
    assert!(out.contains("pub fn transfer("));
    assert!(out.contains("pub fn freeze_account("));
    assert!(out.contains("// Invariant: NoNegativeBalances"));
    assert!(out.contains("// Edge cases:"));
}

#[test]
fn transfer_typescript() {
    let src = include_str!("../../../examples/transfer.intent");
    let out = codegen(src, Language::TypeScript);
    assert!(out.contains("export interface Account"));
    assert!(out.contains("export interface TransferRecord"));
    assert!(out.contains("export function transfer("));
    assert!(out.contains("export function freezeAccount("));
}

#[test]
fn transfer_python() {
    let src = include_str!("../../../examples/transfer.intent");
    let out = codegen(src, Language::Python);
    assert!(out.contains("class Account:"));
    assert!(out.contains("class TransferRecord:"));
    assert!(out.contains("def transfer("));
    assert!(out.contains("def freeze_account("));
}

#[test]
fn shopping_cart_all_langs() {
    let src = include_str!("../../../examples/shopping_cart.intent");
    for lang in [Language::Rust, Language::TypeScript, Language::Python] {
        let out = codegen(src, lang);
        assert!(!out.is_empty(), "output should not be empty for {:?}", lang);
    }
}

#[test]
fn auth_all_langs() {
    let src = include_str!("../../../examples/auth.intent");
    for lang in [Language::Rust, Language::TypeScript, Language::Python] {
        let out = codegen(src, lang);
        assert!(!out.is_empty(), "output should not be empty for {:?}", lang);
    }
}
