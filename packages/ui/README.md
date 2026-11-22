# ui

React/Vite supplies two separate interfaces over the same local application. The UI owns presentation and interaction; the Rust backend owns score analysis, document state, render identity and version history.

## Entry points and privileges

`scores.html` loads [`src/score`](src/score/index.tsx), the browser score page. `main.html` loads [`src/main`](src/main/index.tsx), the Tauri management interface. Only the management window uses Tauri commands; score-page features must work in an ordinary browser through HTTP.

The score route uses `/scores?documentId=…`. [`vite.config.ts`](vite.config.ts) rewrites `/scores` to the HTML entry while preserving the query. In development Vite uses port 5173 and proxies `/api` to the backend on port 3211.

## Synchronization and render state

[`useScoreSync`](src/score/hooks/useScoreSync.ts) first fetches status and document context, then subscribes using the instance/sequence cursor. It refreshes context on relevant events and reconnects through a new snapshot. It rejects older context responses for the same instance and ignores its own selection event after applying the corresponding response.

Selection and SVG geometry come from the backend. Playback joins backend timing and rendered note groups with browser audio. `public/audio/manifest.json` describes the shared sample resources used by playback and WAV export; the packaged HTTP UI carries those files. A render's identity and source hash matter when caching or displaying results because asynchronous jobs can complete after the source changes.

## Generated types and validation

`src/generated/protocol.ts` is generated from Rust DTOs. Change and register types in `ossia-types`, then run `pnpm generate:types` from the repository root.

```sh
pnpm dev:backend
pnpm check
pnpm test:ui
pnpm test:ui:native
```

Playwright separates browser and native projects in [`playwright.config.ts`](playwright.config.ts); the native project exercises additional runtime integration.
