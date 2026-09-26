use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    Error, bitwarden,
    db::{Filter, ImportResult, SearchResult, Store},
    service::{AddressStatus, ChangeMethod, NewService, Patch, Service},
};

pub type AppState = Arc<Store>;

#[derive(Deserialize)]
pub struct ListQuery {
    q: Option<String>,
    address_status: Option<String>,
    change_method: Option<String>,
}

/// Build a filter from wire values; empty strings mean "any".
pub fn filter(
    query: Option<String>,
    address_status: Option<&str>,
    change_method: Option<&str>,
) -> Result<Filter, Error> {
    let status = match address_status.filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(AddressStatus::parse(s).ok_or_else(|| {
            Error::Invalid("address_status は old / changed / unknown のいずれかです".into())
        })?),
    };
    let method = match change_method.filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(ChangeMethod::parse(s).ok_or_else(|| {
            Error::Invalid(
                "change_method は bulk / member_page / phone_or_paper / unknown のいずれかです"
                    .into(),
            )
        })?),
    };
    Ok(Filter {
        query,
        address_status: status,
        change_method: method,
    })
}

fn body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Error> {
    payload
        .map(|Json(value)| value)
        .map_err(|rejection| Error::Invalid(rejection.body_text()))
}

pub async fn healthz() -> &'static str {
    "ok\n"
}

pub async fn api_health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

pub async fn list(
    State(store): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<SearchResult>, Error> {
    let filter = filter(
        query.q,
        query.address_status.as_deref(),
        query.change_method.as_deref(),
    )?;
    store.search(&filter).map(Json)
}

pub async fn get(
    State(store): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Service>, Error> {
    store.get(id).map(Json)
}

pub async fn create(
    State(store): State<AppState>,
    payload: Result<Json<NewService>, JsonRejection>,
) -> Result<impl IntoResponse, Error> {
    let fields = body(payload)?.validate().map_err(Error::Invalid)?;
    Ok((StatusCode::CREATED, Json(store.create(&fields)?)))
}

pub async fn update(
    State(store): State<AppState>,
    Path(id): Path<i64>,
    payload: Result<Json<Patch>, JsonRejection>,
) -> Result<Json<Service>, Error> {
    store.update(id, body(payload)?).map(Json)
}

pub async fn delete(
    State(store): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, Error> {
    store.delete(id).map(|()| StatusCode::NO_CONTENT)
}

/// The raw listing is parsed in memory and dropped; only names and origins reach the ledger.
pub async fn import_bitwarden(
    State(store): State<AppState>,
    text: String,
) -> Result<Json<ImportResult>, Error> {
    let candidates = bitwarden::parse(&text).map_err(Error::Invalid)?;
    store.import(candidates).map(Json)
}

pub async fn api_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "API route not found\n")
}
