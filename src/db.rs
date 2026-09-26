//! The ledger database: one `SQLite` file in WAL mode, so `VACUUM INTO` can copy it while the server runs.
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Mutex, MutexGuard},
};

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use crate::{
    Error,
    bitwarden::{self, Candidate},
    service::{AddressStatus, ChangeMethod, Fields, Patch, Service, like_pattern, same_service},
};

const SCHEMA_VERSION: i64 = 1;
const COLUMNS: &str = "id, name, website_url, address_change_url, change_method, \
    address_status, source, note, created_at, updated_at";
const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%SZ', 'now')";
pub const IMPORT_SOURCE: &str = "Bitwarden";

#[derive(Default)]
pub struct Filter {
    pub query: Option<String>,
    pub address_status: Option<AddressStatus>,
    pub change_method: Option<ChangeMethod>,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub services: Vec<Service>,
    /// Matches per address status for the same query and method, ignoring the status filter.
    pub counts: BTreeMap<&'static str, i64>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ImportResult {
    pub created: usize,
    pub duplicates: usize,
    pub skipped: usize,
}

pub struct Store {
    // ponytail: one serialized connection; a pool only if measured contention warrants it.
    connection: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, Error> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        // Personal data: create the file private before SQLite opens it (WAL/SHM inherit the mode).
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        options.open(path)?;

        let connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        let mode: String = connection.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            tracing::error!(%mode, "SQLite refused WAL mode");
            return Err(Error::Unavailable);
        }
        let version: i64 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        match version {
            0 => connection.execute_batch(&format!(
                "BEGIN;
                CREATE TABLE services (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    website_url TEXT,
                    address_change_url TEXT,
                    change_method TEXT NOT NULL
                        CHECK (change_method IN ('bulk', 'member_page', 'phone_or_paper', 'unknown')),
                    address_status TEXT NOT NULL
                        CHECK (address_status IN ('old', 'changed', 'unknown')),
                    source TEXT NOT NULL,
                    note TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE INDEX services_address_status ON services (address_status);
                PRAGMA user_version = {SCHEMA_VERSION};
                COMMIT;"
            ))?,
            SCHEMA_VERSION => {}
            other => {
                tracing::error!(version = other, "database schema is newer than this server");
                return Err(Error::Unavailable);
            }
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    // ponytail: substring LIKE is a full scan, well under a millisecond for thousands of
    // services; switch to an FTS5 trigram index if the ledger reaches ~100k rows.
    pub fn search(&self, filter: &Filter) -> Result<SearchResult, Error> {
        let pattern = filter
            .query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
            .map(like_pattern);
        let method = filter.change_method.map(ChangeMethod::as_str);
        let status = filter.address_status.map(AddressStatus::as_str);
        let matches = "(?1 IS NULL OR name LIKE ?1 ESCAPE '\\' OR website_url LIKE ?1 ESCAPE '\\' \
            OR address_change_url LIKE ?1 ESCAPE '\\' OR source LIKE ?1 ESCAPE '\\' \
            OR note LIKE ?1 ESCAPE '\\') AND (?2 IS NULL OR change_method = ?2)";
        let connection = self.lock();
        let services = connection
            .prepare_cached(&format!(
                "SELECT {COLUMNS} FROM services WHERE {matches} \
                 AND (?3 IS NULL OR address_status = ?3) ORDER BY name COLLATE NOCASE, id"
            ))?
            .query_map(params![pattern, method, status], service)?
            .collect::<Result<Vec<_>, _>>()?;
        let mut counts: BTreeMap<&'static str, i64> = [
            AddressStatus::Old,
            AddressStatus::Changed,
            AddressStatus::Unknown,
        ]
        .into_iter()
        .map(|s| (s.as_str(), 0))
        .collect();
        let mut statement = connection.prepare_cached(&format!(
            "SELECT address_status, count(*) FROM services WHERE {matches} GROUP BY address_status"
        ))?;
        let mut rows = statement.query(params![pattern, method])?;
        while let Some(row) = rows.next()? {
            let status = AddressStatus::parse(&row.get::<_, String>(0)?).unwrap_or_default();
            counts.insert(status.as_str(), row.get(1)?);
        }
        Ok(SearchResult { services, counts })
    }

    pub fn get(&self, id: i64) -> Result<Service, Error> {
        get(&self.lock(), id)
    }

    pub fn create(&self, fields: &Fields) -> Result<Service, Error> {
        let mut connection = self.lock();
        let tx = connection.transaction()?;
        if let Some(existing) = duplicate(&tx, fields, None)? {
            return Err(Error::Duplicate(existing));
        }
        let id = insert(&tx, fields)?;
        let created = get(&tx, id)?;
        tx.commit()?;
        Ok(created)
    }

    pub fn update(&self, id: i64, patch: Patch) -> Result<Service, Error> {
        let mut connection = self.lock();
        let tx = connection.transaction()?;
        let current = get(&tx, id)?;
        let fields = patch.apply(&current.fields).map_err(Error::Invalid)?;
        if let Some(existing) = duplicate(&tx, &fields, Some(id))? {
            return Err(Error::Duplicate(existing));
        }
        tx.execute(
            &format!(
                "UPDATE services SET name = ?2, website_url = ?3, address_change_url = ?4, \
                 change_method = ?5, address_status = ?6, source = ?7, note = ?8, updated_at = {NOW} \
                 WHERE id = ?1"
            ),
            params![
                id,
                fields.name,
                fields.website_url,
                fields.address_change_url,
                fields.change_method.as_str(),
                fields.address_status.as_str(),
                fields.source,
                fields.note,
            ],
        )?;
        let updated = get(&tx, id)?;
        tx.commit()?;
        Ok(updated)
    }

    pub fn delete(&self, id: i64) -> Result<(), Error> {
        match self
            .lock()
            .execute("DELETE FROM services WHERE id = ?1", params![id])?
        {
            0 => Err(Error::NotFound),
            _ => Ok(()),
        }
    }

    /// Create every new service from a vault listing in one transaction.
    pub fn import(&self, candidates: Vec<Candidate>) -> Result<ImportResult, Error> {
        let mut connection = self.lock();
        let tx = connection.transaction()?;
        let existing = identities(&tx)?;
        let (create, duplicates) = bitwarden::plan(
            existing.iter().map(|(_, n, u)| (n.as_str(), u.as_deref())),
            candidates,
        );
        let mut result = ImportResult {
            created: 0,
            duplicates,
            skipped: 0,
        };
        for candidate in create {
            let fields = Fields {
                name: candidate.name,
                website_url: candidate.website_url,
                address_change_url: None,
                change_method: ChangeMethod::Unknown,
                address_status: AddressStatus::Unknown,
                source: IMPORT_SOURCE.into(),
                note: String::new(),
            }
            .validate();
            match fields {
                Ok(fields) => {
                    insert(&tx, &fields)?;
                    result.created += 1;
                }
                Err(_) => result.skipped += 1,
            }
        }
        tx.commit()?;
        Ok(result)
    }
}

fn service(row: &Row<'_>) -> rusqlite::Result<Service> {
    // CHECK constraints keep both enums to their known values.
    Ok(Service {
        id: row.get(0)?,
        fields: Fields {
            name: row.get(1)?,
            website_url: row.get(2)?,
            address_change_url: row.get(3)?,
            change_method: ChangeMethod::parse(&row.get::<_, String>(4)?).unwrap_or_default(),
            address_status: AddressStatus::parse(&row.get::<_, String>(5)?).unwrap_or_default(),
            source: row.get(6)?,
            note: row.get(7)?,
        },
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn get(connection: &Connection, id: i64) -> Result<Service, Error> {
    connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM services WHERE id = ?1"),
            params![id],
            service,
        )
        .optional()?
        .ok_or(Error::NotFound)
}

fn insert(connection: &Connection, fields: &Fields) -> Result<i64, Error> {
    connection.execute(
        &format!(
            "INSERT INTO services (name, website_url, address_change_url, change_method, \
             address_status, source, note, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, {NOW}, {NOW})"
        ),
        params![
            fields.name,
            fields.website_url,
            fields.address_change_url,
            fields.change_method.as_str(),
            fields.address_status.as_str(),
            fields.source,
            fields.note,
        ],
    )?;
    Ok(connection.last_insert_rowid())
}

type Identity = (i64, String, Option<String>);

fn identities(connection: &Connection) -> Result<Vec<Identity>, Error> {
    Ok(connection
        .prepare_cached("SELECT id, name, website_url FROM services ORDER BY id")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?)
}

// ponytail: compares against every row; fine for a personal ledger of thousands.
fn duplicate(
    connection: &Connection,
    fields: &Fields,
    except: Option<i64>,
) -> Result<Option<i64>, Error> {
    Ok(identities(connection)?
        .into_iter()
        .filter(|(id, _, _)| Some(*id) != except)
        .find(|(_, name, url)| {
            same_service(
                &fields.name,
                fields.website_url.as_deref(),
                name,
                url.as_deref(),
            )
        })
        .map(|(id, _, _)| id))
}
