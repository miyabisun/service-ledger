//! MCP tools over the same store as the HTTP API.
use std::sync::Arc;

use axum::Router;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Implementation, ServerCapabilities, ServerConfig},
    schemars, tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    Error,
    http::{AppState, filter},
    service::{NewService, Patch},
};

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Search {
    /// Case-insensitive substring of name, URLs, source or note. Omit to list all.
    #[serde(default)]
    pub query: Option<String>,
    /// `old` (旧住所のまま) / `changed` (変更済み) / `unknown` (不明).
    #[serde(default)]
    pub address_status: Option<String>,
    /// `bulk` (一括手段) / `member_page` (会員ページ) / `phone_or_paper` (電話・書面) / `unknown`.
    #[serde(default)]
    pub change_method: Option<String>,
    /// 1..200, default 50.
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Id {
    pub id: i64,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub id: i64,
    /// Fields to change (nested so unknown keys are rejected, not ignored).
    pub changes: Patch,
}

fn answer(result: Result<Value, Error>) -> CallToolResult {
    match result {
        Ok(value) => CallToolResult::structured(value),
        Err(error) => {
            let mut result = CallToolResult::structured(error.body());
            result.is_error = Some(true);
            result
        }
    }
}

fn to_value(value: impl serde::Serialize) -> Value {
    serde_json::to_value(value).expect("records serialize to JSON")
}

#[derive(Clone)]
struct Ledger {
    store: AppState,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Ledger {
    fn new(store: AppState) -> Self {
        Self {
            store,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Search registered services (contracts and accounts) by substring and filters. Sorted by name. Returns services, counts per address_status for the query, total and next_offset (follow until null)."
    )]
    fn service_search(&self, Parameters(a): Parameters<Search>) -> CallToolResult {
        answer((|| {
            let limit = a.limit.unwrap_or(50);
            if !(1..=200).contains(&limit) {
                return Err(Error::Invalid("limit は 1..200 です".into()));
            }
            let filter = filter(
                a.query,
                a.address_status.as_deref(),
                a.change_method.as_deref(),
            )?;
            let found = self.store.search(&filter)?;
            let total = found.services.len();
            let offset = a.offset.unwrap_or(0).min(total);
            let end = offset.saturating_add(limit).min(total);
            Ok(json!({
                "services": to_value(&found.services[offset..end]),
                "counts": found.counts,
                "total": total,
                "next_offset": (end < total).then_some(end),
            }))
        })())
    }

    #[tool(description = "Read one service by id.")]
    fn service_get(&self, Parameters(a): Parameters<Id>) -> CallToolResult {
        answer(self.store.get(a.id).map(to_value))
    }

    #[tool(
        description = "Register a service the user contracts or uses. name and source (where you found it) are required. A service with the same name or the same website host is a duplicate: the error returns existing_id; update that record instead. Never pass passwords, account numbers or the user's address."
    )]
    fn service_create(&self, Parameters(a): Parameters<NewService>) -> CallToolResult {
        answer(
            a.validate()
                .map_err(Error::Invalid)
                .and_then(|fields| self.store.create(&fields))
                .map(to_value),
        )
    }

    #[tool(
        description = "Update a service by id with {id, changes: {...}}. Fields omitted from changes stay unchanged; website_url or address_change_url null clears it. Set address_status to changed after the address change is done."
    )]
    fn service_update(&self, Parameters(a): Parameters<Update>) -> CallToolResult {
        answer(self.store.update(a.id, a.changes).map(to_value))
    }

    #[tool(description = "Delete a service by id. It cannot be undone.")]
    fn service_delete(&self, Parameters(a): Parameters<Id>) -> CallToolResult {
        answer(
            self.store
                .delete(a.id)
                .map(|()| json!({"ok": true, "id": a.id, "deleted": true})),
        )
    }
}

// The macro writes async fns without awaits; newer clippy flags that.
#[allow(unknown_lints, clippy::unused_async_trait_impl)]
#[tool_handler(router = self.tool_router)]
impl ServerHandler for Ledger {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_server_info(
            Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        )
    }
}

pub fn endpoint<S: Clone + Send + Sync + 'static>(store: &AppState) -> Router<S> {
    let ledger = Ledger::new(Arc::clone(store));
    let service = StreamableHttpService::new(
        move || Ok(ledger.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().disable_allowed_hosts(),
    );
    Router::new().nest_service("/mcp", service)
}
