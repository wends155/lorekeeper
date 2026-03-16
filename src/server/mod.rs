//! MCP server handler implementation.
//!
//! This module wires up the [`LoreHandler`] as the MCP server's request handler,
//! registering all Lorekeeper tools and dispatching incoming tool calls to the
//! appropriate repository methods.

use crate::config::LoreConfig;
use crate::model::entry::{NewEntry, UpdateEntry};
use crate::model::types::{EntryType, ReflectCriteria};
use crate::render::render_entries;
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
use tracing::{error, info, warn};

// ---- Tool description strings -----------------------------------------------
// Stored as named `&str` constants so rustfmt does not reformat the interior of
// json!() macro calls, which would break the HashMap::from([...]) structure.

const DESC_STORE: &str = "Store a new memory entry in the Lorekeeper database.\n\n\
    WHEN TO USE:\n\
    - After making an architectural decision -> type DECISION\n\
    - After completing a git commit -> type COMMIT\n\
    - When discovering a project constraint -> type CONSTRAINT\n\
    - When learning from a bug or incident -> type LESSON\n\
    - When creating an implementation plan -> type PLAN\n\
    - When defining a new feature -> type FEATURE\n\
    - When registering a stub for future work -> type STUB\n\
    - When deferring work to a later phase -> type DEFERRED\n\
    - When recording implementation observations -> type BUILDER_NOTE\n\
    - When flagging technical debt -> type TECH_DEBT\n\n\
    ROLE ENFORCEMENT:\n\
    - Architect-only: DECISION, CONSTRAINT, LESSON, PLAN, FEATURE\n\
    - Builder-only: COMMIT, STUB, BUILDER_NOTE\n\
    - Both roles: DEFERRED, TECH_DEBT\n\n\
    DATA SCHEMA (required fields by type):\n\
    - COMMIT: { hash: <git-hash>, files: [path/to/file] }\n\
    - CONSTRAINT: { source: <origin> }\n\
    - LESSON: { root_cause: <explanation> }\n\
    - PLAN: { scope: <area>, tier: S|M|L, status: planned }\n\
    - FEATURE: { status: <status> }\n\
    - STUB: { phase_number: N, contract: <desc>, module: <mod>, status: open }\n\
    - DEFERRED: { reason: <why>, target_phase: N }\n\
    - BUILDER_NOTE: { note_type: <type>, step_ref: <step>, plan_ref: <id> }\n\
    - TECH_DEBT: { severity: low|medium|high, origin_phase: N }\n\
    - DECISION: no data object required\n\n\
    RETURNS: { status: success, id: <uuid> }";

const DESC_UPDATE: &str = "Update fields on an existing memory entry. Only provided fields change; \
    omitted fields keep their current values.\n\n\
    WHEN TO USE:\n\
    - To mark a PLAN as executed or abandoned\n\
    - To resolve a STUB after implementing it\n\
    - To add tags or related entries to an existing record\n\
    - To correct or enrich the body of a previous entry\n\n\
    STATE TRANSITIONS (enforced server-side):\n\
    - PLAN status: planned -> executed | planned -> abandoned (no revert)\n\
    - STUB status: open -> resolved (no revert)\n\n\
    RETURNS: Full updated Entry JSON object.";

const DESC_DELETE: &str = "Soft-delete a memory entry. The entry is hidden from searches but preserved in the database.\n\n\
    WHEN TO USE:\n\
    - When an entry was created in error\n\
    - When a decision has been superseded (prefer updating over deleting)\n\n\
    RETURNS: { status: success }";

const DESC_RENDER: &str = "Render all memory entries as a formatted Markdown document, grouped by type.\n\n\
    WHEN TO USE:\n\
    - When the user asks for a full memory dump\n\
    - When generating a human-readable summary of all stored knowledge\n\
    - For periodic review of the complete memory bank\n\n\
    RETURNS: Markdown string with entries grouped by type, sorted chronologically.";

const DESC_SEARCH: &str = "Search memory entries using full-text search across titles, bodies, and tags.\n\n\
    WHEN TO USE:\n\
    - At session start to recall past decisions, constraints, or lessons\n\
    - Before making a decision, to check if a similar one already exists\n\
    - When you need context about a specific topic or keyword\n\
    - When starting a new task, to find relevant constraints and prior art\n\n\
    RETURNS: JSON array of matching Entry objects, ranked by relevance.";

const DESC_GET: &str = "Retrieve a specific memory entry by its UUID.\n\n\
    WHEN TO USE:\n\
    - When you have an entry ID from a search result or related_entries reference\n\
    - To get the full details of a specific decision, plan, or constraint\n\n\
    RETURNS: Full Entry JSON object with all fields.";

const DESC_RECENT: &str = "List the most recently created memory entries across all types.\n\n\
    WHEN TO USE:\n\
    - At session start to get a quick overview of recent activity\n\
    - To understand what was done in the last session\n\
    - When you need broad context without a specific search query\n\n\
    RETURNS: JSON array of Entry objects, newest first.";

const DESC_BY_TYPE: &str = "List memory entries filtered by type, with optional pagination.\n\n\
    WHEN TO USE:\n\
    - To review all decisions made in the project\n\
    - To list all open stubs that need implementation\n\
    - To find all constraints before starting a new feature\n\
    - To audit technical debt items\n\n\
    RETURNS: JSON array of Entry objects matching the type, ordered newest first.";

const DESC_STATS: &str = "Get aggregate statistics about the memory bank.\n\n\
    WHEN TO USE:\n\
    - At session start to understand the current state of the memory bank\n\
    - To check how many entries of each type exist\n\
    - To see when the last update was made\n\n\
    RETURNS: JSON object with total count, by-type breakdown, and last_updated timestamp.";

const DESC_HELP: &str = "Get contextual help about Lorekeeper tools, entry types, and workflows.\n\n\
    WHEN TO USE:\n\
    - When unsure which tool to use for a given situation\n\
    - When you need the required data schema for a specific entry type\n\
    - When you want to understand role enforcement rules\n\
    - At the start of a session to review the workflow\n\n\
    RETURNS: Markdown help text for the requested topic.";

mod help;

/// The primary MCP server handler for Lorekeeper tools.
pub struct LoreHandler {
    repo: Arc<dyn EntryRepository>,
    config: LoreConfig,
}

impl std::fmt::Debug for LoreHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoreHandler").finish_non_exhaustive()
    }
}

impl LoreHandler {
    /// Creates a new `LoreHandler` with the given repository and project config.
    ///
    /// # Arguments
    ///
    /// * `repo` - A thread-safe reference to an [`EntryRepository`] implementation.
    /// * `config` - The project-local [`LoreConfig`] loaded from `.lorekeeper/config.toml`.
    pub fn new(repo: Arc<dyn EntryRepository>, config: LoreConfig) -> Self {
        Self { repo, config }
    }

    /// Creates a `LoreHandler` with default config. Intended for tests and toolcheck scenarios.
    #[cfg(test)]
    pub fn with_defaults(repo: Arc<dyn EntryRepository>) -> Self {
        Self { repo, config: LoreConfig::default() }
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

    #[rustfmt::skip]
    #[allow(clippy::too_many_lines)]
    fn get_tools() -> Vec<Tool> {
        let entry_type_enum = json!({
            "type": "string",
            "description": "The category of memory entry.",
            "enum": ["DECISION","COMMIT","CONSTRAINT","LESSON","PLAN",
                     "FEATURE","STUB","DEFERRED","BUILDER_NOTE","TECH_DEBT"]
        });
        let role_enum = json!({
            "type": "string",
            "description": "Your current TARS role. Must match role restrictions for entry type.",
            "enum": ["architect", "builder"]
        });
        let id_field = json!({
            "type": "string",
            "format": "uuid",
            "description": "UUID of the target entry."
        });
        let tags_field = json!({
            "type": "array",
            "items": {"type": "string"},
            "description": "Lowercase keyword tags for searchability."
        });
        let related_field = json!({
            "type": "array",
            "items": {"type": "string", "format": "uuid"},
            "description": "UUIDs of related memory entries."
        });
        let data_field = json!({
            "type": "object",
            "description": "Type-specific metadata. See tool description for schema per type."
        });
        let limit_field = json!({"type": "integer", "description": "Maximum results to return.", "default": 20});
        let offset_field = json!({"type": "integer", "description": "Number of entries to skip for pagination.", "default": 0});

        vec![
            // -- WRITE TOOLS --------------------------------------------------
            Self::make_tool(
                "lorekeeper_store",
                DESC_STORE,
                HashMap::from([
                    ("entry_type".into(), entry_type_enum.clone()),
                    ("title".into(), json!({"type":"string","description":"A brief one-line summary. Required, non-empty."})),
                    ("body".into(), json!({"type":"string","description":"Extended description or reasoning. Optional."})),
                    ("role".into(), role_enum),
                    ("tags".into(), tags_field.clone()),
                    ("related_entries".into(), related_field.clone()),
                    ("data".into(), data_field.clone()),
                ]),
                vec!["entry_type".into(), "title".into(), "role".into()],
            ),
            Self::make_tool(
                "lorekeeper_update",
                DESC_UPDATE,
                HashMap::from([
                    ("id".into(), id_field.clone()),
                    ("title".into(), json!({"type":"string","description":"New title (optional)."})),
                    ("body".into(), json!({"type":"string","description":"New body (optional)."})),
                    ("tags".into(), tags_field),
                    ("related_entries".into(), related_field),
                    ("data".into(), data_field),
                ]),
                vec!["id".into()],
            ),
            Self::make_tool(
                "lorekeeper_delete",
                DESC_DELETE,
                HashMap::from([("id".into(), id_field.clone())]),
                vec!["id".into()],
            ),
            Self::make_tool("lorekeeper_render", DESC_RENDER, HashMap::new(), vec![]),
            // -- READ TOOLS ---------------------------------------------------
            Self::make_tool(
                "lorekeeper_search",
                DESC_SEARCH,
                HashMap::from([
                    ("query".into(), json!({"type":"string","description":"Search keywords. FTS5 full-text search syntax."})),
                    ("entry_type".into(), entry_type_enum.clone()),
                    ("limit".into(), limit_field.clone()),
                ]),
                vec!["query".into()],
            ),
            Self::make_tool(
                "lorekeeper_get",
                DESC_GET,
                HashMap::from([("id".into(), id_field)]),
                vec!["id".into()],
            ),
            Self::make_tool(
                "lorekeeper_recent",
                DESC_RECENT,
                HashMap::from([("limit".into(), json!({"type":"integer","description":"Number of entries to return.","default":10}))]),
                vec![],
            ),
            Self::make_tool(
                "lorekeeper_by_type",
                DESC_BY_TYPE,
                HashMap::from([
                    ("entry_type".into(), entry_type_enum),
                    ("limit".into(), limit_field),
                    ("offset".into(), offset_field),
                ]),
                vec!["entry_type".into()],
            ),
            Self::make_tool("lorekeeper_stats", DESC_STATS, HashMap::new(), vec![]),
            // -- REFLECT TOOL --------------------------------------------------
            Self::make_tool(
                "lorekeeper_reflect",
                "Analyze the memory bank and surface entries needing attention.\n\n\
                 Returns a JSON report with findings across 5 categories:\n\
                 - stale: entries not updated beyond the stale_days threshold\n\
                 - dead: entries with no access since creation\n\
                 - hot: frequently accessed entries (worth reviewing for freshness)\n\
                 - orphaned: entries whose related_entries links are broken\n\
                 - contradictions: textually similar same-type entries (possible duplicates)\n\n\
                 WHEN TO USE: at session start (toolcheck) or after large batches of changes.",
                HashMap::from([
                    ("focus".into(), json!({
                        "type": "string",
                        "description": "Which finding category to surface. Omit for all.",
                        "enum": ["stale", "dead", "hot", "orphaned", "contradictions", "all"]
                    })),
                    ("stale_days".into(), json!({
                        "type": "integer",
                        "description": "Override config stale_days threshold for this call."
                    })),
                    ("min_access_count".into(), json!({
                        "type": "integer",
                        "description": "Override config hot_access_threshold for this call."
                    })),
                    ("limit".into(), json!({
                        "type": "integer",
                        "description": "Maximum number of findings to return. Default: 20."
                    })),
                ]),
                vec![],
            ),
            // -- HELP TOOL ----------------------------------------------------
            Self::make_tool(
                "lorekeeper_help",
                DESC_HELP,
                HashMap::from([("topic".into(), json!({
                    "type": "string",
                    "description": "Help topic. Omit for overview.",
                    "enum": ["overview","workflow","roles","tools",
                             "DECISION","COMMIT","CONSTRAINT","LESSON","PLAN",
                             "FEATURE","STUB","DEFERRED","BUILDER_NOTE","TECH_DEBT",
                             "SESSION_SUMMARY"]
                }))]),
                vec![],
            ),
        ]
    }

    #[allow(clippy::too_many_lines)]
    fn handle_tool_call(&self, params: CallToolRequestParams) -> Result<Value, String> {
        let tool_name = params.name.clone();
        info!(tool = %tool_name, "tool call received");

        let args = params.arguments.unwrap_or_default();
        let args_value = Value::Object(args.clone());

        match params.name.as_str() {
            "lorekeeper_store" => {
                let input: NewEntry =
                    serde_json::from_value(args_value).map_err(|e| e.to_string())?;
                let title = input.title.clone();
                let body = input.body.clone();
                let entry_type = input.entry_type;
                let entry = self.repo.store(input).map_err(|e| {
                    warn!(error = %e, "lorekeeper_store rejected");
                    e.to_string()
                })?;
                info!(id = %entry.id.0, "entry stored");

                // Duplicate detection: find similar entries in the same type
                let threshold = self.config.store.similarity_threshold;
                let similar =
                    self.repo.find_similar(&title, body, entry_type, threshold).unwrap_or_default();

                if similar.is_empty() {
                    Ok(json!({ "status": "success", "id": entry.id.0 }))
                } else {
                    Ok(json!({
                        "status": "success",
                        "id": entry.id.0,
                        "similar_entries": similar,
                        "warning": "Potential duplicates detected — review similar_entries"
                    }))
                }
            }
            "lorekeeper_update" => {
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;
                let update: UpdateEntry =
                    serde_json::from_value(args_value).map_err(|e| e.to_string())?;
                let entry = self.repo.update(id, update).map_err(|e| {
                    warn!(id = %id, error = %e, "lorekeeper_update rejected");
                    e.to_string()
                })?;
                info!(id = %id, "entry updated");
                serde_json::to_value(entry).map_err(|e| e.to_string())
            }
            "lorekeeper_delete" => {
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;
                self.repo.delete(id).map_err(|e| {
                    warn!(id = %id, error = %e, "lorekeeper_delete failed");
                    e.to_string()
                })?;
                info!(id = %id, "entry soft-deleted");
                Ok(json!({ "status": "success" }))
            }
            "lorekeeper_render" => {
                let entries = self.repo.render_all().map_err(|e| {
                    error!(error = %e, "lorekeeper_render: repo.render_all failed");
                    e.to_string()
                })?;
                let md = render_entries(&entries);
                Ok(json!({ "content": md }))
            }
            "lorekeeper_search" => {
                let query: SearchQuery =
                    serde_json::from_value(args_value).map_err(|e| e.to_string())?;
                let entries = self.repo.search(&query).map_err(|e| {
                    error!(error = %e, "lorekeeper_search: db error");
                    e.to_string()
                })?;
                info!(results = entries.len(), "search complete");
                serde_json::to_value(entries).map_err(|e| e.to_string())
            }
            "lorekeeper_get" => {
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;
                let entry = self.repo.get(id).map_err(|e| {
                    warn!(id = %id, error = %e, "lorekeeper_get: not found or error");
                    e.to_string()
                })?;
                serde_json::to_value(entry).map_err(|e| e.to_string())
            }
            "lorekeeper_recent" => {
                let limit = u32::try_from(
                    args.get("limit").and_then(serde_json::Value::as_u64).unwrap_or(10),
                )
                .unwrap_or(10);
                let entries = self.repo.recent(limit).map_err(|e| {
                    error!(error = %e, "lorekeeper_recent: db error");
                    e.to_string()
                })?;
                serde_json::to_value(entries).map_err(|e| e.to_string())
            }
            "lorekeeper_by_type" => {
                let et_str =
                    args.get("entry_type").and_then(|v| v.as_str()).ok_or("missing entry_type")?;
                let et: EntryType =
                    serde_json::from_value(json!(et_str)).map_err(|e| e.to_string())?;
                let filters: Filters =
                    serde_json::from_value(args_value).map_err(|e| e.to_string())?;
                let entries = self.repo.by_type(et, &filters).map_err(|e| {
                    error!(error = %e, "lorekeeper_by_type: db error");
                    e.to_string()
                })?;
                serde_json::to_value(entries).map_err(|e| e.to_string())
            }
            "lorekeeper_stats" => {
                let stats = self.repo.stats().map_err(|e| {
                    error!(error = %e, "lorekeeper_stats: db error");
                    e.to_string()
                })?;
                serde_json::to_value(stats).map_err(|e| e.to_string())
            }
            "lorekeeper_reflect" => {
                let criteria: ReflectCriteria =
                    serde_json::from_value(args_value).unwrap_or_default();
                let report = self.repo.reflect(&criteria, &self.config).map_err(|e| {
                    error!(error = %e, "lorekeeper_reflect: analysis failed");
                    e.to_string()
                })?;
                info!(
                    state = ?report.state,
                    findings = report.summary.total,
                    "reflect analysis complete"
                );
                serde_json::to_value(report).map_err(|e| e.to_string())
            }
            "lorekeeper_help" => {
                let topic = args.get("topic").and_then(|v| v.as_str()).unwrap_or("overview");
                Ok(json!({ "content": Self::get_help(topic) }))
            }
            other => {
                warn!(tool = %other, "unknown tool called");
                Err(format!("unknown tool: {other}"))
            }
        }
    }

    /// Returns contextual help text for a given topic.
    fn get_help(topic: &str) -> &'static str {
        help::get_help(topic)
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic, clippy::str_to_string, clippy::unimplemented)]
    use super::*;
    use crate::error::LoreError;
    use crate::model::entry::{Entry, EntryId};
    use crate::store::repository::MockEntryRepository;
    use async_trait::async_trait;
    use chrono::Utc;
    use rust_mcp_sdk::schema::CallToolRequestParams;
    use serde_json::{Map, Value, json};
    use std::sync::Arc;

    /// Helper to convert json! object to Map<String, Value>
    fn to_map(v: &Value) -> Map<String, Value> {
        v.as_object().unwrap().to_owned()
    }

    struct NoOpMcpServer;

    #[async_trait]
    impl McpServer for NoOpMcpServer {
        async fn start(self: Arc<Self>) -> rust_mcp_sdk::error::SdkResult<()> {
            unimplemented!("test stub")
        }
        async fn set_client_details(
            &self,
            _: rust_mcp_sdk::schema::InitializeRequestParams,
        ) -> rust_mcp_sdk::error::SdkResult<()> {
            unimplemented!("test stub")
        }
        fn server_info(&self) -> &rust_mcp_sdk::schema::InitializeResult {
            unimplemented!("test stub")
        }
        fn client_info(&self) -> Option<rust_mcp_sdk::schema::InitializeRequestParams> {
            unimplemented!("test stub")
        }
        async fn auth_info(
            &self,
        ) -> tokio::sync::RwLockReadGuard<'_, Option<rust_mcp_sdk::auth::AuthInfo>> {
            unimplemented!("test stub")
        }
        async fn auth_info_cloned(&self) -> Option<rust_mcp_sdk::auth::AuthInfo> {
            unimplemented!("test stub")
        }
        async fn update_auth_info(&self, _: Option<rust_mcp_sdk::auth::AuthInfo>) {
            unimplemented!("test stub")
        }
        async fn wait_for_initialization(&self) {
            unimplemented!("test stub")
        }
        fn task_store(&self) -> Option<Arc<rust_mcp_sdk::task_store::ServerTaskStore>> {
            None
        }
        fn client_task_store(&self) -> Option<Arc<rust_mcp_sdk::task_store::ClientTaskStore>> {
            None
        }
        async fn stderr_message(&self, _: String) -> rust_mcp_sdk::error::SdkResult<()> {
            Ok(())
        }
        fn session_id(&self) -> Option<rust_mcp_sdk::SessionId> {
            None
        }
        async fn send(
            &self,
            _: rust_mcp_sdk::schema::MessageFromServer,
            _: Option<rust_mcp_sdk::schema::RequestId>,
            _: Option<std::time::Duration>,
        ) -> rust_mcp_sdk::error::SdkResult<Option<rust_mcp_sdk::schema::ClientMessage>> {
            unimplemented!("test stub")
        }
        async fn send_batch(
            &self,
            _: Vec<rust_mcp_sdk::schema::ServerMessage>,
            _: Option<std::time::Duration>,
        ) -> rust_mcp_sdk::error::SdkResult<Option<Vec<rust_mcp_sdk::schema::ClientMessage>>>
        {
            unimplemented!("test stub")
        }
    }

    #[test]
    fn handle_store_malformed_json() {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_store".to_owned(),
            arguments: Some(to_map(&json!({ "entry_type": 123 }))),
            meta: None,
            task: None,
        };
        let result = handler.handle_tool_call(params);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid type: integer `123`"));
    }

    #[test]
    fn handle_get_missing_id() {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_get".to_owned(),
            arguments: Some(Map::new()),
            meta: None,
            task: None,
        };
        let result = handler.handle_tool_call(params);
        assert_eq!(result.unwrap_err(), "missing id");
    }

    #[test]
    fn handle_delete_missing_id() {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_delete".to_owned(),
            arguments: Some(Map::new()),
            meta: None,
            task: None,
        };
        let result = handler.handle_tool_call(params);
        assert_eq!(result.unwrap_err(), "missing id");
    }

    #[test]
    fn handle_by_type_missing_entry_type() {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_by_type".to_owned(),
            arguments: Some(Map::new()),
            meta: None,
            task: None,
        };
        let result = handler.handle_tool_call(params);
        assert_eq!(result.unwrap_err(), "missing entry_type");
    }

    #[test]
    fn handle_by_type_invalid_entry_type() {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_by_type".to_owned(),
            arguments: Some(to_map(&json!({ "entry_type": "INVALID" }))),
            meta: None,
            task: None,
        };
        let result = handler.handle_tool_call(params);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown variant `INVALID`"));
    }

    #[test]
    fn handle_render_repo_error() {
        let mut mock = MockEntryRepository::new();
        mock.expect_render_all().times(1).returning(|| Err(LoreError::Internal("db down".into())));
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_render".to_owned(),
            arguments: None,
            meta: None,
            task: None,
        };
        let result = handler.handle_tool_call(params);
        assert!(result.unwrap_err().contains("db down"));
    }

    #[test]
    fn handle_search_repo_error() {
        let mut mock = MockEntryRepository::new();
        mock.expect_search().times(1).returning(|_| Err(LoreError::Internal("db down".into())));
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_search".to_owned(),
            arguments: Some(to_map(&json!({ "query": "test", "limit": 10 }))),
            meta: None,
            task: None,
        };
        let result = handler.handle_tool_call(params);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("db down"));
    }

    #[test]
    fn handle_recent_repo_error() {
        let mut mock = MockEntryRepository::new();
        mock.expect_recent().times(1).returning(|_| Err(LoreError::Internal("db down".into())));
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_recent".to_owned(),
            arguments: None,
            meta: None,
            task: None,
        };
        let result = handler.handle_tool_call(params);
        assert!(result.unwrap_err().contains("db down"));
    }

    #[test]
    fn handle_stats_repo_error() {
        let mut mock = MockEntryRepository::new();
        mock.expect_stats().times(1).returning(|| Err(LoreError::Internal("db down".into())));
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_stats".to_owned(),
            arguments: None,
            meta: None,
            task: None,
        };
        let result = handler.handle_tool_call(params);
        assert!(result.unwrap_err().contains("db down"));
    }

    #[tokio::test]
    async fn handle_request_list_tools() {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let request =
            RequestFromClient::ListToolsRequest(serde_json::from_value(json!({})).unwrap());

        let result = handler.handle_request(request, Arc::new(NoOpMcpServer)).await;
        if let Ok(ResultFromServer::ListToolsResult(res)) = result {
            let ListToolsResult { tools, .. } = res;
            assert_eq!(tools.len(), 11);
        } else {
            panic!("expected ListToolsResult, got {result:?}");
        }
    }

    #[tokio::test]
    async fn handle_request_call_tool_ok() {
        use crate::store::repository::MemoryStats;
        let mut mock = MockEntryRepository::new();
        mock.expect_stats().times(1).returning(|| {
            Ok(MemoryStats { total: 42, by_type: vec![], by_status: vec![], last_updated: None })
        });

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_stats".to_owned(),
            arguments: None,
            meta: None,
            task: None,
        };
        let request = RequestFromClient::CallToolRequest(params);

        let result = handler.handle_request(request, Arc::new(NoOpMcpServer)).await;
        if let Ok(ResultFromServer::CallToolResult(res)) = result {
            assert!(res.is_error.is_none());
            assert!(serde_json::to_string(&res.content).unwrap().contains("42"));
        } else {
            panic!("expected CallToolResult, got {result:?}");
        }
    }

    #[tokio::test]
    async fn handle_request_call_tool_err() {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "unknown_tool".to_owned(),
            arguments: None,
            meta: None,
            task: None,
        };
        let request = RequestFromClient::CallToolRequest(params);

        let result = handler.handle_request(request, Arc::new(NoOpMcpServer)).await;
        if let Ok(ResultFromServer::CallToolResult(res)) = result {
            assert_eq!(res.is_error, Some(true));
        } else {
            panic!("expected CallToolResult, got {result:?}");
        }
    }

    #[tokio::test]
    async fn handle_request_unknown_method() {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let result = handler
            .handle_request(
                RequestFromClient::InitializeRequest(
                    serde_json::from_value(json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {},
                        "clientInfo": { "name": "test", "version": "1.0" }
                    }))
                    .unwrap(),
                ),
                Arc::new(NoOpMcpServer),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn handle_notification_returns_ok() {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let note = NotificationFromClient::InitializedNotification(
            serde_json::from_value(json!({})).unwrap(),
        );
        let result = handler.handle_notification(note, Arc::new(NoOpMcpServer)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn handle_error_returns_ok() {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let err = RpcError::internal_error();
        let result = handler.handle_error(&err, Arc::new(NoOpMcpServer)).await;
        assert!(result.is_ok());
    }

    fn test_entry(id: &str) -> Entry {
        Entry {
            id: EntryId(id.into()),
            entry_type: EntryType::Decision,
            title: "Test Entry".into(),
            body: None,
            role: "architect".into(),
            tags: vec![],
            related_entries: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_deleted: false,
            access_count: 0,
            last_accessed_at: None,
            data: serde_json::Value::Null,
        }
    }

    #[test]
    fn handle_store_success() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_store().times(1).returning(|_| Ok(test_entry("uuid1")));
        mock.expect_find_similar().times(1).returning(|_, _, _, _| Ok(vec![]));

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_store".to_owned(),
            arguments: Some(to_map(&json!({
                "entry_type": "DECISION",
                "role": "architect",
                "title": "New Decision"
            }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params)?;
        assert_eq!(result["status"], "success");
        assert_eq!(result["id"], "uuid1");
        Ok(())
    }

    #[test]
    fn handle_store_validation_error() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_store()
            .times(1)
            .returning(|_| Err(LoreError::Validation("missing title".into())));

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_store".to_owned(),
            arguments: Some(to_map(&json!({
                "entry_type": "DECISION",
                "role": "architect",
                "title": ""
            }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params);
        let err: String = match result {
            Err(e) => e,
            Ok(_) => return Err("should have failed".into()),
        };
        assert!(err.contains("missing title"));
        Ok(())
    }

    #[test]
    fn handle_get_success() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_get().times(1).returning(|id| {
            if id == "id1" { Ok(test_entry("id1")) } else { Err(LoreError::NotFound(id.into())) }
        });

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_get".to_owned(),
            arguments: Some(to_map(&json!({ "id": "id1" }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params)?;
        assert_eq!(result["id"], "id1");
        assert_eq!(result["title"], "Test Entry");
        Ok(())
    }

    #[test]
    fn handle_get_not_found() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_get().times(1).returning(|id| Err(LoreError::NotFound(id.into())));

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_get".to_owned(),
            arguments: Some(to_map(&json!({ "id": "missing" }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params);
        let err: String = match result {
            Err(e) => e,
            Ok(_) => return Err("should have failed".into()),
        };
        assert!(err.contains("not found: missing"));
        Ok(())
    }

    #[test]
    fn handle_unknown_tool() -> Result<(), String> {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "unknown_tool".to_owned(),
            arguments: None,
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params);
        let err: String = match result {
            Err(e) => e,
            Ok(_) => return Err("should have failed".into()),
        };
        assert!(err.contains("unknown tool: unknown_tool"));
        Ok(())
    }

    #[test]
    fn handle_update_success() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_update().times(1).returning(|id, _| Ok(test_entry(id)));

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_update".to_owned(),
            arguments: Some(to_map(&json!({ "id": "id1", "title": "Updated" }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params)?;
        assert_eq!(result["id"], "id1");
        assert_eq!(result["title"], "Test Entry");
        Ok(())
    }

    #[test]
    fn handle_update_not_found() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_update().times(1).returning(|id, _| Err(LoreError::NotFound(id.into())));

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_update".to_owned(),
            arguments: Some(to_map(&json!({ "id": "missing", "title": "X" }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params);
        match result {
            Err(e) => {
                assert!(e.contains("not found"));
                Ok(())
            }
            Ok(_) => Err("expected error".into()),
        }
    }

    #[test]
    fn handle_update_missing_id() -> Result<(), String> {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_update".to_owned(),
            arguments: Some(to_map(&json!({ "title": "No ID" }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params);
        match result {
            Err(e) => {
                assert_eq!(e, "missing id");
                Ok(())
            }
            Ok(_) => Err("expected error".into()),
        }
    }

    #[test]
    fn handle_delete_success() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_delete().times(1).returning(|_| Ok(()));

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_delete".to_owned(),
            arguments: Some(to_map(&json!({ "id": "id1" }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params)?;
        assert_eq!(result["status"], "success");
        Ok(())
    }

    #[test]
    fn handle_delete_not_found() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_delete().times(1).returning(|id| Err(LoreError::NotFound(id.into())));

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_delete".to_owned(),
            arguments: Some(to_map(&json!({ "id": "missing" }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params);
        match result {
            Err(e) => {
                assert!(e.contains("not found"));
                Ok(())
            }
            Ok(_) => Err("expected error".into()),
        }
    }

    #[test]
    fn handle_render_success() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_render_all().times(1).returning(|| Ok(vec![test_entry("id1")]));

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_render".to_owned(),
            arguments: None,
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params)?;
        let content = result["content"].as_str().ok_or("missing content")?;
        assert!(content.contains("Test Entry"));
        assert!(content.contains("Lorekeeper Memory Dump"));
        Ok(())
    }

    #[test]
    fn handle_search_success() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_search().times(1).returning(|_| Ok(vec![test_entry("s1")]));

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_search".to_owned(),
            arguments: Some(to_map(&json!({ "query": "test", "limit": 10 }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params)?;
        let arr = result.as_array().ok_or("expected array")?;
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "s1");
        Ok(())
    }

    #[test]
    fn handle_recent_success() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_recent().times(1).returning(|_| Ok(vec![test_entry("r1")]));

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_recent".to_owned(),
            arguments: Some(to_map(&json!({ "limit": 5 }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params)?;
        let arr = result.as_array().ok_or("expected array")?;
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "r1");
        Ok(())
    }

    #[test]
    fn handle_recent_default_limit() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_recent().withf(|limit| *limit == 10).times(1).returning(|_| Ok(vec![]));

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_recent".to_owned(),
            arguments: None,
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params)?;
        assert_eq!(result.as_array().ok_or("expected array")?.len(), 0);
        Ok(())
    }

    #[test]
    fn handle_by_type_success() -> Result<(), String> {
        let mut mock = MockEntryRepository::new();
        mock.expect_by_type().times(1).returning(|_, _| Ok(vec![test_entry("bt1")]));

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_by_type".to_owned(),
            arguments: Some(to_map(&json!({
                "entry_type": "DECISION",
                "limit": 10,
                "offset": 0
            }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params)?;
        let arr = result.as_array().ok_or("expected array")?;
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "bt1");
        Ok(())
    }

    #[test]
    fn handle_stats_success() -> Result<(), String> {
        use crate::store::repository::MemoryStats;

        let mut mock = MockEntryRepository::new();
        mock.expect_stats().times(1).returning(|| {
            Ok(MemoryStats { total: 42, by_type: vec![], by_status: vec![], last_updated: None })
        });

        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_stats".to_owned(),
            arguments: None,
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params)?;
        assert_eq!(result["total"], 42);
        Ok(())
    }

    #[test]
    fn handle_help_success() -> Result<(), String> {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_help".to_owned(),
            arguments: Some(to_map(&json!({ "topic": "roles" }))),
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params)?;
        let content = result["content"].as_str().ok_or("missing content")?;
        assert!(content.contains("Role Enforcement"));
        Ok(())
    }

    #[test]
    fn handle_help_default_topic() -> Result<(), String> {
        let mock = MockEntryRepository::new();
        let handler = LoreHandler::with_defaults(Arc::new(mock));
        let params = CallToolRequestParams {
            name: "lorekeeper_help".to_owned(),
            arguments: None,
            meta: None,
            task: None,
        };

        let result = handler.handle_tool_call(params)?;
        let content = result["content"].as_str().ok_or("missing content")?;
        assert!(content.contains("Workflow Guide"));
        Ok(())
    }
}
