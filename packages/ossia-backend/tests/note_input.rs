mod common;

use common::{Fixture, SOURCE};
use ossia_backend::*;

fn args(
    _: &Fixture,
    edit: EditId,
    hash: ContentHash,
    at: &str,
    action: Option<NoteInputAction>,
) -> NoteInputArgs {
    let source = std::str::from_utf8(SOURCE).unwrap();
    NoteInputArgs {
        edit_id: edit,
        expected_source_hash: hash,
        target: source.find(at).unwrap() as u32,
        action,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn note_input_writes_the_edit_without_a_preview_render_and_returns_mei() {
    let f = Fixture::new(false).await;
    let edit = f.begin();
    let preview = f
        .runtime
        .note_input(
            f.doc.document_id,
            args(&f, edit.edit_id, f.doc.source_hash.clone(), "c'1", None),
        )
        .unwrap();
    assert_eq!(preview.source, std::str::from_utf8(SOURCE).unwrap());
    assert_eq!(preview.mei.matches("<measure ").count(), 2);

    let jobs = f.runtime.library.list_jobs().unwrap().len();
    let raised = f
        .runtime
        .note_input(
            f.doc.document_id,
            args(
                &f,
                edit.edit_id,
                preview.source_hash.clone(),
                "c'1",
                Some(NoteInputAction::Transpose { semitones: 1 }),
            ),
        )
        .unwrap();
    assert!(raised.source.contains("cis'1 | d'1"), "{}", raised.source);
    assert_eq!(
        raised.source_hash,
        ContentHash::of(raised.source.as_bytes())
    );
    assert!(raised.mei.contains(r#"accid="s""#));
    assert_eq!(f.runtime.library.list_jobs().unwrap().len(), jobs);

    // Undo puts the earlier source back.
    let undone = f
        .runtime
        .note_input(
            f.doc.document_id,
            args(
                &f,
                edit.edit_id,
                raised.source_hash,
                "c'1",
                Some(NoteInputAction::Restore {
                    source: preview.source.clone(),
                }),
            ),
        )
        .unwrap();
    assert_eq!(undone.source_hash, f.doc.source_hash);

    // A stale hash is refused.
    let stale = f.runtime.note_input(
        f.doc.document_id,
        args(
            &f,
            edit.edit_id,
            ContentHash::of(b"stale"),
            "c'1",
            Some(NoteInputAction::Delete),
        ),
    );
    assert!(stale.is_err());
    f.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn note_input_takes_back_its_own_edit_but_not_an_agents() {
    let f = Fixture::new(false).await;
    let begin = || NoteInputBeginArgs {
        base_version_id: f.doc.head_version_id,
        expected_source_hash: f.doc.source_hash.clone(),
    };
    let first = f
        .runtime
        .note_input_begin(f.doc.document_id, begin())
        .unwrap();
    // A reopened page continues the same edit instead of failing.
    let again = f
        .runtime
        .note_input_begin(f.doc.document_id, begin())
        .unwrap();
    assert_eq!(again.edit_id, first.edit_id);
    assert_eq!(again.author, NOTE_INPUT_AUTHOR);
    f.finish().await;

    let f = Fixture::new(false).await;
    f.begin();
    let busy = f
        .runtime
        .note_input_begin(
            f.doc.document_id,
            NoteInputBeginArgs {
                base_version_id: f.doc.head_version_id,
                expected_source_hash: f.doc.source_hash.clone(),
            },
        )
        .unwrap_err();
    assert_eq!(busy.code, ErrorCode::EditBusy);
    f.finish().await;
}
