use ossia_backend::*;
use ossia_engine::{CompileOptions, RunContext};
use ossia_library::{EngineBackend, ImportRequest, Library, Result};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

struct FixtureEngine {
    midi: AtomicBool,
    rendered_audio: Mutex<Vec<ScorePlayback>>,
}

impl FixtureEngine {
    fn new(midi: bool) -> Self {
        Self {
            midi: AtomicBool::new(midi),
            rendered_audio: Mutex::new(Vec::new()),
        }
    }
}

impl EngineBackend for FixtureEngine {
    fn render_wav(&self, playback: &ScorePlayback, _: &RunContext) -> Result<Vec<u8>> {
        self.rendered_audio.lock().unwrap().push(playback.clone());
        Ok(b"captured wav".to_vec())
    }
    fn build_id(&self) -> &str {
        "playback-fixture-engine"
    }

    fn convert(
        &self,
        input: &[u8],
        _: InputFormat,
        _: Option<&ContentHash>,
        context: &RunContext,
    ) -> Result<ImportReport> {
        context.check()?;
        let source = std::str::from_utf8(input)
            .map_err(|error| AppError::new(ErrorCode::InvalidEncoding, error.to_string()))?
            .to_owned();
        Ok(ImportReport {
            status: ImportStatus::Clean,
            input_hash: ContentHash::of(input),
            engine_build_id: self.build_id().into(),
            omr_source: None,
            source_hash: Some(ContentHash::of(input)),
            source: Some(source),
            confirmation: None,
            confirmed: false,
            measures: Some(1),
            measure_count_reason: None,
            voices: 1,
            note_events: 3,
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
        let svg = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/rendered/absolute/score.svg"),
        )
        .map_err(|error| AppError::new(ErrorCode::StorageError, error.to_string()))?;
        let midi = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/rendered/absolute/score.midi"),
        )
        .map_err(|error| AppError::new(ErrorCode::StorageError, error.to_string()))?;
        std::fs::create_dir_all(artifact_root.join("assets"))
            .map_err(|error| AppError::new(ErrorCode::StorageError, error.to_string()))?;
        let svg_hash = ContentHash::of(&svg);
        let midi_hash = ContentHash::of(&midi);
        let svg_path = format!("assets/{svg_hash}.svg");
        let midi_path = format!("assets/{midi_hash}.midi");
        std::fs::write(artifact_root.join(&svg_path), &svg)
            .map_err(|error| AppError::new(ErrorCode::StorageError, error.to_string()))?;
        std::fs::write(artifact_root.join(&midi_path), &midi)
            .map_err(|error| AppError::new(ErrorCode::StorageError, error.to_string()))?;
        Ok(RenderManifest {
            source_hash: snapshot.source_hash().clone(),
            engine_build_id: self.build_id().into(),
            options_hash: ossia_engine::render_options_hash(options.point_and_click),
            pages: vec![RenderPage {
                page: 1,
                artifact: ArtifactRef {
                    path: svg_path,
                    sha256: svg_hash,
                    bytes: svg.len() as u64,
                },
            }],
            pdf: None,
            midi: self.midi.load(Ordering::Acquire).then_some(ArtifactRef {
                path: midi_path,
                sha256: midi_hash,
                bytes: midi.len() as u64,
            }),
            diagnostics: vec![],
            format_errors: vec![],
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wav_job_uses_settings_captured_at_submission() {
    let root = tempfile::tempdir().unwrap();
    let library = Library::open(root.path()).unwrap();
    let source = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/rendered/absolute/score.ly"),
    )
    .unwrap();
    let engine = Arc::new(FixtureEngine::new(true));
    let document = library
        .import(
            engine.as_ref(),
            ImportRequest {
                title: "audio",
                input: &source,
                format: InputFormat::Ly,
                confirmation: None,
                author: "test",
            },
            &RunContext::default(),
        )
        .unwrap()
        .document
        .unwrap();
    let runtime = Runtime::new(library, engine.clone(), "http://127.0.0.1:3211".into());
    let key = runtime
        .context(document.document_id)
        .unwrap()
        .render
        .unwrap()
        .render_key;
    let playback = runtime.playback(&key).unwrap();
    let part = playback.notes[0].part_id.clone();
    let mut settings = runtime
        .library
        .score_settings(document.document_id)
        .unwrap();
    settings
        .instruments
        .insert(part.clone(), SampleInstrument::Guitar);
    settings = runtime
        .library
        .save_score_settings(document.document_id, settings)
        .unwrap();
    let args: OutputArgs = serde_json::from_value(serde_json::json!({
        "source": { "kind": "version", "versionId": document.head_version_id.unwrap() },
        "format": "wav", "outputPath": root.path().join("captured.wav"), "waitMs": 0,
    }))
    .unwrap();
    let job = runtime.output(args).unwrap();
    settings.instruments.insert(part, SampleInstrument::Violin);
    runtime
        .library
        .save_score_settings(document.document_id, settings)
        .unwrap();
    let scheduler = tokio::spawn(runtime.clone().schedule());
    let mut terminal = None;
    for _ in 0..100 {
        let job = runtime.library.get_job(job.job_id).unwrap();
        if matches!(job.state, JobState::Done | JobState::Error) {
            terminal = Some(job);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(
        terminal.expect("output job timed out").state,
        JobState::Done
    );
    {
        let captured = engine.rendered_audio.lock().unwrap();
        assert!(captured[0]
            .notes
            .iter()
            .all(|note| note.instrument == SampleInstrument::Guitar));
    }
    let args: OutputArgs = serde_json::from_value(serde_json::json!({
        "source": { "kind": "version", "versionId": document.head_version_id.unwrap() },
        "format": "wav", "audioRange": {"start": 1, "end": 1},
        "outputPath": root.path().join("range.wav"), "waitMs": 0,
    }))
    .unwrap();
    let range_job = runtime.output(args).unwrap();
    let mut terminal = None;
    for _ in 0..100 {
        let job = runtime.library.get_job(range_job.job_id).unwrap();
        if matches!(job.state, JobState::Done | JobState::Error) {
            terminal = Some(job);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(
        terminal.expect("range output job timed out").state,
        JobState::Done
    );
    let captured = engine.rendered_audio.lock().unwrap();
    assert!(!captured[1].notes.is_empty());
    assert!(captured[1]
        .notes
        .iter()
        .all(|note| note.instrument == SampleInstrument::Violin));
    scheduler.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires native LilyPond 2.26.0"]
async fn native_wav_export_renders_the_saved_instrument_and_measure_range() {
    let root = tempfile::tempdir().unwrap();
    let lilypond = std::path::PathBuf::from(std::env::var_os("OSSIA_TEST_LILYPOND_ROOT").unwrap());
    let audio = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui/public/audio");
    let engine = Arc::new(
        ossia_engine::Engine::verify(
            &lilypond,
            &root.path().join("scratch"),
            &RunContext::default(),
        )
        .unwrap()
        .with_audio_root(audio),
    );
    let library = Library::open(&root.path().join("library")).unwrap();
    let source = b"\\version \"2.26.0\" \\score { \\new Staff = \"solo\" { c'4 d'4 e'4 f'4 } \\layout {} \\midi {} }";
    let document = library
        .import(
            engine.as_ref(),
            ImportRequest {
                title: "audio",
                input: source,
                format: InputFormat::Ly,
                confirmation: None,
                author: "test",
            },
            &RunContext::default(),
        )
        .unwrap()
        .document
        .unwrap();
    let runtime = Runtime::new(library, engine, "http://127.0.0.1:3211".into());
    let key = runtime
        .context(document.document_id)
        .unwrap()
        .render
        .unwrap()
        .render_key;
    let part = runtime.playback(&key).unwrap().notes[0].part_id.clone();
    let mut settings = runtime
        .library
        .score_settings(document.document_id)
        .unwrap();
    settings
        .instruments
        .insert(part.clone(), SampleInstrument::Guitar);
    settings = runtime
        .library
        .save_score_settings(document.document_id, settings)
        .unwrap();
    let make_args = |file: &str, range: bool| -> OutputArgs {
        serde_json::from_value(serde_json::json!({
        "source": { "kind": "version", "versionId": document.head_version_id.unwrap() },
        "format": "wav", "audioRange": if range { Some(serde_json::json!({"start":1,"end":1})) } else { None },
        "outputPath": root.path().join(file), "waitMs": 0,
    })).unwrap()
    };
    let guitar_job = runtime.output(make_args("guitar.wav", false)).unwrap();
    settings.instruments.insert(part, SampleInstrument::Violin);
    runtime
        .library
        .save_score_settings(document.document_id, settings)
        .unwrap();
    let scheduler = tokio::spawn(runtime.clone().schedule());
    let violin_job = runtime.output(make_args("violin.wav", false)).unwrap();
    let range_job = runtime.output(make_args("range.wav", true)).unwrap();
    for job in [guitar_job, violin_job, range_job] {
        let mut terminal = None;
        for _ in 0..500 {
            let current = runtime.library.get_job(job.job_id).unwrap();
            if matches!(current.state, JobState::Done | JobState::Error) {
                terminal = Some(current);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let current = terminal.expect("native WAV job timed out");
        assert_eq!(current.state, JobState::Done, "{:?}", current.error);
    }
    let guitar = std::fs::read(root.path().join("guitar.wav")).unwrap();
    let violin = std::fs::read(root.path().join("violin.wav")).unwrap();
    let range = std::fs::read(root.path().join("range.wav")).unwrap();
    assert!(
        guitar.starts_with(b"RIFF") && violin.starts_with(b"RIFF") && range.starts_with(b"RIFF")
    );
    assert_ne!(guitar, violin);
    scheduler.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn playback_endpoint_returns_verified_timeline_for_a_recorded_render() {
    let root = tempfile::tempdir().unwrap();
    let library = Library::open(root.path()).unwrap();
    let source = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/rendered/absolute/score.ly"),
    )
    .unwrap();
    let engine = Arc::new(FixtureEngine::new(true));
    let imported = library
        .import(
            engine.as_ref(),
            ImportRequest {
                title: "playback",
                input: &source,
                format: InputFormat::Ly,
                confirmation: None,
                author: "test",
            },
            &RunContext::default(),
        )
        .unwrap();
    let document = imported.document.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let runtime = Runtime::new(library, engine, format!("http://localhost:{port}"));
    let context = runtime.context(document.document_id).unwrap();
    let key = context.render.unwrap().render_key;
    let playback = runtime.playback(&key).unwrap();
    assert_eq!(playback.render_key, key);
    assert_eq!(playback.source_hash, document.source_hash);
    assert_eq!(playback.notes.len(), 3);
    assert!(!playback.systems.is_empty());
    assert_eq!(playback.notes[0].start_seconds, 0.0);

    let server = tokio::spawn(async move {
        axum::serve(listener, http::router(runtime, None, None, Vec::new()))
            .await
            .unwrap()
    });
    let response = reqwest::get(format!(
        "http://localhost:{port}/api/v1/renders/{key}/playback"
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), 200);
    let remote: ScorePlayback = response.json().await.unwrap();
    assert_eq!(remote.render_key, key);
    assert_eq!(remote.notes.len(), 3);
    server.abort();
}

#[test]
fn svg_only_render_generates_midi_when_playback_is_requested() {
    let root = tempfile::tempdir().unwrap();
    let library = Library::open(root.path()).unwrap();
    let source = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/rendered/absolute/score.ly"),
    )
    .unwrap();
    let engine = Arc::new(FixtureEngine::new(false));
    let imported = library
        .import(
            engine.as_ref(),
            ImportRequest {
                title: "playback-on-demand",
                input: &source,
                format: InputFormat::Ly,
                confirmation: None,
                author: "test",
            },
            &RunContext::default(),
        )
        .unwrap();
    let document = imported.document.unwrap();
    let runtime = Runtime::new(library, engine.clone(), "http://127.0.0.1:3211".into());
    let context = runtime.context(document.document_id).unwrap();
    let render = context.render.unwrap();
    assert!(render.manifest.midi.is_none());
    engine.midi.store(true, Ordering::Release);
    let playback = runtime.playback(&render.render_key).unwrap();
    assert_eq!(playback.notes.len(), 3);
}
