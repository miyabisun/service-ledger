# service-ledger

A small ledger of the services you contract or use. It answers one question after a move:
which services still have my old address. Each record keeps the service name, its website
and the page where the address is changed. It also records how the change is done, whether
the registered address is old, changed or unknown, where the record came from, and a note.

The server does CRUD and search only. AI agents and other apps register services through
the HTTP API or MCP, and a Bitwarden vault listing can be imported. The browser UI lists,
searches, filters by address status, opens the address-change page, and edits or deletes.

Records live in one SQLite file. The ledger holds personal data, so never commit a database
to a repository; tests and examples use fictional services only.

## Run with Docker

Requires Docker. Each tagged release publishes `ghcr.io/miyabisun/service-ledger` with its
version as the tag (for example `0.1.0`) and `latest`; see [Releases](#releases).

```sh
docker volume create service-ledger-data
docker run -d --name service-ledger \
  -p 127.0.0.1:3000:3000 \
  -v service-ledger-data:/app/data \
  ghcr.io/miyabisun/service-ledger:latest
```

Open <http://127.0.0.1:3000>. The ledger starts empty; register services with the API, MCP
or an import (below). `GET /healthz` returns `ok` for health checks.

The container runs as uid/gid `10001` and writes only below `/app/data`.

## Configuration

All settings are optional and read at startup.

| Variable | Default | Purpose |
| --- | --- | --- |
| `PORT` | `3000` | TCP port from `1` to `65535`, listened on `0.0.0.0`. Invalid values fail startup. |
| `DB_PATH` | `data/service-ledger.db` (image: `/app/data/service-ledger.db`) | SQLite database file. Its directory is created when missing. |
| `LOG_LEVEL` | `info` | Exactly `off`, `error`, `warn`, `info`, `debug` or `trace`; anything else means `info`. |

The server has no authentication of its own. Expose it only on a trusted network such as
a home LAN or Tailscale, and control reachability with the published port or a proxy.

## Data and backup

- The database is a single SQLite file in **WAL mode**. While the server runs, SQLite keeps
  `service-ledger.db-wal` and `service-ledger.db-shm` beside it; they are part of the live
  database, so mount the whole directory, not just the file.
- The file is created with mode `0600` (WAL/SHM follow it).
- A consistent copy can be taken while the server runs with
  `sqlite3 /app/data/service-ledger.db "VACUUM INTO '/backup/service-ledger.db'"`. Because
  of WAL, the backup process needs read-write access to the directory.
- To restore, stop the server, replace `service-ledger.db` with the copy (after
  `gunzip` if compressed) and delete any leftover `-wal`/`-shm` files, then start it again.
- The schema version is stored in `PRAGMA user_version`. A server refuses to open a database
  written by a newer schema instead of modifying it.

## HTTP API

JSON in and out under `/api`. Errors are `{"error": "...", "code": "..."}` with codes
`invalid` (400), `not_found` (404), `duplicate` (409, plus `existing_id`) and `unavailable`
(503).

| Method and path | Purpose |
| --- | --- |
| `GET /api/services?q=&address_status=&change_method=` | Search. `q` is a case-insensitive substring of name, URLs, source and note. Returns `{services, counts}`; `counts` has matches per address status for the same `q` and method. |
| `POST /api/services` | Register a service; returns `201` and the record. |
| `GET /api/services/{id}` | Read one service. |
| `PATCH /api/services/{id}` | Change the given fields only; `null` clears `website_url` or `address_change_url`. |
| `DELETE /api/services/{id}` | Delete; returns `204`. |
| `POST /api/import/bitwarden` | Import a vault listing (see below). |

A service record:

| Field | Values |
| --- | --- |
| `name` | Required, 1–200 characters, one line. |
| `website_url`, `address_change_url` | Optional absolute `http://` or `https://` URLs without credentials. |
| `change_method` | `bulk` (one-stop service), `member_page`, `phone_or_paper` or `unknown` (default). |
| `address_status` | `old`, `changed` or `unknown` (default). |
| `source` | Required: where the service was found, e.g. `card statement 2026-04`. |
| `note` | Optional multi-line text. |
| `id`, `created_at`, `updated_at` | Set by the server (UTC, RFC 3339). |

Unknown fields are rejected, so values such as passwords cannot be stored by mistake.
Two records are duplicates when their names match, ignoring case and surrounding spaces.
They are also duplicates when both websites are on the same host, ignoring `www.`.
A duplicate request fails with `409` and `existing_id`; update that record instead.

```sh
curl -X POST http://127.0.0.1:3000/api/services \
  -H 'content-type: application/json' \
  -d '{"name":"Example Mobile","website_url":"https://www.example.com/",
       "address_change_url":"https://www.example.com/account/address",
       "change_method":"member_page","address_status":"old","source":"invoice email"}'

curl 'http://127.0.0.1:3000/api/services?address_status=old'

curl -X PATCH http://127.0.0.1:3000/api/services/1 \
  -H 'content-type: application/json' -d '{"address_status":"changed"}'
```

## MCP

A Streamable HTTP MCP server is served at `http://<host>:<port>/mcp` (for example
`http://127.0.0.1:3000/mcp`). It shares the database and the rules of the HTTP API.

| Tool | Arguments |
| --- | --- |
| `service_search` | `query`, `address_status`, `change_method` (all optional), `limit` 1–200 (default 50), `offset`. Returns `services`, `counts`, `total` and `next_offset`. |
| `service_get` | `id` |
| `service_create` | The record fields above; `name` and `source` are required. |
| `service_update` | `id` and `changes` (an object of fields to change). |
| `service_delete` | `id` |

For Claude Code, for example:

```sh
claude mcp add --transport http service-ledger http://127.0.0.1:3000/mcp
```

## Import from Bitwarden

The import reads only each entry's **name** and the **origin** (`scheme://host`) of its
first `http(s)` URI. Nothing else is stored, even if the file contains it.
That excludes usernames, passwords, notes, custom fields, TOTP seeds, URL paths and queries.
Login and card entries are imported; secure notes, identities and other types are skipped.
Duplicates of existing services or of earlier entries are skipped (same rule as above).
Running the import again is therefore safe. New services start with status `unknown`,
method `unknown` and the source `Bitwarden`.

Export the list with [rbw](https://github.com/doy/rbw), which contains no passwords, and
post it:

```sh
rbw list --raw > bitwarden.json
# Optional: drop folders that are not services.
jq 'map(select(.folder != "Env Files"))' bitwarden.json > services.json
curl -X POST --data-binary @services.json http://127.0.0.1:3000/api/import/bitwarden
# {"created":3,"duplicates":2,"skipped":0}
```

The official `bw export --format json` file (`{"items": [...]}`) is accepted too.
Delete the exported file afterwards. [`tests/fixtures/rbw-list.json`](tests/fixtures/rbw-list.json)
shows the expected shape with fictional entries.

## Development

Requires Rust 1.96 (pinned by `rust-toolchain.toml`) and Node.js 24. The frontend is built
first because the Rust binary embeds `client/dist`.

```sh
npm --prefix client ci
npm --prefix client run build
cargo run
```

The server listens on <http://127.0.0.1:3000> and creates `data/service-ledger.db` (ignored
by Git). For frontend work, also run `npm --prefix client run dev` and open
<http://127.0.0.1:5173>; Vite proxies `/api` to port 3000.

Run the full verification locally before every commit:

```sh
npm --prefix client run format:check
npm --prefix client run check
npm --prefix client test
npm --prefix client run build
npm --prefix client run lint:design
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked
npx --prefix client playwright install chromium
npm --prefix client run test:e2e
docker build --tag service-ledger:test .
bash scripts/smoke-image.sh service-ledger:test
```

The browser tests start `target/debug/service-ledger` on port 5187.
They use a throwaway database in `target/e2e-data` and register fictional services.
Chromium then drives the UI at desktop and phone widths in both themes.
The image smoke test runs the container on an empty volume, registers a fictional service,
restarts it, and reads the service back.
[`DESIGN.md`](DESIGN.md) is the UI's design contract.

## Releases

GitHub Actions only builds releases; verification runs locally as above. A `vX.Y.Z` tag
that matches `Cargo.toml` builds the image, pushes `ghcr.io/miyabisun/service-ledger:X.Y.Z`
and `latest`, then creates a GitHub Release.

## License

[MIT](LICENSE)
