# Rendering and output

For version history, see [History and deletion](library.md). For musical evidence,
see [Evidence and checks](evidence.md). For job polling, see [Operations](operations.md).

## Source locations

Locations use UTF-8 byte spans, one-based pages/measures, zero-based staves and
SVG user units.

## Rendered structure

`ossia_layout` accepts a completed import or check `jobId` and returns
`pageCount`, `systemCount`, `staffCount`, and per-page `systems[].staves` with
bounds in SVG user units. Each system also reports `bars`, `measureStart` and
`measureEnd` when the source measure grid is reliable. Use these fields instead
of counting measures in a PNG. `measureMapping` is `complete`, `partial` or
`none`; inspect the page reason when it is not complete. `ossia_layout` does not
identify clefs, keys or time signatures; read the source and docs for those.

## Output files

Call `ossia_output` with a `source` of `{"kind":"job","jobId":…}` or
`{"kind":"version","versionId":…}` and a `format` of `ly`, `svg`, `png`, `midi`,
`pdf` or `wav`, then poll its returned job. The result includes absolute file
paths, format, size and source provenance. Use `ossia_job` with `detail:"full"`
to inspect file hashes and media types. Read output files with the host's
file/image tool. Job outputs use the captured import/check snapshot; version
outputs use the immutable version snapshot.

For PDF, PNG and SVG, `includeDrawing: true` includes the document's shared
freehand drawing as it existed when the output job was submitted. It defaults to
false. Drawings use page-relative positions, so a different score layout can
place them over different notation. Cropped SVG/PNG outputs include only the
part of the drawing inside the crop.

MIDI and WAV accept `audioRange: {"start":1,"end":4}` for an inclusive one-based
measure range. Notes overlapping the range are retained, including notes that
began before it and continue into it; notes are clipped at the range end. MIDI
preserves channel, controller, program, SysEx, tempo and time-signature events.
WAV uses the same piano playback mix as the score page and may extend beyond the
range end by its release tail. Pickup/partial measures, SMPTE timing, format-2
MIDI and unreliable time-signature boundaries reject range export.

Output accepts an absolute `outputPath` for one file or `outputDirectory` for a
batch; the two cannot be combined. When the applicable destination is omitted,
output goes to the system temporary directory, so pass one for files to keep.
`outputPath` must not already exist, and its parent directory must exist.

SVG/PNG accept a one-based `pages` array (default `[1]`). One page may use
`outputPath`; multiple pages use `outputDirectory` and return a batch. PDF, MIDI,
WAV and LilyPond do not accept `pages`. PNG defaults to a white 1600-pixel-wide
page; optional `width` is 1..4096 and output is limited to 16 million pixels. WAV
always writes 44.1 kHz stereo 16-bit PCM and accepts no quality, rate or channel
options. Generate missing formats with the same engine build or re-check first.

`crop` selects a visual region and supports SVG/PNG. Use
`{"kind":"measures","range":{"start":1,"end":4}}` or
`{"kind":"system","page":1,"systemIndex":0}`; optional `staff` selects one
zero-based staff and `paddingStaffSpaces` defaults to 2. A range crossing pages
returns one verified file per page in page order, merging matches on the same
page. A one-item `pages` list restricts a measure crop to one page; for a
system crop it must equal the crop's `page`. `outputPath` selects a crop that
resolves to exactly one page; multi-page crops publish under `outputDirectory`.
The result is an `OutputBatchOutcome` with ordered `files`; for a single-file
crop, `directory` is the file's parent. Indexed files can be downloaded or
revealed through HTTP routes ending in `/outputs/<index>` and
`/outputs/<index>/reveal`.
