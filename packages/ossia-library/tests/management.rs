use ossia_engine::{CompileOptions, RunContext};
use ossia_library::*;
use std::{fs, path::Path};

struct TestEngine;

impl EngineBackend for TestEngine {
    fn build_id(&self) -> &str {
        "management-test-engine"
    }

    fn convert(
        &self,
        input: &[u8],
        _: InputFormat,
        _: Option<&ContentHash>,
        context: &RunContext,
    ) -> Result<ImportReport> {
        context.check()?;
        Ok(ImportReport {
            status: ImportStatus::Clean,
            input_hash: ContentHash::of(input),
            engine_build_id: self.build_id().into(),
            omr_source: None,
            lilypond_version: None,
            playback_error: None,
            source_hash: Some(ContentHash::of(input)),
            source: Some(String::from_utf8(input.to_vec()).unwrap()),
            confirmation: None,
            confirmed: false,
            measures: Some(1),
            measure_count_reason: None,
            voices: 1,
            note_events: 1,
            losses: vec![],
            blocking: vec![],
            diagnostics: vec![],
        })
    }

    fn compile(
        &self,
        snapshot: &SourceSnapshot,
        options: &CompileOptions,
        artifact_root: &Path,
        context: &RunContext,
    ) -> Result<RenderManifest> {
        context.check()?;
        let bytes = format!("<svg>{}</svg>", snapshot.source_hash()).into_bytes();
        let hash = ContentHash::of(&bytes);
        let relative = format!("assets/{hash}.svg");
        fs::create_dir_all(artifact_root.join("assets")).unwrap();
        fs::write(artifact_root.join(&relative), &bytes).unwrap();
        let midi = b"MThd\0\0\0\x06\0\0\0\x01\0\x60MTrk\0\0\0\x0c\0\x90\x3c\x40\x60\x80\x3c\x40\0\xff\x2f\0";
        let midi_hash = ContentHash::of(midi);
        let midi_path = format!("assets/{midi_hash}.midi");
        fs::write(artifact_root.join(&midi_path), midi).unwrap();
        Ok(RenderManifest {
            source_hash: snapshot.source_hash().clone(),
            engine_build_id: self.build_id().into(),
            options_hash: ContentHash::of(&serde_json::to_vec(options).unwrap()),
            pages: vec![RenderPage {
                page: 1,
                artifact: ArtifactRef {
                    path: relative,
                    sha256: hash,
                    bytes: bytes.len() as u64,
                },
            }],
            pdf: None,
            midi: Some(ArtifactRef {
                path: midi_path,
                sha256: midi_hash,
                bytes: midi.len() as u64,
            }),
            diagnostics: vec![],
            format_errors: vec![],
        })
    }
}

fn import(library: &Library, title: &str, source: &[u8]) -> DocumentState {
    library
        .import(
            &TestEngine,
            ImportRequest {
                title,
                input: source,
                format: InputFormat::Ly,
                confirmation: None,
                author: "test",
            },
            &RunContext::default(),
        )
        .unwrap()
        .document
        .unwrap()
}

#[test]
fn management_pages_split_active_archived_scores_and_keep_previews_available() {
    let root = tempfile::tempdir().unwrap();
    let library = Library::open(root.path()).unwrap();
    let first = import(&library, "First", b"c'1");
    let second = import(&library, "Second", b"d'1");

    let page = library.management_score_page(false, 1, 0).unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.next_offset, Some(1));
    assert!(page.items[0].archived_at.is_none());
    assert!(!page.items[0].created_at.is_empty());
    assert!(!page.items[0].updated_at.is_empty());

    let second_page = library.management_score_page(false, 1, 1).unwrap();
    assert_eq!(second_page.items.len(), 1);
    assert_eq!(second_page.next_offset, None);
    assert_eq!(
        [page.items[0].document_id, second_page.items[0].document_id]
            .into_iter()
            .collect::<std::collections::HashSet<_>>(),
        [first.document_id, second.document_id]
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
    );

    let preview = library
        .management_score_preview(first.document_id)
        .unwrap()
        .unwrap();
    assert!(preview.starts_with(b"<svg"));

    library
        .trash(first.document_id, &first.source_hash)
        .unwrap();
    let active = library.management_score_page(false, 10, 0).unwrap();
    assert_eq!(active.total, 1);
    assert_eq!(active.items[0].document_id, second.document_id);
    let archived = library.management_score_page(true, 10, 0).unwrap();
    assert_eq!(archived.total, 1);
    assert_eq!(archived.items[0].document_id, first.document_id);
    assert!(archived.items[0].archived_at.is_some());
    assert!(library
        .management_score_preview(first.document_id)
        .unwrap()
        .is_some());

    library.purge(first.document_id).unwrap();
    assert!(library
        .management_score_preview(first.document_id)
        .unwrap()
        .is_none());
}

#[test]
fn management_preview_is_absent_without_a_last_render() {
    let root = tempfile::tempdir().unwrap();
    let library = Library::open(root.path()).unwrap();
    let document = import(&library, "No preview", b"e'1");
    let connection = rusqlite::Connection::open(root.path().join("library.sqlite")).unwrap();
    connection
        .execute(
            "UPDATE documents SET last_render=NULL WHERE document_id=?1",
            [document.document_id.to_string()],
        )
        .unwrap();
    assert!(library
        .management_score_preview(document.document_id)
        .unwrap()
        .is_none());
}
