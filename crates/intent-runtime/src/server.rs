use intent_ir::Module;
use serde::Serialize;
use serde_json::json;
use tiny_http::{Header, Method, Response, Server};

use crate::contract::{ActionRequest, execute_action};

/// Serve a compiled module as a REST API.
///
/// Endpoints:
/// - `GET  /`                  — module info (name, actions, entities)
/// - `POST /actions/{name}`    — execute an action
pub fn serve(module: Module, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::http(addr).map_err(|e| format!("failed to bind {addr}: {e}"))?;
    eprintln!("intent serve: listening on http://{addr}");
    eprintln!("  module: {}", module.name);
    for func in &module.functions {
        eprintln!("  POST /actions/{}", func.name);
    }
    eprintln!("  GET  /");

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().clone();

        let (status, body) = match (method, url.as_str()) {
            (Method::Get, "/") => {
                let info = module_info(&module);
                (200, serde_json::to_string_pretty(&info).unwrap())
            }
            (Method::Post, path) if path.starts_with("/actions/") => {
                let action_name = &path["/actions/".len()..];
                handle_action(&module, action_name, &mut request)
            }
            _ => (404, json!({"error": "not found"}).to_string()),
        };

        let content_type = Header::from_bytes("Content-Type", "application/json").unwrap();
        let response = Response::from_string(body)
            .with_status_code(status)
            .with_header(content_type);
        request.respond(response).ok();
    }
    Ok(())
}

fn handle_action(
    module: &Module,
    action_name: &str,
    request: &mut tiny_http::Request,
) -> (i32, String) {
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return (
            400,
            json!({"error": "failed to read request body"}).to_string(),
        );
    }

    let action_request: ActionRequest = match serde_json::from_str::<ActionRequest>(&body) {
        Ok(mut req) => {
            req.action = action_name.to_string();
            req
        }
        Err(e) => {
            return (
                400,
                json!({"error": format!("invalid JSON: {e}")}).to_string(),
            );
        }
    };

    match execute_action(module, &action_request) {
        Ok(result) => {
            let status = if result.ok { 200 } else { 422 };
            (status, serde_json::to_string_pretty(&result).unwrap())
        }
        Err(e) => (400, json!({"error": format!("{e}")}).to_string()),
    }
}

#[derive(Serialize)]
struct ModuleInfo {
    name: String,
    entities: Vec<EntityInfo>,
    actions: Vec<ActionInfo>,
    invariants: Vec<String>,
}

#[derive(Serialize)]
struct EntityInfo {
    name: String,
    fields: Vec<FieldInfo>,
}

#[derive(Serialize)]
struct FieldInfo {
    name: String,
    #[serde(rename = "type")]
    ty: String,
}

#[derive(Serialize)]
struct ActionInfo {
    name: String,
    params: Vec<FieldInfo>,
    precondition_count: usize,
    postcondition_count: usize,
}

fn module_info(module: &Module) -> ModuleInfo {
    ModuleInfo {
        name: module.name.clone(),
        entities: module
            .structs
            .iter()
            .map(|s| EntityInfo {
                name: s.name.clone(),
                fields: s
                    .fields
                    .iter()
                    .map(|f| FieldInfo {
                        name: f.name.clone(),
                        ty: format!("{:?}", f.ty),
                    })
                    .collect(),
            })
            .collect(),
        actions: module
            .functions
            .iter()
            .map(|f| ActionInfo {
                name: f.name.clone(),
                params: f
                    .params
                    .iter()
                    .map(|p| FieldInfo {
                        name: p.name.clone(),
                        ty: format!("{:?}", p.ty),
                    })
                    .collect(),
                precondition_count: f.preconditions.len(),
                postcondition_count: f.postconditions.len(),
            })
            .collect(),
        invariants: module.invariants.iter().map(|i| i.name.clone()).collect(),
    }
}
