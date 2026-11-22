# ossia-types

This crate defines the values exchanged by Ossia's Rust layers, browser UI, CLI, MCP server and OMR service. It owns identifiers, errors, requests, results and compatibility markers; behavior and persistence live in the consuming crates.

## Identity and source positions

[`src/lib.rs`](src/lib.rs) defines distinct UUID-backed identifiers for documents, versions, edits and jobs. They serialize as strings, but are not interchangeable in Rust. `ContentHash` validates a full lowercase SHA-256 digest.

A `SourceSnapshot` couples a document ID to immutable, validated UTF-8 bytes and their hash. The hash covers the original bytes, including BOM and line endings. `SourceSpan` ranges address UTF-8 bytes in that exact source; line and column positions count Unicode scalar values from one. Use `ossia-score::SourceMap` to validate and translate positions instead of indexing strings by character count.

## Compatibility and generation

The constants in `src/lib.rs` have separate purposes: `PROTOCOL_VERSION` identifies runtime compatibility, `SCHEMA_VERSION` identifies library storage, `INTERACTION_VERSION` identifies interactive render metadata, and `EVIDENCE_ANALYSIS_VERSION` identifies derived musical evidence. A change can affect one without affecting the others.

Rust owns the frontend contract. Add or change DTOs here, register frontend exports in [`src/bin/generate-types.rs`](src/bin/generate-types.rs), then run `pnpm generate:types` from the repository root. MCP schemas instead come from `ossia_backend::mcp::tools()`; run `pnpm generate:mcp` after changing those tool contracts. Generated files are outputs, not editing points.

## Checks

```sh
cargo test -p ossia-types
pnpm check:types
pnpm check:mcp
```
