# ossia-engine

The engine executes the prepared LilyPond toolchain and produces conversion reports and immutable render artifacts. It is synchronous: callers provide the source snapshot, artifact and scratch locations, deadline and cancellation context. Queueing and version decisions belong to higher layers.

## Resource and process boundary

`Engine::verify` checks the locked target, resource inventory and hashes, then probes the native tools. The resource root is supplied explicitly; [`src/process.rs`](src/process.rs) runs tools in isolated scratch directories with controlled environment and process-tree cleanup. A `RunContext` supplies one operation-wide deadline and cancellation signal, including time spent waiting for a cache lock.

Source validation limits file dependencies to supported engine resources. LilyPond source can execute Scheme, so this validation is a dependency boundary, not a sandbox for untrusted input.

## Conversion and compilation

[`src/lib.rs`](src/lib.rs) converts LilyPond, MusicXML, compressed MusicXML and MIDI to LilyPond. The `ImportReport` can contain readable candidate source even when publication is refused or requires confirmation. A confirmation hash binds the input, candidate, engine build and loss report. PDF and image recognition is handled by the backend's OMR adapter, not this crate.

`Engine::compile` compiles an immutable `SourceSnapshot` into requested SVG, PDF and MIDI formats. Each format can fail independently and contribute a `formatErrors` entry; cancellation and deadline failures abort the operation. `render_wav` produces WAV from a playback timeline and an explicit audio resource directory. Configure an `Engine` with `with_audio_root` before using it as a library backend for WAV export.

## Source mapping and artifact identity

Interactive SVG compilation uses [`src/interaction.ily`](src/interaction.ily) and derived source to attach source origins, graphic ownership and geometry. Original source bytes remain authoritative: inserted derivative text has no original source position. Static exports can omit the interactive instrumentation.

The engine cache key combines source hash, engine build, render options and format. Assets are published before the manifest and protected by per-key locks; cached artifacts are checked before reuse. This identity is distinct from the library render key, which additionally includes the document ID. See [`src/cache.rs`](src/cache.rs) and [`ossia-library/src/artifacts.rs`](../ossia-library/src/artifacts.rs).

## Checks

```sh
pnpm engine:verify
cargo test -p ossia-engine
```

Native integration tests are explicitly ignored by default and require a prepared engine root. Resource recipes and target locks are maintained under [`vendor/`](../../vendor/).
