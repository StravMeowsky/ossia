# Project architecture

Ossia is a local-first score editor. Rust owns the core; Vite + React owns the
browser score page; Tauri owns the management window.
`packages/desktop/src-tauri` is the shipping app entry: it dispatches CLI
subcommands without creating a Tauri window, and starts the GUI when no
subcommand is given.

- `packages/ossia-types`: shared DTOs, typed identifiers, error codes and
  protocol/schema versions.
- `packages/ossia-score`: pure in-memory analysis. It must not use files, a
  database, the network or LilyPond processes, and must not depend on Tauri.
- `packages/ossia-engine`: bundled LilyPond toolchain, conversion/compilation,
  process supervision and render artifacts.
- `packages/ossia-library`: SQLite, content-addressed snapshots, the public
  `score.ly`, edit ownership, versions and recovery.
- `packages/ossia-backend`: single-instance local backend, IPC, HTTP, jobs,
  file watching, document contexts/events, CLI and MCP.
- `packages/ossia-integrations`: Agent host detection and integration lifecycle;
  it stays independent of the backend's HTTP/IPC implementation.
- `packages/ui`: score SPA (`scores.html`) and separate management entry
  (`main.html`); it owns display and interaction, not parsing or version state.
- `packages/ossia-integrations/resources`: embedded Skill, score templates,
  icon and DSH resources. Host adapters construct their plugin configuration.
- `packages/ossia-backend/resources/guide.md` and `resources/guide/`: embedded
  Agent workflow guide and topic documents; the integration Skill is its index.

Allowed dependency direction:
`ossia-types` → `ossia-score` → `ossia-engine` → `ossia-library` →
`ossia-backend`; each crate in that chain may depend only on crates before it.
`ossia-integrations` depends on `ossia-types` and is used by the backend and
desktop. One backend instance owns each data directory.

The score page talks to the backend over HTTP, which listens on loopback by
default and can be configured for LAN access. CLI operations and MCP call the
same backend operations over IPC and start the resident backend on demand.
Closing a page or MCP connection does not stop compilation. The browser score
page has no Tauri privileges; only the `management` window does.
PDF and image recognition can send source files to a configured remote OMR
service; the desktop updater also accesses the network.

# Cross-layer contracts

- Rust owns shared frontend types. Edit definitions in `packages/ossia-types`,
  register new exports in `packages/ossia-types/src/bin/generate-types.rs`, then
  run `pnpm generate:types`; never edit
  `packages/ui/src/generated/protocol.ts` by hand.
- MCP tool schemas are generated from `ossia_backend::mcp::tools()`; run
  `pnpm generate:mcp`; never edit
  `tests/fixtures/contracts/mcp-tools.json` by hand.
- A Tauri command needs a function in
  `packages/desktop/src-tauri/src/lib.rs`, an entry in
  `tauri::generate_handler!`, the same command in `AppManifest::commands` in
  `packages/desktop/src-tauri/build.rs`, and an `allow-<command>` grant in
  `packages/desktop/src-tauri/capabilities/default.json`. Keep it scoped to the
  `management` window.

# Required reading

- Before changing code, follow [docs/coding.md](docs/coding.md).
- Before changing the updater, follow [docs/self-update.md](docs/self-update.md).
- When writing a commit message, follow
  [.github/COMMIT_MESSAGE_INSTRUCTIONS.md](.github/COMMIT_MESSAGE_INSTRUCTIONS.md).
