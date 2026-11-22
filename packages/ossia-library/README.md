# ossia-library

The library owns durable score state in one local data directory: source files, snapshots, metadata, edit ownership, versions, jobs and recoverable operations. It calls an `EngineBackend` supplied by its owner; it does not run the backend scheduler or expose a transport.

## Storage and ownership

`Library::open` holds an OS lock for its lifetime. SQLite tracks documents and metadata while `documents/<documentId>/score.ly` is the public, mutable source. Snapshot content is gzip-compressed and addressed by the SHA-256 of its uncompressed bytes. Engine artifacts live in a separate cache namespace. The database schema and its relationships are in [`src/schema.sql`](src/schema.sql).

A document's current file and its version head are distinct. Outside tools may modify the public file, producing a dirty workspace; observing a file does not create a version. Original import data and committed versions have different retention requirements from regenerable jobs and renders.

A new library carries `seeds/starter-scores.pending` until the embedded starter manifest has imported. Existing libraries are never backfilled; interrupted batches skip entries whose initial source is already present.

## Edit, check and commit

An edit capability identifies the owner of an active edit. Source writes require that capability, the expected source hash and, for range edits, the expected original text. Batched ranges must not overlap. Unknown workspace changes require explicit adoption.

A check captures the source snapshot when queued and later compiles those captured bytes, even if `score.ly` changes meanwhile. A successful check associated with the active edit records a receipt; committing requires a matching receipt, head and current source. A preview check without the edit capability does not authorize a commit. See [`src/workflow.rs`](src/workflow.rs) and [`src/recovery.rs`](src/recovery.rs).

## Crash recovery and history

File-changing operations journal an intent in SQLite, atomically replace the public file, then finalize the database state. On reopening, recovery compares the file and expected hashes: it can complete or cancel an interrupted intent, or report a conflict while preserving the file. Interrupted edits require their capability to resume; ending an edit preserves the source, while discard is a separate guarded action.

Restoring a version appends a new history entry after checking the workspace and rendering the selected snapshot. Hash checks detect observed conflicts but cannot provide compare-and-swap against an external editor that ignores this protocol.

## Derived data and maintenance

Render keys include document identity as well as source, engine build and options. Evidence is derived from immutable snapshots and cached separately from version history. [`src/storage_manager.rs`](src/storage_manager.rs) defines retention and cache cleanup; automatic maintenance targets regenerable data, not public sources or committed versions.

## Checks

```sh
cargo test -p ossia-library
```

Native integration tests are opt-in and require a prepared LilyPond root.
