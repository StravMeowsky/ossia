# History and deletion

For exported files, see [Rendering and output](output.md).

## History and restoration

Use `ossia_history` to page through versions. To make an older version the
current base, call `ossia_restore` with the target `versionId`, current head
`baseVersionId`, and current workspace `expectedSourceHash`. The workspace must
be clean with no active edit. Restoration queues a job and appends a new version;
wait for it to finish before editing. See [Operations](operations.md) for jobs.
To inspect or compare a version without restoring it, see
[Evidence and checks](evidence.md).

## Trash and permanent deletion

Deletion uses a separate lifecycle from version restoration. `ossia_delete`
moves a score to the recoverable trash using `documentId` and
`expectedSourceHash`. It rejects dirty sources, active or interrupted edits and
queued or running jobs. Set `force:true` only when the user asked to delete the
score and accept ending those edits and cancelling those jobs; forcing retains
the current source, including uncommitted changes. A forced delete waits up to
10 seconds for running work to stop. Retry after resolving any returned
conflict.

`ossia_trash` pages through deleted scores. `ossia_untrash` restores the original
document ID, source and complete history without creating a version or
reactivating an edit. `ossia_purge` permanently removes one score that is
already in the trash, including its versions and derived data. It does not
delete the external file originally imported into Ossia or any file already
exported elsewhere. Trashed scores remain until explicitly purged.
