# Verification

Run all verification suites without a library or server:

```sh
pnpm verify
```

To run only the playback cases, optionally selecting specific cases:

```sh
pnpm verify:playback
pnpm verify:playback xinhu-ledger bittersweet-ties
pnpm verify:audition
pnpm verify:audition sor-original-locations
```

## Playback fixtures

Each case in `tests/fixtures/playback/cases.json` references a LilyPond source
derived from a reported issue. Prefer a complete score with personal metadata
removed, and verify that redaction preserves the relevant behavior. Use small
independent excerpts for distinct musical rules or isolated diagnosis. A passing
case verifies its declared checkpoints, not every note in the score.
The command builds the current engine,
renders each source with the installed LilyPond, and checks sounding notes,
their source-note associations and selected playhead positions. It exits with a
failure if any expected behavior is missing. Some cases intentionally expose
unfixed bugs; they must not be changed to expect the incorrect result. The
checks describe the intended result and are not copied from current output.
CI installs the locked LilyPond engine on macOS and runs `pnpm verify`. Known
failures cause the job to fail until fixed.

To add a case, add a self-contained `.ly` excerpt and an entry in `cases.json`.
Confirm the case fails for the reported problem before applying its fix; retain
the original score where necessary to reproduce the issue.
For `mapped` or `playhead` checks, specify `time` in seconds, MIDI `pitch`, and
a source `needle` that occurs exactly once. `offset` is the character offset
within that needle. Optional `continuation` and `continuationOffset` require
both ends of a tie to be in the same group; `duration` checks the sounding
duration. A `forward` check verifies that positions exist and do not move
backwards within a time range. Use `playhead` only when the chosen notehead is
expected to intersect the line at that time; simultaneous voices may choose
another valid notehead. Keep browser tests for interactions, scheduling, and
DOM rendering that the fixture runner does not exercise.

## Individual audition

`pnpm verify:audition` renders every case in `cases.json`.
Explicit case IDs select a subset. Complete scores cover the local library's
piano, guitar, ukulele, violin and ensemble examples; focused excerpts retain
checks for individual musical constructs. It checks every visible notehead for a
clickable hit region, one timed single-pitch group, a matching sounding attack
and reachability through the page's ordinary arrow-key navigation. Tied
continuations may share their attack's group. Timed groups with no pitches are
intentionally silent notation, such as dead notes; they must remain clickable
and keyboard-reachable, and are counted separately. Coincident unisons are counted
separately when they occupy identical bounds and produce the same pitch and
onset.

The command also runs Chrome against the generated SVG, geometry and playback
data. On the first system of the first page, it clicks each notehead, walks
each voice forwards and backwards, and checks Alt/Option-click against the
simultaneous MIDI attacks. It records calls to the real audio player while
allowing playback to run, including duplicate-attack detection across notation
and TAB. This checks scheduled pitches, not the sound heard through speakers.
Install Chrome with `pnpm --filter @ossia/ui exec playwright install chrome`.

To audit the current library without changing scores:

```sh
pnpm audit:audition
pnpm audit:audition -- <document-id>
pnpm audit:audition -- --base=http://127.0.0.1:3211/api/v1/
pnpm audit:audition -- --input=path/to/playback-fixture.json
```

The JSON report includes every failed notehead's page, source location and
reason. Any finding or failed request makes the command fail. This data audit
uses the page's hit testing and navigation helpers; it does not run a browser.
Rebuild and restart the backend after Rust changes. Engraved event metadata
requires a render from the current engine.

## Playback audit

Start the development backend from the current checkout, then run:

```sh
pnpm audit:playback > .ossia/playback-audit-report.json
pnpm audit:playback -- <document-id>
pnpm audit:playback -- --base=http://127.0.0.1:3211/api/v1/ --strict
pnpm audit:playback -- --expect=path/to/checked-onsets.json --strict
```

The command reads the current render of each score through the local HTTP API;
it does not modify scores. The backend must be rebuilt and restarted after Rust
changes. The JSON report records the title, onset coverage, source locations,
coordinates and times. `--strict` exits unsuccessfully for any issue or untimed
group; without it only failed requests or incompatible snapshots fail the run.
`trace` records the selected position at each group onset and the held position
when locating fails. Compare it with independently checked score examples when
validating a mapping change.

For independently checked examples, `--expect` accepts a JSON object keyed by
document ID. Each value has the exact `sourceHash` and a `checkpoints` array of
`{ "time": 0, "page": 1, "system": 0, "sourceByteStart": 123 }` objects.
Times are seconds; source offsets are zero-based UTF-8 bytes. The expected
source offset must belong to the selected note at that onset. A stale source or
an omitted expected document is an error; a mismatch is an issue. Expectations
must be established from independent evidence, not copied from the audit trace.

- `unlocated`: the same locator used by the page cannot find one unambiguous
  position for the chosen group; playback holds the previous position.
- `backward`: the chosen position moved backwards in score order. A repeat,
  alternate ending, grace note or polyphony may make this legitimate; inspect
  the score and timing before calling it a bug.
- `unmapped-note`: a sounding note has no group with the same pitch and onset.
- `unmapped-head`: a visible notehead belongs to no playback group. Silent
  notation, ornaments and unsupported notation need individual review.
- `multiple-positions`: one group contains noteheads at different positions.
  Ties and displaced chord noteheads commonly do this correctly; inspect the
  source and the note-to-time correspondence before treating it as an error.

These are diagnostic checks, not a musical oracle. A pitch/onset match does not
prove that a note was matched to the correct occurrence, nor that MIDI matches
the intended score. Simultaneous voices need not all move the playhead. The
audit does not exercise the browser, audio output, seeking, repeats or the
visible animation in real time. Verify confirmed failures with independent
score or MIDI evidence and add a focused regression test before fixing them.
