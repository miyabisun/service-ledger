use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

#[derive(Debug)]
pub enum Error {
    Invalid(String),
    NotFound,
    /// Another record already names the same service.
    Duplicate(i64),
    Unavailable,
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        tracing::error!(%error, "SQLite operation failed");
        Self::Unavailable
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        tracing::error!(%error, "database file operation failed");
        Self::Unavailable
    }
}

impl Error {
    #[must_use]
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Invalid(_) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Duplicate(_) => StatusCode::CONFLICT,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// `{error, code}` plus `existing_id` for duplicates; shared by HTTP and MCP.
    #[must_use]
    pub fn body(&self) -> Value {
        match self {
            Self::Invalid(message) => json!({"error": message, "code": "invalid"}),
            Self::NotFound => json!({"error": "サービスが見つかりません", "code": "not_found"}),
            Self::Duplicate(id) => json!({
                "error": "同じ名前または同じ Web サイトのサービスが登録済みです",
                "code": "duplicate",
                "existing_id": id,
            }),
            Self::Unavailable => json!({
                "error": "データベースを操作できませんでした。サーバーのログを確認してください",
                "code": "unavailable",
            }),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.status(), Json(self.body())).into_response()
    }
}
