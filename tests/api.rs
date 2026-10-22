//! HTTP and MCP contracts against a temporary `SQLite` file. Only fictional services appear here.
#![allow(clippy::too_many_lines)] // each test walks one end-to-end scenario
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode},
};
use serde_json::{Value, json};
use service_ledger::{Store, app};
use tower::ServiceExt;

struct Ledger {
    dir: tempfile::TempDir,
    app: Router,
}

impl Ledger {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temporary directory is creatable");
        let app = app(Store::open(&dir.path().join("data/service-ledger.db"))
            .expect("a fresh ledger opens in the temporary directory"));
        Self { dir, app }
    }

    fn reopen(self) -> Self {
        let dir = self.dir;
        drop(self.app);
        let app = app(Store::open(&dir.path().join("data/service-ledger.db"))
            .expect("the ledger written by the previous app reopens"));
        Self { dir, app }
    }

    async fn send(&self, method: &str, uri: &str, body: Option<String>) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(uri);
        if body.is_some() {
            request = request.header("content-type", "application/json");
        }
        let response = self
            .app
            .clone()
            .oneshot(
                request
                    .body(body.map_or_else(Body::empty, Body::from))
                    .expect("request parts are valid"),
            )
            .await
            .expect("the router is infallible");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the response body is readable");
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn json(&self, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
        self.send(method, uri, Some(body.to_string())).await
    }

    async fn names(&self, uri: &str) -> Vec<String> {
        let (status, body) = self.send("GET", uri, None).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        body["services"]
            .as_array()
            .expect("the list response has a services array")
            .iter()
            .map(|s| {
                s["name"]
                    .as_str()
                    .expect("every service has a string name")
                    .to_owned()
            })
            .collect()
    }
}

#[tokio::test]
async fn http_registers_searches_updates_and_deletes() {
    let ledger = Ledger::new();
    let (status, mobile) = ledger
        .json(
            "POST",
            "/api/services",
            json!({
                "name": "Example Mobile",
                "website_url": "https://www.example.com/",
                "address_change_url": "https://www.example.com/account/address",
                "change_method": "member_page",
                "address_status": "old",
                "source": "fictional invoice mail",
                "note": "line one\nline two",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{mobile}");
    assert_eq!(mobile["address_status"], "old");
    assert_eq!(mobile["change_method"], "member_page");
    assert!(mobile["created_at"].as_str().unwrap().ends_with('Z'));
    let id = mobile["id"].as_i64().unwrap();

    for (name, url, status) in [
        ("Example Power", "https://power.example.net", "unknown"),
        ("100% Example Water", "https://water.example.org", "changed"),
    ] {
        let (code, body) = ledger
            .json(
                "POST",
                "/api/services",
                json!({"name": name, "website_url": url, "address_status": status, "source": "fixture"}),
            )
            .await;
        assert_eq!(code, StatusCode::CREATED, "{body}");
    }

    assert_eq!(
        ledger.names("/api/services").await,
        ["100% Example Water", "Example Mobile", "Example Power"]
    );
    assert_eq!(
        ledger.names("/api/services?q=POWER").await,
        ["Example Power"]
    );
    assert_eq!(
        ledger.names("/api/services?q=example.net").await,
        ["Example Power"]
    );
    assert_eq!(
        ledger.names("/api/services?q=line%20two").await,
        ["Example Mobile"]
    );
    assert_eq!(
        ledger.names("/api/services?q=%25").await,
        ["100% Example Water"]
    );
    assert_eq!(
        ledger.names("/api/services?q=_").await,
        Vec::<String>::new()
    );
    assert_eq!(
        ledger.names("/api/services?address_status=old").await,
        ["Example Mobile"]
    );
    assert_eq!(
        ledger
            .names("/api/services?change_method=unknown&q=example")
            .await,
        ["100% Example Water", "Example Power"]
    );
    let (_, listed) = ledger
        .send("GET", "/api/services?q=example&address_status=old", None)
        .await;
    assert_eq!(
        listed["counts"],
        json!({"old": 1, "changed": 1, "unknown": 1})
    );
    let (status, _) = ledger
        .send("GET", "/api/services?address_status=moved", None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, updated) = ledger
        .json(
            "PATCH",
            &format!("/api/services/{id}"),
            json!({"address_status": "changed", "website_url": null}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["address_status"], "changed");
    assert_eq!(updated["website_url"], Value::Null);
    assert_eq!(updated["address_change_url"], mobile["address_change_url"]);
    assert_eq!(updated["created_at"], mobile["created_at"]);

    let (status, got) = ledger
        .send("GET", &format!("/api/services/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got, updated);

    let (status, _) = ledger
        .send("DELETE", &format!("/api/services/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    for method in ["GET", "DELETE"] {
        let (status, body) = ledger
            .send(method, &format!("/api/services/{id}"), None)
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }

    let ledger = ledger.reopen();
    assert_eq!(
        ledger.names("/api/services").await,
        ["100% Example Water", "Example Power"]
    );
}

#[tokio::test]
async fn http_rejects_invalid_input_and_duplicates() {
    let ledger = Ledger::new();
    let (status, first) = ledger
        .json(
            "POST",
            "/api/services",
            json!({"name": "Example Mobile", "website_url": "https://example.com", "source": "fixture"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    for body in [
        json!({"name": "", "source": "fixture"}),
        json!({"name": "Script", "website_url": "javascript:alert(1)", "source": "fixture"}),
        json!({"name": "Credentials", "website_url": "https://user:pw@creds.example", "source": "fixture"}),
        json!({"name": "No source"}),
        json!({"name": "Password", "source": "fixture", "password": "hunter2"}),
        json!({"name": "Status", "source": "fixture", "address_status": "moved"}),
    ] {
        let (status, error) = ledger.json("POST", "/api/services", body.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {error}");
        assert_eq!(error["code"], "invalid");
    }
    for body in [
        json!({"name": " example MOBILE ", "source": "fixture"}),
        json!({"name": "Example Mobile Store", "website_url": "https://www.example.com/shop", "source": "fixture"}),
    ] {
        let (status, error) = ledger.json("POST", "/api/services", body).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(error["existing_id"], first["id"]);
    }
    let (_, other) = ledger
        .json(
            "POST",
            "/api/services",
            json!({"name": "Example Power", "source": "fixture"}),
        )
        .await;
    let (status, error) = ledger
        .json(
            "PATCH",
            &format!("/api/services/{}", other["id"]),
            json!({"website_url": "https://example.com/other"}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["existing_id"], first["id"]);
    let (status, _) = ledger
        .json(
            "PATCH",
            &format!("/api/services/{}", first["id"]),
            json!({"name": "Example Mobile"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "a record is not its own duplicate");
    assert_eq!(
        ledger.names("/api/services").await,
        ["Example Mobile", "Example Power"]
    );
}

#[tokio::test]
async fn bitwarden_listing_imports_once_without_secret_values() {
    let ledger = Ledger::new();
    let listing = include_str!("fixtures/rbw-list.json");
    let (status, result) = ledger
        .send("POST", "/api/import/bitwarden", Some(listing.into()))
        .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result, json!({"created": 3, "duplicates": 2, "skipped": 0}));
    let (_, again) = ledger
        .send("POST", "/api/import/bitwarden", Some(listing.into()))
        .await;
    assert_eq!(again, json!({"created": 0, "duplicates": 5, "skipped": 0}));
    let (_, listed) = ledger.send("GET", "/api/services", None).await;
    let services = listed["services"].as_array().unwrap();
    let summary: Vec<_> = services
        .iter()
        .map(|s| {
            (
                s["name"].as_str().unwrap(),
                s["website_url"].as_str(),
                s["source"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("Example Card", None, "Bitwarden"),
            (
                "Example Mobile",
                Some("https://www.example.com"),
                "Bitwarden"
            ),
            (
                "Example Power",
                Some("https://power.example.net"),
                "Bitwarden"
            ),
        ]
    );
    assert!(services.iter().all(|s| s["address_status"] == "unknown"));

    // An export that does carry secrets still stores only the name and origin.
    let export = json!({"items": [{
        "type": 1, "name": "Example Books", "notes": "fictional-secret-note",
        "login": {"username": "fictional-user", "password": "fictional-password-123",
                  "uris": [{"uri": "https://books.example.com/login?token=fictional-token"}]},
        "fields": [{"name": "pin", "value": "fictional-pin-9876"}]
    }]});
    let (_, result) = ledger
        .send("POST", "/api/import/bitwarden", Some(export.to_string()))
        .await;
    assert_eq!(result["created"], 1);
    let (status, _) = ledger
        .send(
            "POST",
            "/api/import/bitwarden",
            Some("user,password\n".into()),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let dir = ledger.dir.path().join("data");
    drop(ledger.app);
    let mut stored = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        stored.extend(std::fs::read(entry.unwrap().path()).unwrap());
    }
    let stored = String::from_utf8_lossy(&stored);
    assert!(stored.contains("https://books.example.com"));
    for secret in [
        "fictional-password-123",
        "fictional-user",
        "fictional-secret-note",
        "fictional-token",
        "fictional-pin-9876",
        "someone@example.com",
    ] {
        assert!(
            !stored.contains(secret),
            "{secret} reached the database files"
        );
    }
}

#[tokio::test]
async fn database_is_a_private_wal_file() {
    let ledger = Ledger::new();
    let path = ledger.dir.path().join("data/service-ledger.db");
    let connection = rusqlite::Connection::open(&path).unwrap();
    let mode: String = connection
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "database readable by others: {mode:o}");
    }
    // A newer schema is refused instead of being written by an older server.
    connection.pragma_update(None, "user_version", 99).unwrap();
    drop(connection);
    drop(ledger.app);
    assert!(Store::open(&path).is_err());
}

async fn rpc(app: &Router, session: Option<&str>, message: Value) -> (HeaderMap, Value) {
    let mut request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("host", "localhost")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream");
    if let Some(session) = session {
        request = request.header("mcp-session-id", session);
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(Body::from(message.to_string()))
                .expect("request parts are valid"),
        )
        .await
        .expect("the router is infallible");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("the response body is readable");
    let text = String::from_utf8(bytes.to_vec()).expect("the MCP response is UTF-8");
    assert!(status.is_success(), "MCP {status}: {text}");
    let value = serde_json::from_str(&text).unwrap_or_else(|_| {
        text.lines()
            .find_map(|line| {
                line.strip_prefix("data: ")
                    .and_then(|j| serde_json::from_str(j).ok())
            })
            .unwrap_or_default()
    });
    (headers, value)
}

async fn call(app: &Router, session: &str, name: &str, arguments: Value) -> (bool, Value) {
    let (_, value) = rpc(
        app,
        Some(session),
        json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":name,"arguments":arguments}}),
    )
    .await;
    if value.get("error").is_some() {
        // Argument schema violations are JSON-RPC errors rather than tool results.
        return (true, value["error"].clone());
    }
    let result = &value["result"];
    (
        result["isError"].as_bool().unwrap_or(false),
        result["structuredContent"].clone(),
    )
}

#[tokio::test]
async fn mcp_registers_searches_updates_and_deletes() {
    let ledger = Ledger::new();
    let app = &ledger.app;
    let (headers, initialized) = rpc(
        app,
        None,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"contract-test","version":"1"}}}),
    )
    .await;
    assert!(initialized["result"]["capabilities"]["tools"].is_object());
    let session = headers["mcp-session-id"].to_str().unwrap().to_owned();
    let session = session.as_str();
    rpc(
        app,
        Some(session),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    )
    .await;
    let (_, tools) = rpc(
        app,
        Some(session),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    )
    .await;
    let mut names: Vec<_> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "service_create",
            "service_delete",
            "service_get",
            "service_search",
            "service_update"
        ]
    );

    let (error, created) = call(
        app,
        session,
        "service_create",
        json!({"name":"Example Mobile","website_url":"https://example.com","address_status":"old","source":"fictional mail"}),
    )
    .await;
    assert!(!error, "{created}");
    let id = created["id"].as_i64().unwrap();
    for n in 0..3 {
        let (error, _) = call(
            app,
            session,
            "service_create",
            json!({"name": format!("Example Shop {n}"), "source": "fixture"}),
        )
        .await;
        assert!(!error);
    }
    let (error, duplicate) = call(
        app,
        session,
        "service_create",
        json!({"name":"example mobile","source":"fixture"}),
    )
    .await;
    assert!(error);
    assert_eq!(duplicate["existing_id"], id);
    let (error, invalid) = call(
        app,
        session,
        "service_create",
        json!({"name":"Bad","website_url":"javascript:alert(1)","source":"fixture"}),
    )
    .await;
    assert!(error);
    assert_eq!(invalid["code"], "invalid");

    let (_, page) = call(
        app,
        session,
        "service_search",
        json!({"query":"shop","limit":2}),
    )
    .await;
    assert_eq!(page["total"], 3);
    assert_eq!(page["services"].as_array().unwrap().len(), 2);
    assert_eq!(page["next_offset"], 2);
    let (_, rest) = call(
        app,
        session,
        "service_search",
        json!({"query":"shop","limit":2,"offset":2}),
    )
    .await;
    assert_eq!(rest["services"][0]["name"], "Example Shop 2");
    assert_eq!(rest["next_offset"], Value::Null);
    let (_, old) = call(
        app,
        session,
        "service_search",
        json!({"address_status":"old"}),
    )
    .await;
    assert_eq!(old["services"][0]["id"], id);
    assert_eq!(old["counts"]["unknown"], 3);

    let (error, updated) = call(
        app,
        session,
        "service_update",
        json!({"id": id, "changes": {"address_status": "changed", "note": "done by fixture"}}),
    )
    .await;
    assert!(!error, "{updated}");
    assert_eq!(updated["address_status"], "changed");
    assert_eq!(updated["website_url"], "https://example.com");
    let (error, _) = call(
        app,
        session,
        "service_update",
        json!({"id": id, "changes": {"password": "x"}}),
    )
    .await;
    assert!(error, "unknown update keys are rejected");
    let (_, got) = call(app, session, "service_get", json!({"id": id})).await;
    assert_eq!(got["note"], "done by fixture");

    // HTTP sees what MCP wrote.
    assert_eq!(
        ledger.names("/api/services?address_status=changed").await,
        ["Example Mobile"]
    );

    let (error, deleted) = call(app, session, "service_delete", json!({"id": id})).await;
    assert!(!error);
    assert_eq!(deleted["deleted"], true);
    let (error, missing) = call(app, session, "service_get", json!({"id": id})).await;
    assert!(error);
    assert_eq!(missing["code"], "not_found");
}

#[tokio::test]
async fn embedded_ui_and_unknown_routes() {
    let ledger = Ledger::new();
    let response = ledger
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/services/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/html");
    let html = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(html.to_vec()).unwrap();
    assert!(html.starts_with("<!doctype html>"));
    let asset = html
        .split("src=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("compiled HTML references its script");
    let response = ledger
        .app
        .clone()
        .oneshot(Request::builder().uri(asset).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["content-type"], "text/javascript");
    for uri in ["/api", "/api/", "/api/missing"] {
        let (status, _) = ledger.send("GET", uri, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
    let (status, _) = ledger.send("GET", "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
}
