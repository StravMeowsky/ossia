# ossia-score

`ossia-score` transforms supplied LilyPond text, SVG and MIDI bytes into in-memory musical and graphical analysis. It depends on `ossia-types`; callers own file I/O, native processes, scheduling and persistence.

## Source analysis

[`src/index`](src/index/mod.rs) indexes voices, staves and written measures, preserving source spans. The lexer and `SourceMap` retain byte-accurate positions. This is a conservative analysis of supported syntax, not execution of LilyPond: an opaque expression can yield partial capabilities and a reason rather than a fabricated location. A compressed rest may associate multiple measures with the same source span.

Measure numbering has a special case for sequential voices on a single staff; parallel voices and multi-staff scores retain local numbering when a single sequence cannot be established. An explicit staff or voice locator must resolve within that scope.

## Render geometry and selection

[`src/layout`](src/layout/mod.rs) joins numbered SVG pages to a source index; [`src/selection.rs`](src/selection.rs) derives hit geometry and selection targets from interactive render metadata. Coordinates are SVG user units. Source and layout identities must agree, and a missing page or unsupported geometry can reduce capability without inventing a nearby hit.

An element target represents a source object across its printed occurrences. Note ownership, attachments and tie links are expanded from engraving metadata; shared stems and beams can belong to several notes without making those notes one target. Geometry is therefore not a substitute for source identity.

## Musical evidence and playback

[`src/analysis.rs`](src/analysis.rs) separates source analysis from MIDI performance analysis. Written pitch tokens and performed MIDI notes answer different questions; repeats, timing and tie behavior cannot be inferred reliably from a token count. `resolve_pitches` returns a reason when the expression cannot be predicted safely.

[`src/playback.rs`](src/playback.rs) joins one render's source, layout and MIDI to create timed audio notes, audition groups and page playhead spans. [`src/midi_range.rs`](src/midi_range.rs) clips MIDI to an inclusive one-based measure range. These operations consume a matching immutable snapshot and must not silently combine artifacts from different sources.

## Checks

```sh
cargo test -p ossia-score
```

Tests under `tests/` use repository fixtures; native LilyPond cross-checks are in the engine's `score_native_crosscheck` integration test and require prepared native resources.
