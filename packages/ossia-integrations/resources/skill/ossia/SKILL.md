---
name: ossia
description: Create, browse, edit, render, version, export, delete, and recover scores in a local Ossia library.
---

# Ossia

Work with the local Ossia score library through MCP or CLI.

## Read the manual first

Before performing score operations, call `ossia_guide` and read its `text`.
Call it again with `document` set to each relevant linked path to read that
document. The guide defines workflows, parameters, editing rules, and supported
formats.

## CLI fallback

If MCP is unavailable, get the guide path through this instance's CLI:

{{OSSIA_CLI_GUIDE}}

Keep the executable path and global options from this command for subsequent CLI
operations. The entry is published in the data directory when Ossia starts.

## Capability index

- Backend, documentation health and diagnostics: `ossia_status` (CLI `doctor` for offline resources).
- CLI operation schemas: `ossia schema <command>`, or `schema <command>` through the published CLI.
- Immutable version/job music evidence and measure filters: `ossia_evidence`.
- Offline LilyPond syntax reference: `ossia_docs`.
- Inspect or repair converted candidates without publishing: `ossia_convert`.
- Library, title metadata, score creation, and import: `ossia_list`,
  `ossia_update_metadata`, `ossia_import`.
- Score link and selection context: `ossia_context`.
- Source reading and editing: `ossia_read_source`, `ossia_edit`, `ossia_write_source`.
- Validation, optional reference comparison, and version creation: `ossia_check`, `ossia_commit`.
- Page, system, staff and source-measure structure: `ossia_layout`.
- Output files from job or version snapshots: `ossia_output`.
- Fixed-snapshot musical evidence: `ossia_job` with `section`.
- Version history and restoration: `ossia_history`, `ossia_restore`.
- Recoverable score deletion: `ossia_delete`, `ossia_trash`, `ossia_untrash`,
  `ossia_purge`.
- Background job status and cancellation: `ossia_job`.
