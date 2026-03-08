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

// ── Go generation ──────────────────────────────────────────

#[test]
fn go_entity_struct() {
    let src = "module Test\nentity Item {\n  id: UUID\n  name: String\n}";
    let out = codegen(src, Language::Go);
    assert!(out.contains("type Item struct {"));
    assert!(out.contains("Id uuid.UUID `json:\"id\"`"));
    assert!(out.contains("Name string `json:\"name\"`"));
}

#[test]
fn go_package_declaration() {
    let src = "module Transfer\nentity X { id: UUID }";
    let out = codegen(src, Language::Go);
    assert!(out.contains("package transfer"));
}

#[test]
fn go_optional_field() {
    let src = "module Test\nentity Item {\n  label: String?\n}";
    let out = codegen(src, Language::Go);
    assert!(out.contains("*string"));
}

#[test]
fn go_union_const_block() {
    let src = "module Test\nentity Account {\n  status: Active | Frozen\n}";
    let out = codegen(src, Language::Go);
    assert!(out.contains("type AccountStatus string"));
    assert!(out.contains("AccountStatusActive AccountStatus = \"Active\""));
    assert!(out.contains("AccountStatusFrozen AccountStatus = \"Frozen\""));
    assert!(out.contains("Status AccountStatus"));
}

#[test]
fn go_union_valid_method() {
    let src = "module Test\nentity Account {\n  status: Active | Frozen\n}";
    let out = codegen(src, Language::Go);
    assert!(out.contains("func (v AccountStatus) Valid() bool {"));
    assert!(out.contains("func (v *AccountStatus) UnmarshalText(data []byte) error {"));
}

#[test]
fn go_action_function() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n  requires {\n    x.id != x.id\n  }\n}";
    let out = codegen(src, Language::Go);
    assert!(out.contains("func DoThing("));
    assert!(out.contains("return errors.New(\"TODO: implement DoThing\")"));
    assert!(out.contains("Requires:"));
}

#[test]
fn go_imports_uuid() {
    let src = "module Test\nentity Item {\n  id: UUID\n}";
    let out = codegen(src, Language::Go);
    assert!(out.contains("\"github.com/google/uuid\""));
}

#[test]
fn go_imports_decimal() {
    let src = "module Test\nentity Item {\n  price: Decimal(precision: 2)\n}";
    let out = codegen(src, Language::Go);
    assert!(out.contains("\"github.com/shopspring/decimal\""));
}

#[test]
fn go_imports_time() {
    let src = "module Test\nentity Item {\n  ts: DateTime\n}";
    let out = codegen(src, Language::Go);
    assert!(out.contains("\"time\""));
}

#[test]
fn go_collection_types() {
    let src = "module Test\nentity Item {\n  tags: List<String>\n  ids: Set<UUID>\n  meta: Map<String, Int>\n}";
    let out = codegen(src, Language::Go);
    assert!(out.contains("[]string"));
    assert!(out.contains("map[uuid.UUID]struct{}"));
    assert!(out.contains("map[string]int64"));
}

#[test]
fn go_doc_block() {
    let src = "module Test\n--- A test module.\nentity X { id: UUID }";
    let out = codegen(src, Language::Go);
    assert!(out.contains("// A test module."));
}

#[test]
fn go_invariant_comment() {
    let src =
        "module Test\nentity X { id: UUID }\ninvariant Unique {\n  forall a: X => a.id == a.id\n}";
    let out = codegen(src, Language::Go);
    assert!(out.contains("// Invariant: Unique"));
}

#[test]
fn go_json_tags() {
    let src = "module Test\nentity Item {\n  created_at: DateTime\n  full_name: String\n}";
    let out = codegen(src, Language::Go);
    assert!(out.contains("`json:\"createdAt\"`"));
    assert!(out.contains("`json:\"fullName\"`"));
}

#[test]
fn go_pascal_case_fields() {
    let src = "module Test\nentity Item {\n  created_at: DateTime\n  full_name: String\n}";
    let out = codegen(src, Language::Go);
    assert!(out.contains("CreatedAt time.Time"));
    assert!(out.contains("FullName string"));
}

#[test]
fn go_generated_header() {
    let src = "module Test\nentity X { id: UUID }";
    let out = codegen(src, Language::Go);
    assert!(out.contains("// Code generated from Test.intent. DO NOT EDIT."));
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
    for lang in [
        Language::Rust,
        Language::TypeScript,
        Language::Python,
        Language::Go,
        Language::Java,
        Language::CSharp,
        Language::Swift,
    ] {
        let out = codegen(src, lang);
        assert!(!out.is_empty(), "output should not be empty for {:?}", lang);
    }
}

#[test]
fn auth_all_langs() {
    let src = include_str!("../../../examples/auth.intent");
    for lang in [
        Language::Rust,
        Language::TypeScript,
        Language::Python,
        Language::Go,
        Language::Java,
        Language::CSharp,
        Language::Swift,
    ] {
        let out = codegen(src, lang);
        assert!(!out.is_empty(), "output should not be empty for {:?}", lang);
    }
}

#[test]
fn transfer_go() {
    let src = include_str!("../../../examples/transfer.intent");
    let out = codegen(src, Language::Go);
    assert!(out.contains("type Account struct {"));
    assert!(out.contains("type TransferRecord struct {"));
    assert!(out.contains("func Transfer("));
    assert!(out.contains("func FreezeAccount("));
    assert!(out.contains("// Invariant: NoNegativeBalances"));
    assert!(out.contains("// Edge cases:"));
    assert!(out.contains("package transfer"));
}

// ── Java generation ────────────────────────────────────────

#[test]
fn java_entity_record() {
    let src = "module Test\nentity Item {\n  id: UUID\n  name: String\n}";
    let out = codegen(src, Language::Java);
    assert!(out.contains("public record Item("));
    assert!(out.contains("UUID id"));
    assert!(out.contains("String name"));
}

#[test]
fn java_package_declaration() {
    let src = "module Transfer\nentity X { id: UUID }";
    let out = codegen(src, Language::Java);
    assert!(out.contains("package transfer;"));
}

#[test]
fn java_module_class() {
    let src = "module Transfer\nentity X { id: UUID }";
    let out = codegen(src, Language::Java);
    assert!(out.contains("public final class Transfer {"));
    assert!(out.contains("private Transfer() {}"));
}

#[test]
fn java_optional_field() {
    let src = "module Test\nentity Item {\n  label: String?\n  count: Int?\n}";
    let out = codegen(src, Language::Java);
    // String stays String (reference type, nullable by default)
    assert!(out.contains("String label"));
    // Int? becomes Long (boxed primitive)
    assert!(out.contains("Long count"));
}

#[test]
fn java_union_generates_enum() {
    let src = "module Test\nentity Account {\n  status: Active | Frozen\n}";
    let out = codegen(src, Language::Java);
    assert!(out.contains("public enum AccountStatus {"));
    assert!(out.contains("Active"));
    assert!(out.contains("Frozen"));
    assert!(out.contains("AccountStatus status"));
}

#[test]
fn java_action_method() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n  requires {\n    x.id != x.id\n  }\n}";
    let out = codegen(src, Language::Java);
    assert!(out.contains("public static void doThing("));
    assert!(out.contains("throw new UnsupportedOperationException(\"TODO: implement doThing\")"));
    assert!(out.contains("Requires:"));
}

#[test]
fn java_imports_uuid() {
    let src = "module Test\nentity Item {\n  id: UUID\n}";
    let out = codegen(src, Language::Java);
    assert!(out.contains("import java.util.UUID;"));
}

#[test]
fn java_imports_bigdecimal() {
    let src = "module Test\nentity Item {\n  price: Decimal(precision: 2)\n}";
    let out = codegen(src, Language::Java);
    assert!(out.contains("import java.math.BigDecimal;"));
}

#[test]
fn java_imports_instant() {
    let src = "module Test\nentity Item {\n  ts: DateTime\n}";
    let out = codegen(src, Language::Java);
    assert!(out.contains("import java.time.Instant;"));
}

#[test]
fn java_collection_types() {
    let src = "module Test\nentity Item {\n  tags: List<String>\n  ids: Set<UUID>\n  meta: Map<String, Int>\n}";
    let out = codegen(src, Language::Java);
    assert!(out.contains("List<String>"));
    assert!(out.contains("Set<UUID>"));
    assert!(out.contains("Map<String, Long>"));
}

#[test]
fn java_doc_block() {
    let src = "module Test\n--- A test module.\nentity X { id: UUID }";
    let out = codegen(src, Language::Java);
    assert!(out.contains("// A test module."));
}

#[test]
fn java_invariant_comment() {
    let src =
        "module Test\nentity X { id: UUID }\ninvariant Unique {\n  forall a: X => a.id == a.id\n}";
    let out = codegen(src, Language::Java);
    assert!(out.contains("// Invariant: Unique"));
}

#[test]
fn java_generated_header() {
    let src = "module Test\nentity X { id: UUID }";
    let out = codegen(src, Language::Java);
    assert!(out.contains("// Generated from Test.intent. DO NOT EDIT."));
}

#[test]
fn transfer_java() {
    let src = include_str!("../../../examples/transfer.intent");
    let out = codegen(src, Language::Java);
    assert!(out.contains("public record Account("));
    assert!(out.contains("public record TransferRecord("));
    assert!(out.contains("public static void transfer("));
    assert!(out.contains("public static void freezeAccount("));
    assert!(out.contains("// Invariant: NoNegativeBalances"));
    assert!(out.contains("// Edge cases:"));
    assert!(out.contains("package transferfunds;"));
}

// ── C# generation ──────────────────────────────────────────

#[test]
fn csharp_entity_record() {
    let src = "module Test\nentity Item {\n  id: UUID\n  name: String\n}";
    let out = codegen(src, Language::CSharp);
    assert!(out.contains("public record Item("));
    assert!(out.contains("Guid Id"));
    assert!(out.contains("string Name"));
}

#[test]
fn csharp_namespace() {
    let src = "module Transfer\nentity X { id: UUID }";
    let out = codegen(src, Language::CSharp);
    assert!(out.contains("namespace Transfer;"));
}

#[test]
fn csharp_nullable_enable() {
    let src = "module Test\nentity X { id: UUID }";
    let out = codegen(src, Language::CSharp);
    assert!(out.contains("#nullable enable"));
}

#[test]
fn csharp_optional_field() {
    let src = "module Test\nentity Item {\n  label: String?\n  count: Int?\n}";
    let out = codegen(src, Language::CSharp);
    assert!(out.contains("string? Label"));
    assert!(out.contains("long? Count"));
}

#[test]
fn csharp_union_generates_enum() {
    let src = "module Test\nentity Account {\n  status: Active | Frozen\n}";
    let out = codegen(src, Language::CSharp);
    assert!(out.contains("public enum AccountStatus"));
    assert!(out.contains("Active"));
    assert!(out.contains("Frozen"));
    assert!(out.contains("AccountStatus Status"));
}

#[test]
fn csharp_action_method() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n  requires {\n    x.id != x.id\n  }\n}";
    let out = codegen(src, Language::CSharp);
    assert!(out.contains("public static void DoThing("));
    assert!(out.contains("throw new NotImplementedException(\"TODO: implement DoThing\")"));
    assert!(out.contains("Requires:"));
}

#[test]
fn csharp_static_actions_class() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n}";
    let out = codegen(src, Language::CSharp);
    assert!(out.contains("public static class TestActions"));
}

#[test]
fn csharp_collection_types() {
    let src = "module Test\nentity Item {\n  tags: List<String>\n  ids: Set<UUID>\n  meta: Map<String, Int>\n}";
    let out = codegen(src, Language::CSharp);
    assert!(out.contains("List<string>"));
    assert!(out.contains("HashSet<Guid>"));
    assert!(out.contains("Dictionary<string, long>"));
}

#[test]
fn csharp_doc_block() {
    let src = "module Test\n--- A test module.\nentity X { id: UUID }";
    let out = codegen(src, Language::CSharp);
    assert!(out.contains("// A test module."));
}

#[test]
fn csharp_invariant_comment() {
    let src =
        "module Test\nentity X { id: UUID }\ninvariant Unique {\n  forall a: X => a.id == a.id\n}";
    let out = codegen(src, Language::CSharp);
    assert!(out.contains("// Invariant: Unique"));
}

#[test]
fn csharp_generated_header() {
    let src = "module Test\nentity X { id: UUID }";
    let out = codegen(src, Language::CSharp);
    assert!(out.contains("// Generated from Test.intent. DO NOT EDIT."));
}

#[test]
fn transfer_csharp() {
    let src = include_str!("../../../examples/transfer.intent");
    let out = codegen(src, Language::CSharp);
    assert!(out.contains("public record Account("));
    assert!(out.contains("public record TransferRecord("));
    assert!(out.contains("public static void Transfer("));
    assert!(out.contains("public static void FreezeAccount("));
    assert!(out.contains("// Invariant: NoNegativeBalances"));
    assert!(out.contains("// Edge cases:"));
    assert!(out.contains("namespace TransferFunds;"));
}

// ── Swift generation ───────────────────────────────────────

#[test]
fn swift_entity_struct() {
    let src = "module Test\nentity Item {\n  id: UUID\n  name: String\n}";
    let out = codegen(src, Language::Swift);
    assert!(out.contains("struct Item: Codable {"));
    assert!(out.contains("let id: UUID"));
    assert!(out.contains("let name: String"));
}

#[test]
fn swift_import_foundation() {
    let src = "module Test\nentity X { id: UUID }";
    let out = codegen(src, Language::Swift);
    assert!(out.contains("import Foundation"));
}

#[test]
fn swift_optional_field() {
    let src = "module Test\nentity Item {\n  label: String?\n  count: Int?\n}";
    let out = codegen(src, Language::Swift);
    assert!(out.contains("String?"));
    assert!(out.contains("Int?"));
}

#[test]
fn swift_union_generates_enum() {
    let src = "module Test\nentity Account {\n  status: Active | Frozen\n}";
    let out = codegen(src, Language::Swift);
    assert!(out.contains("enum AccountStatus: String, Codable {"));
    assert!(out.contains("case active = \"Active\""));
    assert!(out.contains("case frozen = \"Frozen\""));
    assert!(out.contains("AccountStatus"));
}

#[test]
fn swift_action_function() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n  requires {\n    x.id != x.id\n  }\n}";
    let out = codegen(src, Language::Swift);
    assert!(out.contains("func doThing("));
    assert!(out.contains("throws {"));
    assert!(out.contains("fatalError(\"TODO: implement doThing\")"));
    assert!(out.contains("Requires:"));
}

#[test]
fn swift_collection_types() {
    let src = "module Test\nentity Item {\n  tags: List<String>\n  ids: Set<UUID>\n  meta: Map<String, Int>\n}";
    let out = codegen(src, Language::Swift);
    assert!(out.contains("[String]"));
    assert!(out.contains("Set<UUID>"));
    assert!(out.contains("[String: Int]"));
}

#[test]
fn swift_doc_block() {
    let src = "module Test\n--- A test module.\nentity X { id: UUID }";
    let out = codegen(src, Language::Swift);
    assert!(out.contains("// A test module."));
}

#[test]
fn swift_invariant_comment() {
    let src =
        "module Test\nentity X { id: UUID }\ninvariant Unique {\n  forall a: X => a.id == a.id\n}";
    let out = codegen(src, Language::Swift);
    assert!(out.contains("// Invariant: Unique"));
}

#[test]
fn swift_generated_header() {
    let src = "module Test\nentity X { id: UUID }";
    let out = codegen(src, Language::Swift);
    assert!(out.contains("// Generated from Test.intent. DO NOT EDIT."));
}

#[test]
fn transfer_swift() {
    let src = include_str!("../../../examples/transfer.intent");
    let out = codegen(src, Language::Swift);
    assert!(out.contains("struct Account: Codable {"));
    assert!(out.contains("struct TransferRecord: Codable {"));
    assert!(out.contains("func transfer("));
    assert!(out.contains("func freezeAccount("));
    assert!(out.contains("// Invariant: NoNegativeBalances"));
    assert!(out.contains("// Edge cases:"));
}
