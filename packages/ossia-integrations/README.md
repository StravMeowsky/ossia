# ossia-integrations

This crate detects supported Agent hosts and manages the files and host configuration needed to connect them to a particular Ossia home. It depends on `ossia-types` and does not open the score library or implement backend IPC.

## Adapters and templates

[`src/registry.rs`](src/registry.rs) registers Claude Desktop, Claude Code, Codex, DeepSeek Harness and dynamically detected WorkBuddy instances. Each implements the lifecycle in [`src/agent.rs`](src/agent.rs): detection, inspection, installation, synchronization and removal. Add a host through this boundary so host-specific behavior stays outside the backend.

[`src/resources.rs`](src/resources.rs) embeds the Skill, score templates, icon and DSH resources from `resources/`. The installed Skill contains a generated CLI guide command for the selected home; MCP launches the Ossia executable with `mcp --home <home> --port <port>`. The full Agent workflow is owned by the backend's runtime guide.

## Ownership and shared paths

`Integrator` keeps ownership records under the selected home's `integrations/managed.json`. Its default host integration root is `<home>/integrations/hosts`; `OSSIA_HOST_INTEGRATION_ROOT` can select an absolute alternative. Shared-root and per-home locks protect mutations. File hashes and host state determine whether a managed entry can be updated or removed. Conflicts are surfaced, and a caller must explicitly request overwrite before an existing host configuration is replaced.

Claude uses Desktop MCP configuration and a user-scoped Claude Code plugin. When enabled, synchronization adds newly detected Claude hosts. Quit and restart Desktop, or reload the Code plugin or start a new session after changes. Desktop hosts without a file reader can use existing scores and read the guide through MCP; creating source files and viewing exported images still require host file tools.

WorkBuddy hosts are discovered per configuration root and installed through the bundled `codebuddy` plugin CLI. `sync_managed` refreshes detected, managed integrations.

## Checks

```sh
cargo test -p ossia-integrations
```

`tests/native_hosts.rs` provides ignored tests for real host CLIs and temporary host directories; host session reload behavior must be checked in the host itself.
