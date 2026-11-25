use ossia_engine::*;
use std::{fs, path::PathBuf, time::Duration};

#[test]
#[ignore = "requires native LilyPond and OSSIA_TEST_LY_INPUTS"]
fn imported_scores_have_playback_trajectory() {
    let root = PathBuf::from(std::env::var_os("OSSIA_TEST_LILYPOND_ROOT").unwrap());
    let paths = std::env::split_paths(&std::env::var_os("OSSIA_TEST_LY_INPUTS").unwrap())
        .collect::<Vec<_>>();
    let work = tempfile::tempdir().unwrap();
    let context = RunContext::new(Duration::from_secs(180));
    let engine = Engine::verify(&root, &work.path().join("scratch"), &context).unwrap();
    let mut failures = Vec::new();
    for path in paths {
        let source = fs::read_to_string(&path).unwrap();
        let snapshot = SourceSnapshot::new(DocumentId::new(), source.as_bytes().to_vec()).unwrap();
        let assets = work.path().join(DocumentId::new().to_string());
        let render = engine
            .compile(
                &snapshot,
                &CompileOptions {
                    formats: vec![RenderFormat::Svg, RenderFormat::Midi],
                    point_and_click: true,
                },
                &assets,
                &context,
            )
            .unwrap();
        assert!(
            render.format_errors.is_empty(),
            "{}: {:?}",
            path.display(),
            render.format_errors
        );
        let midi =
            ossia_score::parse_midi(&fs::read(assets.join(render.midi.unwrap().path)).unwrap())
                .unwrap();
        let pages = render
            .pages
            .iter()
            .map(|p| {
                (
                    p.page,
                    fs::read_to_string(assets.join(&p.artifact.path)).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let index = ossia_score::build_index(&source);
        eprintln!(
            "index: capability={:?} reason={:?} staves={:?} voices={:?}, signatures={:?}",
            index.capability,
            index.reason,
            index.staves,
            index
                .voices
                .iter()
                .map(|v| (&v.name, v.capability, &v.reason, v.measures.len()))
                .collect::<Vec<_>>(),
            midi.time_signatures
        );
        for voice in &index.voices {
            let resolved = ossia_score::resolve_pitches(&source, voice.music_span).unwrap();
            eprintln!(
                "voice {}: pitch events={}, unsupported={:?}",
                voice.name,
                resolved.events.len(),
                resolved.unsupported
            );
        }
        let layout = ossia_score::build_layout(
            &index,
            &pages
                .iter()
                .map(|(page, svg)| ossia_score::SvgPage { page: *page, svg })
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let playback =
            ossia_score::build_playback(&index, &layout, &midi, ContentHash::of(source.as_bytes()))
                .unwrap();
        eprintln!(
            "{}: pages={} columns={} measured={} notes={} groups={} systems={} duration={}",
            path.display(),
            pages.len(),
            layout.columns.len(),
            layout
                .columns
                .iter()
                .filter(|c| c.measure_start.is_some())
                .count(),
            playback.notes.len(),
            playback.groups.len(),
            playback.systems.len(),
            playback.duration_seconds
        );
        let uncovered_notes = playback
            .notes
            .iter()
            .filter(|note| {
                !playback.systems.iter().any(|system| {
                    note.start_seconds >= system.start_seconds
                        && note.start_seconds < system.end_seconds
                }) || !playback.groups.iter().any(|group| {
                    group.start_seconds.is_some_and(|start| {
                        if let Some(anchor) = &note.ornament_anchor {
                            group.element_ids.contains(anchor)
                                && group.part_id.as_ref() == Some(&note.part_id)
                                && start <= note.start_seconds + 0.001
                                && note.start_seconds + note.duration_seconds
                                    <= start + group.duration_seconds.unwrap_or(0.) + 0.001
                        } else {
                            group.pitches.contains(&note.pitch)
                                && (start - note.start_seconds).abs() < 0.001
                        }
                    })
                })
            })
            .count();
        eprintln!("uncovered playback notes: {uncovered_notes}");
        let repeated_target = playback.groups.iter().enumerate().any(|(i, first)| {
            playback.groups[i + 1..].iter().any(|second| {
                first.start_seconds != second.start_seconds
                    && first
                        .element_ids
                        .iter()
                        .any(|id| second.element_ids.contains(id))
            })
        });
        let missing_repeat = source.contains("\\unfoldRepeats") && !repeated_target;
        eprintln!("repeated rendered target: {repeated_target}");
        if playback.systems.is_empty()
            || playback.groups.is_empty()
            || uncovered_notes > 0
            || missing_repeat
        {
            failures.push(path);
        }
    }
    assert!(failures.is_empty(), "missing trajectory: {failures:?}");
}
