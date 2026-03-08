//! OpenAPI 3.0 spec generator for IntentLang specifications.
//!
//! Generates an OpenAPI 3.0.3 JSON document from a parsed `.intent` AST.
//! The output matches the REST API served by `intent serve`:
//! - Each action becomes `POST /actions/{ActionName}`
//! - Each entity becomes a JSON Schema component
//! - Contracts (requires/ensures/invariants) are documented in descriptions

use intent_parser::ast;
use serde_json::{Map, Value, json};

use crate::{doc_text, format_ensures_item, format_expr, to_snake_case};

/// Generate an OpenAPI 3.0.3 spec from a parsed intent file.
pub fn generate(file: &ast::File) -> Value {
    let mut schemas = Map::new();
    let mut paths = Map::new();

    // Collect entity names for $ref resolution
    let entity_names: Vec<String> = file
        .items
        .iter()
        .filter_map(|item| match item {
            ast::TopLevelItem::Entity(e) => Some(e.name.clone()),
            _ => None,
        })
        .collect();

    // Collect invariant descriptions for action documentation
    let invariants: Vec<&ast::InvariantDecl> = file
        .items
        .iter()
        .filter_map(|item| match item {
            ast::TopLevelItem::Invariant(inv) => Some(inv),
            _ => None,
        })
        .collect();

    for item in &file.items {
        match item {
            ast::TopLevelItem::Entity(entity) => {
                let schema = entity_to_schema(entity, &entity_names);
                schemas.insert(entity.name.clone(), schema);
            }
            ast::TopLevelItem::Action(action) => {
                let path = action_to_path(action, &entity_names, &invariants);
                let route = format!("/actions/{}", action.name);
                paths.insert(route, path);
            }
            _ => {}
        }
    }

    // Add shared schemas: Violation, ActionResult
    schemas.insert("Violation".to_string(), violation_schema());
    schemas.insert("ActionResult".to_string(), action_result_schema());

    let mut info = Map::new();
    info.insert(
        "title".to_string(),
        json!(format!("{} API", file.module.name)),
    );
    info.insert("version".to_string(), json!("0.1.0"));
    if let Some(doc) = &file.doc {
        info.insert("description".to_string(), json!(doc_text(doc)));
    }

    json!({
        "openapi": "3.0.3",
        "info": Value::Object(info),
        "paths": Value::Object(paths),
        "components": {
            "schemas": Value::Object(schemas)
        }
    })
}

// ── Entity → JSON Schema ──────────────────────────────────────

fn entity_to_schema(entity: &ast::EntityDecl, entity_names: &[String]) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for field in &entity.fields {
        let schema = type_to_schema(&field.ty, entity_names);
        properties.insert(field.name.clone(), schema);
        if !field.ty.optional {
            required.push(json!(field.name));
        }
    }

    let mut schema = Map::new();
    schema.insert("type".to_string(), json!("object"));
    schema.insert("properties".to_string(), Value::Object(properties));
    if !required.is_empty() {
        schema.insert("required".to_string(), Value::Array(required));
    }
    if let Some(doc) = &entity.doc {
        schema.insert("description".to_string(), json!(doc_text(doc)));
    }

    Value::Object(schema)
}

// ── Type → JSON Schema ───────────────────────────────────────

fn type_to_schema(ty: &ast::TypeExpr, entity_names: &[String]) -> Value {
    let base = type_kind_to_schema(&ty.ty, entity_names);
    if ty.optional {
        // OpenAPI 3.0: nullable flag
        if let Value::Object(mut obj) = base {
            obj.insert("nullable".to_string(), json!(true));
            Value::Object(obj)
        } else {
            base
        }
    } else {
        base
    }
}

fn type_kind_to_schema(kind: &ast::TypeKind, entity_names: &[String]) -> Value {
    match kind {
        ast::TypeKind::Simple(name) => simple_type_schema(name, entity_names),
        ast::TypeKind::Parameterized { name, .. } => simple_type_schema(name, entity_names),
        ast::TypeKind::Union(variants) => {
            let names: Vec<&str> = variants
                .iter()
                .filter_map(|v| match v {
                    ast::TypeKind::Simple(n) => Some(n.as_str()),
                    _ => None,
                })
                .collect();
            json!({
                "type": "string",
                "enum": names
            })
        }
        ast::TypeKind::List(inner) => {
            json!({
                "type": "array",
                "items": type_to_schema(inner, entity_names)
            })
        }
        ast::TypeKind::Set(inner) => {
            json!({
                "type": "array",
                "items": type_to_schema(inner, entity_names),
                "uniqueItems": true
            })
        }
        ast::TypeKind::Map(_k, v) => {
            json!({
                "type": "object",
                "additionalProperties": type_to_schema(v, entity_names)
            })
        }
    }
}

fn simple_type_schema(name: &str, entity_names: &[String]) -> Value {
    match name {
        "UUID" => json!({ "type": "string", "format": "uuid" }),
        "String" => json!({ "type": "string" }),
        "Int" => json!({ "type": "integer", "format": "int64" }),
        "Decimal" => json!({ "type": "number" }),
        "Bool" => json!({ "type": "boolean" }),
        "DateTime" => json!({ "type": "string", "format": "date-time" }),
        "Email" => json!({ "type": "string", "format": "email" }),
        "URL" => json!({ "type": "string", "format": "uri" }),
        "CurrencyCode" => json!({ "type": "string" }),
        other => {
            if entity_names.contains(&other.to_string()) {
                json!({ "$ref": format!("#/components/schemas/{other}") })
            } else {
                json!({ "type": "string" })
            }
        }
    }
}

// ── Action → Path Item ──────────────────────────────────────

fn action_to_path(
    action: &ast::ActionDecl,
    entity_names: &[String],
    invariants: &[&ast::InvariantDecl],
) -> Value {
    let mut description_parts = Vec::new();

    if let Some(doc) = &action.doc {
        description_parts.push(doc_text(doc));
    }

    // Document preconditions
    if let Some(req) = &action.requires {
        description_parts.push("**Preconditions (requires):**".to_string());
        for cond in &req.conditions {
            description_parts.push(format!("- `{}`", format_expr(cond)));
        }
    }

    // Document postconditions
    if let Some(ens) = &action.ensures {
        description_parts.push("**Postconditions (ensures):**".to_string());
        for item in &ens.items {
            description_parts.push(format!("- `{}`", format_ensures_item(item)));
        }
    }

    // Document properties
    if let Some(props) = &action.properties {
        description_parts.push("**Properties:**".to_string());
        for entry in &props.entries {
            description_parts.push(format!(
                "- {}: {}",
                entry.key,
                crate::format_prop_value(&entry.value)
            ));
        }
    }

    // Document relevant invariants
    if !invariants.is_empty() {
        description_parts.push("**Invariants:**".to_string());
        for inv in invariants {
            let mut line = format!("- {}", inv.name);
            if let Some(doc) = &inv.doc {
                line.push_str(&format!(": {}", doc_text(doc)));
            }
            description_parts.push(line);
        }
    }

    let description = description_parts.join("\n\n");

    // Build request body schema
    let request_schema = action_request_schema(action, entity_names);

    let fn_name = to_snake_case(&action.name);

    json!({
        "post": {
            "operationId": fn_name,
            "summary": format!("Execute the {} action", action.name),
            "description": description,
            "requestBody": {
                "required": true,
                "content": {
                    "application/json": {
                        "schema": request_schema
                    }
                }
            },
            "responses": {
                "200": {
                    "description": "Action executed successfully, all contracts satisfied",
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/ActionResult" }
                        }
                    }
                },
                "422": {
                    "description": "Contract violation (precondition, postcondition, or invariant failed)",
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/ActionResult" }
                        }
                    }
                },
                "400": {
                    "description": "Malformed request or runtime error"
                }
            }
        }
    })
}

fn action_request_schema(action: &ast::ActionDecl, entity_names: &[String]) -> Value {
    let mut param_properties = Map::new();
    let mut param_required = Vec::new();

    for param in &action.params {
        let schema = type_to_schema(&param.ty, entity_names);
        param_properties.insert(param.name.clone(), schema);
        if !param.ty.optional {
            param_required.push(json!(param.name));
        }
    }

    let mut params_schema = Map::new();
    params_schema.insert("type".to_string(), json!("object"));
    params_schema.insert("properties".to_string(), Value::Object(param_properties));
    if !param_required.is_empty() {
        params_schema.insert("required".to_string(), Value::Array(param_required));
    }

    json!({
        "type": "object",
        "required": ["params"],
        "properties": {
            "params": Value::Object(params_schema),
            "state": {
                "type": "object",
                "description": "Entity instances keyed by type name (for quantifier/invariant evaluation)",
                "additionalProperties": {
                    "type": "array",
                    "items": {}
                }
            }
        }
    })
}

// ── Shared schemas ─────────────────────────────────────────

fn violation_schema() -> Value {
    json!({
        "type": "object",
        "required": ["kind", "message"],
        "properties": {
            "kind": {
                "type": "string",
                "enum": ["precondition_failed", "postcondition_failed", "invariant_violated", "edge_guard_triggered"]
            },
            "message": {
                "type": "string"
            }
        }
    })
}

fn action_result_schema() -> Value {
    json!({
        "type": "object",
        "required": ["ok", "new_params", "violations"],
        "properties": {
            "ok": {
                "type": "boolean",
                "description": "Whether all contracts were satisfied"
            },
            "new_params": {
                "type": "object",
                "description": "Updated parameter values after postcondition application",
                "additionalProperties": {}
            },
            "violations": {
                "type": "array",
                "items": {
                    "$ref": "#/components/schemas/Violation"
                },
                "description": "List of contract violations (empty if ok is true)"
            }
        }
    })
}
