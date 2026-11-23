# Import and recognition

For score creation and editing, see [Create and edit](editing.md). For verification, see [Evidence and checks](evidence.md).

## Import a score

1. Identify the local input and its format below. Call `ossia_convert` first when
   you need to inspect or repair a candidate before publishing.
2. Call `ossia_import` with the absolute path. Pass `expect` or `reference` only
   when the task establishes the expected result. Use `waitMs` or poll
   `ossia_job`; see [Operations](operations.md) for background jobs.
3. Inspect the outcome, report and any verification. A `needs-confirmation`
   report has no document: review its losses and `playbackError`. If playback is
   unavailable, tell the user the score can be viewed but not played and obtain
   their choice before proceeding. Retry the same import with `confirmation` set to the
   report's `confirmation` value. A refused input cannot be forced.
   If an expectation or reference fails, the job error includes
   `verification.differences` and actual pitches; no document is created.
4. When a document is created, inspect the rendered score as needed. Call
   `ossia_context` for its score link and include the link in the reply. For
   image and PDF setup errors, see remote score recognition below.

## Import inputs

`ossia_import` and `ossia_convert` accept one local file by absolute path. The
file must be at most 128 MiB. `format` is optional and is otherwise inferred
from the extension. `title` defaults from the filename and `author` defaults to
`"Ossia"`.

| Input                                            | Handling                                                                                                                                                                 |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `.ly`                                            | UTF-8 LilyPond source; imported directly.                                                                                                                                |
| `.musicxml`, `.xml`                              | MusicXML converted locally by `musicxml2ly`.                                                                                                                             |
| `.mxl`                                           | Compressed MusicXML converted locally; the archive must contain exactly one rootfile in `META-INF/container.xml`.                                                        |
| `.mid`, `.midi`                                  | MIDI converted locally by `midi2ly`.                                                                                                                                     |
| `.pdf`, `.png`, `.jpg`, `.jpeg`, `.tif`, `.tiff` | Transcribed by the remote OMR service configured under General → Score recognition. The service may take up to 30 minutes and must return exactly one MusicXML movement. |

`waitMs` is limited to 30 seconds. Poll `ossia_job` for long OMR work.

## Import reports and validation

Direct `.ly` imports require one consistent top-level `\version` declaration
between 2.24.0 and the bundled LilyPond version. Missing, malformed, conflicting,
older or newer declarations are refused before a score is created. To migrate an
older file, call `ossia_convert` with `migrateLilypond: true` and
`includeSource: true`. Ossia runs its bundled `convert-ly` on a temporary copy;
the original remains unchanged. Review the returned source, diagnostics, losses
and playback result. If the user wants to proceed, save the candidate as a new
UTF-8 `.ly` file without overwriting an existing file and import it through
`ossia_import`. The conversion confirmation belongs to the original input;
inspect the import report and use its own confirmation when required. Changing
only the version declaration does not migrate its syntax.
MIDI preparation is checked separately: a modern source can still have an
unsupported structure such as `\book`. In that case `playbackError` explains the
failure and confirmation imports a viewable score without playback.

Clean import means conversion succeeded, not musical or engraving correctness.
`ossia_convert` runs the same checks and conversion without publishing a
document, version or job. The original input stays under `original/input.*`,
`inputHash` is its hash, and Audiveris warnings are diagnostics rather than
blocking losses. It returns a compact `ImportReport` by default; set
`includeSource: true` to include `ImportReport.source` when readable text exists,
including refused candidates. Use it to inspect or repair generated sources
before import; import summaries still omit `source`.
By default `ossia_convert` also compiles the candidate with interactive SVG
anchors and checks MIDI. An SVG failure returns `status: "refused"` and adds the reason to
`blocking`; `source` is present only when `includeSource` is true. Pass
`verifyRender: false` only for fast inspection.
A failed import keeps the converted source and format errors under `error.detail`.
OMR transcriptions are retained under `imports/omr/<inputHash>.musicxml` and
reported as `omrSource`. Audiveris page layout and breaks are not copied into
the converted source.
A null report `measures` count means unavailable; read `measureCountReason`.
`noteEvents` counts written note tokens, not performed MIDI notes. MIDI/token
differences are diagnostics, not proof of musical intent; an unavailable
difference result is not proof of no change.

Inspect the rendered score for beams, clefs, spacing, and collisions. Successful
compilation and MIDI checks alone cannot establish readability or fidelity to a
reference.

## Remote score recognition

PDF, PNG, JPEG and TIFF imports require a remote OMR service. Configure it in
the Ossia desktop app under **Settings → General → Score recognition**; MCP and
CLI do not expose a settings command. The settings apply to the next import
without restarting the backend.

1. For the built-in service, select **Ossia OMR** and leave the service address
   blank. Enter the **Access key** (访问密钥), then select **Save**. A masked key
   confirms that it was stored; there is no sign-in or connection-test button.
2. For a self-hosted service, select **Custom**, enter its base URL and bearer
   token, then select **Save**. The URL must use HTTPS (HTTP is accepted only
   for loopback), with no query, fragment or trailing slash. Its access key is
   the value configured on that service as `OSSIA_OMR_TOKEN`.

The built-in and custom services store access keys separately; switching the
service does not reuse the other service's key. Access keys are one-line
secrets: do not ask the user to send one in chat or pass it on a command line.

`Remote OMR token is not configured` means the selected service has no stored
access key. It does not mean that the service is unreachable or that its URL is
wrong. Tell the user:

> PDF and image imports need an access key. Open the Ossia desktop app and go to
> **Settings → General → Score recognition**. Select **Ossia OMR**, or **Custom**
> and its service address, enter the matching **Access key**, then click
> **Save**. MCP and CLI cannot configure this credential. Tell me once it is
> saved and I will retry the same import.

Do not suggest an authorization or login button, probe the endpoint with a
guessed key, or edit `preferences.json` directly. If the error is
`Custom remote OMR URL is not configured`, the selected custom service has no
address; direct the user to the same settings page to fill in **Service
address**.
