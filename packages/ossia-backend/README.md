# ossia-backend

The backend coordinates a single local library, its engine and its clients. Browser HTTP, local IPC, CLI and MCP converge on the same `Runtime` operations. The executable and desktop lifecycle are owned by [`desktop`](../desktop/README.md).

## Runtime and transport boundaries

[`src/server.rs`](src/server.rs) resolves the data directory and resources, starts the runtime and publishes discovery. The library's lifetime OS lock determines ownership; the discovery record only tells clients where to connect. CLI and stdio MCP use IPC, and MCP can start the resident backend on demand. Closing a browser page or MCP connection does not terminate queued compilation.

The browser API is served by [`src/http.rs`](src/http.rs). HTTP defaults to loopback; LAN listening is configurable. Host and Origin validation restrict browser access, but LAN access is not authenticated with a bearer token: a device on the trusted network that knows a document ID can access that document. IPC uses platform-specific local access controls.

## Jobs and immutable inputs

[`src/core/jobs.rs`](src/core/jobs.rs) queues imports, checks, restores, version renders and outputs. The library persists job state and captured source snapshots; the runtime schedules work and propagates cancellation to the engine. Job results describe the captured input, which may no longer be the current source when work completes.

Only pending preview jobs for the same document are coalesced. Superseded jobs are recorded as cancelled; explicit checks remain distinct. Source watching combines filesystem notifications with polling, and context and check operations re-observe the source.

## Document synchronization

[`src/core/context.rs`](src/core/context.rs) combines persistent document state with runtime selection and render context. Context carries an instance ID and sequence; clients obtain a snapshot and then subscribe to events. The event buffer is bounded, so a stale cursor or restart requires a fresh snapshot. A source hash change clears shared selection and annotations.

The UI receives source-linked selection geometry and playback data produced by the score and engine layers. Backend render references include document ownership, so equal source in two documents does not make their public render identities interchangeable.

## Import and output routing

`RuntimeEngine` in [`src/core/engine.rs`](src/core/engine.rs) delegates normal conversion and all rendering to the local engine. For PDF and images it calls the configured remote OMR service, then converts its MusicXML locally. An OMR success is only an intermediate result: import still has conversion, validation and publication steps.

New libraries import the embedded scores under [`resources/starter/`](resources/starter/) asynchronously. The manifest is validated at startup; entries import in order, and a resumed batch skips source hashes already present in the library.

Outputs use a completed job snapshot or an immutable version. Storage cleanup is scheduled around runtime work and is delegated to the library's retention policy.

## Contracts and development

MCP tool definitions and schemas originate in [`src/mcp.rs`](src/mcp.rs); the Agent-facing guide index and linked documents are embedded from [`resources/guide.md`](resources/guide.md) and [`resources/guide/`](resources/guide/) and materialized under `<home>/docs` at startup. CLI and MCP return the index path, byte size and line count so agents can read it and follow task-relevant links. Regenerate and check contracts from the repository root:

```sh
pnpm generate:mcp
pnpm check:mcp
pnpm check:types
cargo test -p ossia-backend
```

`pnpm dev:backend` runs the development backend and Vite; its configuration is in [`scripts/backend-dev.mjs`](../../scripts/backend-dev.mjs).
