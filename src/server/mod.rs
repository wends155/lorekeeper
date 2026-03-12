//! MCP server handler implementation.

use crate::model::entry::{NewEntry, UpdateEntry};
use crate::model::types::EntryType;
use crate::store::repository::{EntryRepository, Filters, SearchQuery};
use async_trait::async_trait;
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::mcp_server::ServerHandlerCore;
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, ListToolsResult, NotificationFromClient,
    RequestFromClient, ResultFromServer, RpcError, TextContent, Tool, ToolInputSchema,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

/// The primary MCP server handler for Lorekeeper tools.
pub struct LoreHandler {
    repo: Arc<dyn EntryRepository>,
}

impl std::fmt::Debug for LoreHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoreHandler").finish_non_exhaustive()
    }
}

impl LoreHandler {
    /// Creates a new `LoreHandler` with the given repository.
    pub fn new(repo: Arc<dyn EntryRepository>) -> Self {
        Self { repo }
    }

    fn make_tool(
        name: &str,
        description: &str,
        props: HashMap<String, Value>,
        required: Vec<String>,
    ) -> Tool {
        let properties = if props.is_empty() {
            None
        } else {
            let mut map = HashMap::new();
            for (k, v) in props {
                map.insert(k, serde_json::from_value(v).unwrap_or_default());
            }
            Some(map)
        };

        Tool {
            name: name.into(),
            description: Some(description.into()),
            input_schema: ToolInputSchema::new(required, properties, None),
            annotations: None,
            execution: None,
            icons: vec![],
            meta: None,
            output_schema: None,
            title: None,
        }
    }

    fn get_tools() -> Vec<Tool> {
        vec![
            Self::make_tool(
                "lore_store",
                "Store a new memory entry (Decision, Commit, Constraint, etc.)",
                HashMap::from([
                    ("entry_type".into(), json!({"type": "string"})),
                    ("title".into(), json!({"type": "string"})),
                    ("body".into(), json!({"type": "string"})),
                    ("role".into(), json!({"type": "string"})),
                    ("tags".into(), json!({"type": "array", "items": {"type": "string"}})),
                    (
                        "related_entries".into(),
                        json!({"type": "array", "items": {"type": "string"}}),
                    ),
                    ("data".into(), json!({"type": "object"})),
                ]),
                vec!["entry_type".into(), "title".into(), "role".into()],
            ),
            Self::make_tool(
                "lore_search",
                "Search entries using FTS5 (title, body, tags)",
                HashMap::from([
                    ("query".into(), json!({"type": "string"})),
                    ("entry_type".into(), json!({"type": "string"})),
                    ("limit".into(), json!({"type": "integer", "default": 20})),
                ]),
                vec!["query".into()],
            ),
            Self::make_tool(
                "lore_get",
                "Retrieve a specific entry by ID",
                HashMap::from([("id".into(), json!({"type": "string"}))]),
                vec!["id".into()],
            ),
            Self::make_tool(
                "lore_update",
                "Update an existing entry",
                HashMap::from([
                    ("id".into(), json!({"type": "string"})),
                    ("title".into(), json!({"type": "string"})),
                    ("body".into(), json!({"type": "string"})),
                    ("tags".into(), json!({"type": "array", "items": {"type": "string"}})),
                    (
                        "related_entries".into(),
                        json!({"type": "array", "items": {"type": "string"}}),
                    ),
                    ("data".into(), json!({"type": "object"})),
                ]),
                vec!["id".into()],
            ),
            Self::make_tool(
                "lore_delete",
                "Soft-delete an entry",
                HashMap::from([("id".into(), json!({"type": "string"}))]),
                vec!["id".into()],
            ),
            Self::make_tool(
                "lore_recent",
                "List recent entries",
                HashMap::from([("limit".into(), json!({"type": "integer", "default": 10}))]),
                vec![],
            ),
            Self::make_tool(
                "lore_by_type",
                "List entries by type with pagination",
                HashMap::from([
                    ("entry_type".into(), json!({"type": "string"})),
                    ("limit".into(), json!({"type": "integer", "default": 20})),
                    ("offset".into(), json!({"type": "integer", "default": 0})),
                ]),
                vec!["entry_type".into()],
            ),
            Self::make_tool("lore_stats", "Get memory statistics", HashMap::new(), vec![]),
        ]
    }

    fn handle_tool_call(&self, params: CallToolRequestParams) -> Result<Value, String> {
        let args = params.arguments.unwrap_or_default();
        let args_value = Value::Object(args.clone());

        match params.name.as_str() {
            "lore_store" => {
                let input: NewEntry =
                    serde_json::from_value(args_value).map_err(|e| e.to_string())?;
                let entry = self.repo.store(input).map_err(|e| e.to_string())?;
                Ok(json!({ "status": "success", "id": entry.id.0 }))
            }
            "lore_search" => {
                let query: SearchQuery =
                    serde_json::from_value(args_value).map_err(|e| e.to_string())?;
                let entries = self.repo.search(&query).map_err(|e| e.to_string())?;
                serde_json::to_value(entries).map_err(|e| e.to_string())
            }
            "lore_get" => {
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;
                let entry = self.repo.get(id).map_err(|e| e.to_string())?;
                serde_json::to_value(entry).map_err(|e| e.to_string())
            }
            "lore_update" => {
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;
                let update: UpdateEntry =
                    serde_json::from_value(args_value).map_err(|e| e.to_string())?;
                let entry = self.repo.update(id, update).map_err(|e| e.to_string())?;
                serde_json::to_value(entry).map_err(|e| e.to_string())
            }
            "lore_delete" => {
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;
                self.repo.delete(id).map_err(|e| e.to_string())?;
                Ok(json!({ "status": "success" }))
            }
            "lore_recent" => {
                let limit = u32::try_from(
                    args.get("limit").and_then(serde_json::Value::as_u64).unwrap_or(10),
                )
                .unwrap_or(10);
                let entries = self.repo.recent(limit).map_err(|e| e.to_string())?;
                serde_json::to_value(entries).map_err(|e| e.to_string())
            }
            "lore_by_type" => {
                let et_str =
                    args.get("entry_type").and_then(|v| v.as_str()).ok_or("missing entry_type")?;
                let et: EntryType =
                    serde_json::from_value(json!(et_str)).map_err(|e| e.to_string())?;
                let filters: Filters =
                    serde_json::from_value(args_value).map_err(|e| e.to_string())?;
                let entries = self.repo.by_type(et, &filters).map_err(|e| e.to_string())?;
                serde_json::to_value(entries).map_err(|e| e.to_string())
            }
            "lore_stats" => {
                let stats = self.repo.stats().map_err(|e| e.to_string())?;
                serde_json::to_value(stats).map_err(|e| e.to_string())
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }

    fn make_text_result(text: String, is_error: bool) -> CallToolResult {
        CallToolResult {
            content: vec![TextContent::new(text, None, None).into()],
            is_error: if is_error { Some(true) } else { None },
            meta: None,
            structured_content: None,
        }
    }
}

#[async_trait]
impl ServerHandlerCore for LoreHandler {
    async fn handle_request(
        &self,
        request: RequestFromClient,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ResultFromServer, RpcError> {
        match request {
            RequestFromClient::ListToolsRequest(_) => {
                Ok(ResultFromServer::ListToolsResult(ListToolsResult {
                    tools: Self::get_tools(),
                    next_cursor: None,
                    meta: None,
                }))
            }
            RequestFromClient::CallToolRequest(params) => {
                let result = match self.handle_tool_call(params) {
                    Ok(val) => {
                        let text = serde_json::to_string_pretty(&val).unwrap_or_default();
                        Self::make_text_result(text, false)
                    }
                    Err(err_msg) => Self::make_text_result(err_msg, true),
                };
                Ok(ResultFromServer::CallToolResult(result))
            }
            _ => Err(RpcError::method_not_found()),
        }
    }

    async fn handle_notification(
        &self,
        _notification: NotificationFromClient,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<(), RpcError> {
        Ok(())
    }

    async fn handle_error(
        &self,
        _error: &RpcError,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<(), RpcError> {
        Ok(())
    }
}
