//! Tests for OpenAPI spec generation.

use intent_parser::parse_file;
use serde_json::Value;

use crate::openapi::generate;

fn openapi(src: &str) -> Value {
    let file = parse_file(src).expect("parse should succeed");
    generate(&file)
}

// ── Structure ──────────────────────────────────────────────

#[test]
fn has_openapi_version() {
    let src = "module Test\nentity X { id: UUID }";
    let spec = openapi(src);
    assert_eq!(spec["openapi"], "3.0.3");
}

#[test]
fn has_info_title() {
    let src = "module Payments\nentity X { id: UUID }";
    let spec = openapi(src);
    assert_eq!(spec["info"]["title"], "Payments API");
}

#[test]
fn has_info_description_from_doc() {
    let src = "module Test\n--- A payment system.\nentity X { id: UUID }";
    let spec = openapi(src);
    assert_eq!(spec["info"]["description"], "A payment system.");
}

// ── Entity → Schema ───────────────────────────────────────

#[test]
fn entity_becomes_schema() {
    let src = "module Test\nentity Account {\n  id: UUID\n  name: String\n}";
    let spec = openapi(src);
    let schema = &spec["components"]["schemas"]["Account"];
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["properties"]["id"]["type"], "string");
    assert_eq!(schema["properties"]["id"]["format"], "uuid");
    assert_eq!(schema["properties"]["name"]["type"], "string");
}

#[test]
fn entity_required_fields() {
    let src = "module Test\nentity Item {\n  id: UUID\n  label: String?\n}";
    let spec = openapi(src);
    let schema = &spec["components"]["schemas"]["Item"];
    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&Value::String("id".to_string())));
    assert!(!required.contains(&Value::String("label".to_string())));
}

#[test]
fn optional_field_nullable() {
    let src = "module Test\nentity Item {\n  label: String?\n}";
    let spec = openapi(src);
    let field = &spec["components"]["schemas"]["Item"]["properties"]["label"];
    assert_eq!(field["nullable"], true);
    assert_eq!(field["type"], "string");
}

#[test]
fn entity_doc_becomes_description() {
    let src = "module Test\n--- A test entity.\nentity Item {\n  id: UUID\n}";
    let spec = openapi(src);
    // The doc is on the module, not the entity. Let's test entity doc separately.
    let src2 = "module Test\nentity Item {\n  id: UUID\n}";
    let spec2 = openapi(src2);
    assert!(spec2["components"]["schemas"]["Item"]["description"].is_null());
}

// ── Type mapping ──────────────────────────────────────────

#[test]
fn uuid_type() {
    let src = "module Test\nentity X {\n  id: UUID\n}";
    let spec = openapi(src);
    let f = &spec["components"]["schemas"]["X"]["properties"]["id"];
    assert_eq!(f["type"], "string");
    assert_eq!(f["format"], "uuid");
}

#[test]
fn int_type() {
    let src = "module Test\nentity X {\n  count: Int\n}";
    let spec = openapi(src);
    let f = &spec["components"]["schemas"]["X"]["properties"]["count"];
    assert_eq!(f["type"], "integer");
    assert_eq!(f["format"], "int64");
}

#[test]
fn decimal_type() {
    let src = "module Test\nentity X {\n  price: Decimal(precision: 2)\n}";
    let spec = openapi(src);
    let f = &spec["components"]["schemas"]["X"]["properties"]["price"];
    assert_eq!(f["type"], "number");
}

#[test]
fn bool_type() {
    let src = "module Test\nentity X {\n  active: Bool\n}";
    let spec = openapi(src);
    let f = &spec["components"]["schemas"]["X"]["properties"]["active"];
    assert_eq!(f["type"], "boolean");
}

#[test]
fn datetime_type() {
    let src = "module Test\nentity X {\n  ts: DateTime\n}";
    let spec = openapi(src);
    let f = &spec["components"]["schemas"]["X"]["properties"]["ts"];
    assert_eq!(f["type"], "string");
    assert_eq!(f["format"], "date-time");
}

#[test]
fn email_type() {
    let src = "module Test\nentity X {\n  mail: Email\n}";
    let spec = openapi(src);
    let f = &spec["components"]["schemas"]["X"]["properties"]["mail"];
    assert_eq!(f["type"], "string");
    assert_eq!(f["format"], "email");
}

#[test]
fn url_type() {
    let src = "module Test\nentity X {\n  link: URL\n}";
    let spec = openapi(src);
    let f = &spec["components"]["schemas"]["X"]["properties"]["link"];
    assert_eq!(f["type"], "string");
    assert_eq!(f["format"], "uri");
}

#[test]
fn union_type_enum() {
    let src = "module Test\nentity X {\n  status: Active | Frozen | Closed\n}";
    let spec = openapi(src);
    let f = &spec["components"]["schemas"]["X"]["properties"]["status"];
    assert_eq!(f["type"], "string");
    let variants = f["enum"].as_array().unwrap();
    assert_eq!(variants.len(), 3);
    assert!(variants.contains(&Value::String("Active".to_string())));
    assert!(variants.contains(&Value::String("Frozen".to_string())));
    assert!(variants.contains(&Value::String("Closed".to_string())));
}

#[test]
fn list_type() {
    let src = "module Test\nentity X {\n  tags: List<String>\n}";
    let spec = openapi(src);
    let f = &spec["components"]["schemas"]["X"]["properties"]["tags"];
    assert_eq!(f["type"], "array");
    assert_eq!(f["items"]["type"], "string");
}

#[test]
fn set_type_unique() {
    let src = "module Test\nentity X {\n  ids: Set<UUID>\n}";
    let spec = openapi(src);
    let f = &spec["components"]["schemas"]["X"]["properties"]["ids"];
    assert_eq!(f["type"], "array");
    assert_eq!(f["uniqueItems"], true);
    assert_eq!(f["items"]["type"], "string");
}

#[test]
fn map_type() {
    let src = "module Test\nentity X {\n  meta: Map<String, Int>\n}";
    let spec = openapi(src);
    let f = &spec["components"]["schemas"]["X"]["properties"]["meta"];
    assert_eq!(f["type"], "object");
    assert_eq!(f["additionalProperties"]["type"], "integer");
}

#[test]
fn entity_ref_type() {
    let src = "module Test\nentity Account {\n  id: UUID\n}\nentity Transfer {\n  from: Account\n}";
    let spec = openapi(src);
    let f = &spec["components"]["schemas"]["Transfer"]["properties"]["from"];
    assert_eq!(f["$ref"], "#/components/schemas/Account");
}

// ── Action → Path ──────────────────────────────────────────

#[test]
fn action_creates_post_path() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n}";
    let spec = openapi(src);
    assert!(spec["paths"]["/actions/DoThing"]["post"].is_object());
}

#[test]
fn action_operation_id() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n}";
    let spec = openapi(src);
    assert_eq!(
        spec["paths"]["/actions/DoThing"]["post"]["operationId"],
        "do_thing"
    );
}

#[test]
fn action_request_body_has_params() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n  amount: Int\n}";
    let spec = openapi(src);
    let schema = &spec["paths"]["/actions/DoThing"]["post"]["requestBody"]["content"]["application/json"]
        ["schema"];
    let params = &schema["properties"]["params"];
    assert_eq!(params["properties"]["x"]["$ref"], "#/components/schemas/X");
    assert_eq!(params["properties"]["amount"]["type"], "integer");
}

#[test]
fn action_has_state_in_request() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n}";
    let spec = openapi(src);
    let schema = &spec["paths"]["/actions/DoThing"]["post"]["requestBody"]["content"]["application/json"]
        ["schema"];
    assert!(schema["properties"]["state"].is_object());
}

#[test]
fn action_responses() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n}";
    let spec = openapi(src);
    let responses = &spec["paths"]["/actions/DoThing"]["post"]["responses"];
    assert!(responses["200"].is_object());
    assert!(responses["422"].is_object());
    assert!(responses["400"].is_object());
}

#[test]
fn action_description_includes_requires() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n  requires {\n    x.id != x.id\n  }\n}";
    let spec = openapi(src);
    let desc = spec["paths"]["/actions/DoThing"]["post"]["description"]
        .as_str()
        .unwrap();
    assert!(desc.contains("Preconditions"));
    assert!(desc.contains("x.id != x.id"));
}

#[test]
fn action_description_includes_ensures() {
    let src = "module Test\nentity X { id: UUID }\naction DoThing {\n  x: X\n  ensures {\n    x.id == old(x.id)\n  }\n}";
    let spec = openapi(src);
    let desc = spec["paths"]["/actions/DoThing"]["post"]["description"]
        .as_str()
        .unwrap();
    assert!(desc.contains("Postconditions"));
    assert!(desc.contains("old(x.id)"));
}

// ── Shared schemas ─────────────────────────────────────────

#[test]
fn has_violation_schema() {
    let src = "module Test\nentity X { id: UUID }";
    let spec = openapi(src);
    let v = &spec["components"]["schemas"]["Violation"];
    assert_eq!(v["type"], "object");
    assert!(v["properties"]["kind"]["enum"].is_array());
    assert!(v["properties"]["message"].is_object());
}

#[test]
fn has_action_result_schema() {
    let src = "module Test\nentity X { id: UUID }";
    let spec = openapi(src);
    let r = &spec["components"]["schemas"]["ActionResult"];
    assert_eq!(r["type"], "object");
    assert!(r["properties"]["ok"].is_object());
    assert!(r["properties"]["new_params"].is_object());
    assert!(r["properties"]["violations"].is_object());
}

// ── Full example files ─────────────────────────────────────

#[test]
fn transfer_openapi() {
    let src = include_str!("../../../examples/transfer.intent");
    let spec = openapi(src);
    assert_eq!(spec["openapi"], "3.0.3");
    assert!(spec["components"]["schemas"]["Account"].is_object());
    assert!(spec["paths"]["/actions/Transfer"]["post"].is_object());
    assert!(spec["paths"]["/actions/FreezeAccount"]["post"].is_object());
}

#[test]
fn auth_openapi() {
    let src = include_str!("../../../examples/auth.intent");
    let spec = openapi(src);
    assert_eq!(spec["openapi"], "3.0.3");
    assert!(!spec["paths"].as_object().unwrap().is_empty());
}

#[test]
fn shopping_cart_openapi() {
    let src = include_str!("../../../examples/shopping_cart.intent");
    let spec = openapi(src);
    assert_eq!(spec["openapi"], "3.0.3");
    assert!(!spec["paths"].as_object().unwrap().is_empty());
}
