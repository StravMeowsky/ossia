# desktop

This package owns the shipping Tauri binary and management window. The binary is also the entry for non-GUI commands, so the backend can run without creating a window.

## Process and window ownership

[`src-tauri/src/main.rs`](src-tauri/src/main.rs) parses arguments and dispatches a subcommand to `ossia-backend` before starting Tauri. With no subcommand, the `gui` feature starts the management application. `--no-default-features` builds a CLI/service binary without the GUI feature.

The management window can invoke native operations; the score page is a browser client of the local HTTP backend. The backend owns the library and jobs, rather than a score tab or an MCP session. UI entry points are described in [`../ui`](../ui/README.md).

## Native commands

A new management command needs four coordinated registrations: its function and `tauri::generate_handler!` entry in [`src-tauri/src/lib.rs`](src-tauri/src/lib.rs), its name in `AppManifest::commands` in [`src-tauri/build.rs`](src-tauri/build.rs), and an `allow-<command>` grant in [`src-tauri/capabilities/default.json`](src-tauri/capabilities/default.json). That capability is scoped to the `management` window.

The build script also supplies a build stamp used by the updater. Follow [`docs/self-update.md`](../../docs/self-update.md) when changing update behavior.

## Development

Run from the repository root:

```sh
pnpm dev:desktop
pnpm --filter @ossia/desktop test
pnpm build:backend
```

The last command builds the UI and the non-GUI desktop binary; prepared engine resources are managed separately.
