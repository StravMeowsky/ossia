# ossia-omr

A separately runnable HTTP service that turns PDF and raster score images into MusicXML using Audiveris. It depends on `ossia-types`; it does not own the local score library. The local backend consumes its result and performs conversion and import locally.

See the [self-hosting guide](deploy/README.md) for installation and client configuration.

## Request and job lifecycle

[`src/api.rs`](src/api.rs) exposes public `GET /v1/health`. All other endpoints require a bearer token: `GET /v1/capabilities`, `POST /v1/jobs`, `GET /v1/jobs/{id}`, `GET /v1/jobs/{id}/result` and `POST /v1/jobs/{id}/cancel`. Submit multipart data with one JSON `metadata` field (`sourceFormat`) and one `file`. PDF, PNG, JPEG and TIFF are accepted, with a 128 MiB file limit.

[`src/queue.rs`](src/queue.rs) persists job state and input files; [`src/worker.rs`](src/worker.rs) runs a worker with process cancellation and a deadline. A result is available only for a finished job. The service accepts exactly one well-formed UTF-8 XML output; zero or multiple movements fail. This validates the output envelope, not musical accuracy.

On startup, jobs that were running are marked interrupted, queued jobs remain queued, and orphan files are removed. Terminal job data is pruned after seven days or when retained data exceeds 2 GiB; queued and running work is retained.

## Configuration

`ossia-omr serve` accepts `--home` and `--listen`. Without those flags, `OSSIA_OMR_HOME` defaults to `/var/lib/ossia-omr` and `OSSIA_OMR_LISTEN` to `127.0.0.1:3212`. `OSSIA_OMR_TOKEN` is required. `AUDIVERIS_BIN` selects Audiveris 5.11.0; `OSSIA_OMR_LANGUAGES` selects the traineddata files expected under `<home>/tessdata`. `OSSIA_OMR_MAX_JOB_SECONDS` sets the deadline. Startup validates the executable, language files and heap limit; see [`src/config.rs`](src/config.rs).

The default listen port is also used by the local development backend, so select a different port when running both on the same host.

## Checks

```sh
cargo test -p ossia-omr
```
