# Evidence and checks

For the edit workflow, see [Create and edit](editing.md). For rendered structure, see [Rendering and output](output.md).

## Immutable evidence and reference checks

Use `ossia_evidence` when you need to inspect an existing version or completed
job without editing it. It accepts `{"kind":"version","versionId":"…"}` or
`{"kind":"job","jobId":"…"}`, plus `section` (`summary`, `performance` or
`source`), `offset`, `limit`, `waitMs` and optional music filters. The first
request may create a durable evidence job; later requests reuse the report for
the same source, engine build and analysis version. Evidence never creates a
version or changes the current source. Music events appear only in the bounded
`page`; `job.result` is null.

`measureRange` is an inclusive one-based range. It filters both source and
performance notes by their own measure number. `voice` filters source events;
`track` and `channel` filter performance notes. Invalid ranges and incompatible
filters fail explicitly. Simultaneous source branches use stable names such as
`left[1]` and `left[2]` in source order.

Source measure numbers come from explicit bar boundaries (`|` or `\bar`) and
the end of a simultaneous branch. Performance measure numbers come from MIDI
time-signature events. A source score with repeats, a pickup, an unsupported
meter map or ambiguous timing may expose `measure: null`; never treat a missing
number as measure one.

`pitchTokens` preserve the literal source note tokens, including duration and
articulation text. `pitches` are the sounding MIDI pitches predicted from the
source. Performance notes use `pitchName`, which is a canonical sharp spelling;
it cannot distinguish enharmonic spellings such as F♯ and G♭. Use source
`pitchTokens` when spelling matters.

Source evidence can be partial when the LilyPond input uses constructs outside
the verified analysis subset. `\transpose`, `\relative`, `\tempo` and
simultaneous `<< … \\ … >>` voices are handled; common non-pitch `\set`
assignments are ignored. Chord mode, repeats with unresolved expansion, semantic
includes and opaque Scheme remain conservative. A partial source report does
not make performed-note evidence unavailable.

Import and check accept either `expect` or `reference`, never both:

```json
{
  "reference": {
    "source": { "kind": "version", "versionId": "…" },
    "comparison": "performance"
  }
}
```

`performance` compares the multiset of performed notes by pitch, quarter-note
start and duration; track order, channel and MIDI file order are ignored.
`tracks` also includes track and channel. Differences report missing and extra
notes without index-based cascades. Missing start or duration makes the result
`unavailable`; it never silently passes. A reference comparison proves performed
equivalence, not identical notation, staff assignment or engraving.

`reference` is an optional oracle, not a default verification step. Use it only
when the user names a reference edition, supplies a reference file, or an
existing library version is the intended base. Do not search for or import a
public-domain reference merely to validate an ordinary new score. If the user
does provide a reference file, import it once as a normal document and use its
`versionId`; there is no direct external-file reference API.

Use `measureRange` on source or performance evidence to keep the answer small:

```json
{
  "source": { "kind": "version", "versionId": "…" },
  "section": "performance",
  "measureRange": { "start": 17, "end": 17 },
  "limit": 64
}
```

## Musical checks

Explicit import/check produces SVG, source evidence and MIDI evidence when
available. Automatic previews produce SVG only. Written note tokens and performed
MIDI note-ons are different counts: repeats, ties and unused definitions matter.
`music.source` and `music.performance` report coverage separately. Unknown counts
are null, not a verified zero. MIDI time uses `{numerator, denominator}` fractions
of a quarter note; duration is key-release time and excludes pedal acoustics.

For the single-staff scaffold in [Create and edit](editing.md), an exact expectation could be:

```json
{
  "expect": {
    "noteCount": 8,
    "tracks": [{ "track": 1, "channel": 0, "pitches": [60, 62, 64, 65, 67, 69, 71, 72] }]
  }
}
```

These values belong to the example; use the requested pitches, timing, track and
channel for the actual score. Read actual track/channel identifiers from the
report. Track expectations also
accept `notes` with `pitch`, `start` and `duration` fractions. Simultaneous notes
sort by pitch. Missing timing, SMPTE and format-2 cannot pass timing conditions.
Empty expectations and zero denominators are invalid.

A check result contains `render`, `music`, and `verification`. Compilation success
and intent verification are independent. A failed/unavailable expectation keeps
preview evidence but authorizes no commit. The latest explicit check supersedes
older checks; automatic previews never authorize a commit. Do not drop an
expectation just to make a check pass. For an older job without recorded evidence,
call `ossia_evidence` with its `jobId`; do not treat missing evidence as success.

## Reviewing an existing version

1. Use `ossia_history` to choose an immutable `versionId`.
2. Call `ossia_evidence` with `section:"summary"` to inspect source and performance
   coverage and counts.
3. Read `source` or `performance` pages when pitches, spelling, timing or measure
   placement matter. Use `measureRange` and page with `offset` and `nextOffset`.
4. If the user explicitly wants to match this version, compare a draft or import
   by passing it as `reference` to `ossia_check` or `ossia_import`. Do not compare
   to it merely because it exists.
5. If the version should become the current editing base, restore it using
   [History and deletion](library.md), then follow the edit workflow. Restoration
   creates a new version. If it already matches the request, report the result.

Use image inspection for readability, collisions, spacing and visual structure.
It is not authoritative evidence for pitch or octave.
