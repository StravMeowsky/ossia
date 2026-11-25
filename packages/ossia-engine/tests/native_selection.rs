use ossia_engine::*;
use ossia_score::{build_index, build_layout, selection_geometry, SvgPage};
use std::{fs, path::PathBuf};

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn instrument_names_have_selectable_source_bounds() {
    let root = PathBuf::from(
        std::env::var_os("OSSIA_TEST_LILYPOND_ROOT").expect("set OSSIA_TEST_LILYPOND_ROOT"),
    );
    let work = tempfile::tempdir().unwrap();
    let engine =
        Engine::verify(&root, &work.path().join("scratch"), &RunContext::default()).unwrap();
    let source = r#"\version "2.26.0"
\score { << \new PianoStaff <<
  \set PianoStaff.instrumentName = "Piano"
  \set PianoStaff.shortInstrumentName = "Pno"
  \new Staff { c'1 \break d'1 \bar "|." }
  \new Staff { c1 }
>> >> \layout {} }
"#;
    let snapshot = SourceSnapshot::new(DocumentId::new(), source.as_bytes().to_vec()).unwrap();
    let render = engine
        .compile(
            &snapshot,
            &CompileOptions {
                formats: vec![RenderFormat::Svg],
                point_and_click: true,
            },
            &work.path().join("artifacts"),
            &RunContext::default(),
        )
        .unwrap();
    assert!(
        render.format_errors.is_empty(),
        "{:?}",
        render.format_errors
    );
    let svgs: Vec<_> = render
        .pages
        .iter()
        .map(|page| {
            (
                page.page,
                fs::read_to_string(work.path().join("artifacts").join(&page.artifact.path))
                    .unwrap(),
            )
        })
        .collect();
    let index = build_index(source);
    let layout = build_layout(
        &index,
        &svgs
            .iter()
            .map(|(page, svg)| SvgPage { page: *page, svg })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let elements = pages
        .iter()
        .flat_map(|page| &page.elements)
        .collect::<Vec<_>>();
    for (kind, raw) in [
        (
            "InstrumentName",
            "\\set PianoStaff.instrumentName = \"Piano\"",
        ),
        (
            "ShortInstrumentName",
            "\\set PianoStaff.shortInstrumentName = \"Pno\"",
        ),
    ] {
        let name = elements
            .iter()
            .find(|element| element.region.element_kind.as_deref() == Some(kind))
            .unwrap_or_else(|| panic!("missing {kind}: {elements:#?}"));
        assert_eq!(
            &source[name.region.spans[0].byte_start..name.region.spans[0].byte_end],
            raw,
            "{kind}"
        );
        let hit = ossia_score::hit_selection(
            &index,
            &pages,
            pages[0].page,
            (name.bounds.x0 + name.bounds.x1) / 2.,
            (name.bounds.y0 + name.bounds.y1) / 2.,
        )
        .unwrap()
        .expect("instrument name is not selectable");
        assert_eq!(hit.target, name.region.target, "{kind}");
    }
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn chord_mode_symbols_are_selectable_source_elements() {
    let root = PathBuf::from(
        std::env::var_os("OSSIA_TEST_LILYPOND_ROOT").expect("set OSSIA_TEST_LILYPOND_ROOT"),
    );
    let work = tempfile::tempdir().unwrap();
    let engine =
        Engine::verify(&root, &work.path().join("scratch"), &RunContext::default()).unwrap();
    let source = r#"\version "2.26.0"
harmonies = \chordmode {
  b1:m7 |
  a1/e |
}
\score {
  <<
    \new ChordNames \harmonies
    \new Staff { b'1 | a'1 }
  >>
  \layout {}
}
"#;
    let snapshot = SourceSnapshot::new(DocumentId::new(), source.as_bytes().to_vec()).unwrap();
    let render = engine
        .compile(
            &snapshot,
            &CompileOptions {
                formats: vec![RenderFormat::Svg],
                point_and_click: true,
            },
            &work.path().join("artifacts"),
            &RunContext::default(),
        )
        .unwrap();
    assert!(
        render.format_errors.is_empty(),
        "{:#?}",
        render.format_errors
    );
    let svgs: Vec<_> = render
        .pages
        .iter()
        .map(|page| {
            (
                page.page,
                fs::read_to_string(work.path().join("artifacts").join(&page.artifact.path))
                    .unwrap(),
            )
        })
        .collect();
    let index = build_index(source);
    let layout = build_layout(
        &index,
        &svgs
            .iter()
            .map(|(page, svg)| SvgPage { page: *page, svg })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let symbols = pages
        .iter()
        .flat_map(|page| &page.elements)
        .filter(|element| element.region.element_kind.as_deref() == Some("ChordName"))
        .collect::<Vec<_>>();
    assert_eq!(symbols.len(), 2, "{symbols:#?}");
    for (symbol, expected) in symbols.iter().zip(["b1:m7", "a1/e"]) {
        assert!(
            !symbol.graphic_ids.is_empty(),
            "chord symbol has no annotation graphics: {symbol:#?}"
        );
        assert_eq!(
            &source[symbol.region.spans[0].byte_start..symbol.region.spans[0].byte_end],
            expected
        );
        let hit = ossia_score::hit_selection(
            &index,
            &pages,
            pages[0].page,
            (symbol.bounds.x0 + symbol.bounds.x1) / 2.,
            (symbol.bounds.y0 + symbol.bounds.y1) / 2.,
        )
        .unwrap()
        .expect("chord symbol is not selectable");
        assert_eq!(hit.target, symbol.region.target);
    }
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn interactive_elements_have_exact_source_bounds_without_changing_the_score() {
    let root = PathBuf::from(
        std::env::var_os("OSSIA_TEST_LILYPOND_ROOT").expect("set OSSIA_TEST_LILYPOND_ROOT"),
    );
    let work = tempfile::tempdir().unwrap();
    let engine =
        Engine::verify(&root, &work.path().join("scratch"), &RunContext::default()).unwrap();
    let source = r#"\version "2.26.0"
\header {
  "title" = "点选测试"
  subtitle = \markup { \italic "第二行" }
  composer = "作者"
  copyright = "版权"
  "id: software" = "music21 v.10.5.2"
  "id: encoding-date" = "2026-11-07"
}
\layout { \context { \Staff printKeyCancellation = ##f } \context { \Score autoBeaming = ##f } }
Music = { \clef treble \key g \major \time 4/4 \tempo "Andante" 4 = 80
  <c' e' g'>4\p-. d'4^\markup { "表情" } e'4\< ( f'4 | g'1 )\! \mark \markup { "A" } \break
  r1 | c'1 \bar "|." }
\score { \new Staff \Music \layout {} \midi {} }
"#;
    let snapshot = SourceSnapshot::new(DocumentId::new(), source.as_bytes().to_vec()).unwrap();
    let mut renders = Vec::new();
    for point_and_click in [false, true] {
        let render = engine
            .compile(
                &snapshot,
                &CompileOptions {
                    formats: vec![RenderFormat::Svg, RenderFormat::Midi],
                    point_and_click,
                },
                &work.path().join("artifacts"),
                &RunContext::default(),
            )
            .unwrap();
        assert!(
            render.format_errors.is_empty(),
            "{:#?}",
            render.format_errors
        );
        renders.push(render);
    }
    assert_eq!(
        renders[0].midi.as_ref().unwrap().sha256,
        renders[1].midi.as_ref().unwrap().sha256
    );
    let pngs = renders
        .iter()
        .map(|render| {
            let svg = fs::read(
                work.path()
                    .join("artifacts")
                    .join(&render.pages[0].artifact.path),
            )
            .unwrap();
            engine
                .rasterize_svg(
                    &svg,
                    1200,
                    &work.path().join("pixels"),
                    &RunContext::default(),
                )
                .unwrap()
                .sha256
        })
        .collect::<Vec<_>>();
    assert_eq!(
        pngs[0], pngs[1],
        "interactive metadata must not change visible pixels"
    );
    let svgs: Vec<_> = renders[1]
        .pages
        .iter()
        .map(|p| {
            (
                p.page,
                fs::read_to_string(work.path().join("artifacts").join(&p.artifact.path)).unwrap(),
            )
        })
        .collect();
    let index = build_index(source);
    let layout = build_layout(
        &index,
        &svgs
            .iter()
            .map(|(page, svg)| SvgPage { page: *page, svg })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let elements = pages.iter().flat_map(|p| &p.elements).collect::<Vec<_>>();
    for kind in ["Slur", "Hairpin"] {
        let curve = elements
            .iter()
            .find(|e| e.region.element_kind.as_deref() == Some(kind))
            .unwrap();
        assert!(
            !curve.hit_bounds.is_empty(),
            "missing ink bounds for {kind}"
        );
    }
    for (kind, raw) in [
        ("Clef", "\\clef treble"),
        ("TimeSignature", "\\time 4/4"),
        ("KeySignature", "\\key g \\major"),
        ("DynamicText", "\\p"),
        ("TextScript", "^\\markup { \"表情\" }"),
        ("RehearsalMark", "\\mark \\markup { \"A\" }"),
    ] {
        let object = elements
            .iter()
            .find(|e| e.region.element_kind.as_deref() == Some(kind))
            .unwrap();
        assert_eq!(
            &source[object.region.spans[0].byte_start..object.region.spans[0].byte_end],
            raw,
            "{kind}"
        );
    }
    for kind in [
        "header-title",
        "header-subtitle",
        "header-composer",
        "header-copyright",
        "NoteHead",
        "Rest",
        "Clef",
        "KeySignature",
        "TimeSignature",
        "DynamicText",
        "Script",
        "TextScript",
        "MetronomeMark",
        "RehearsalMark",
        "Hairpin",
        "Slur",
    ] {
        assert!(
            elements
                .iter()
                .any(|e| e.region.element_kind.as_deref() == Some(kind)),
            "missing {kind}: {elements:#?}"
        );
    }
    let chord: Vec<_> = elements
        .iter()
        .filter(|e| {
            e.region.element_kind.as_deref() == Some("NoteHead")
                && e.region
                    .spans
                    .iter()
                    .any(|s| ["c'", "e'", "g'"].contains(&&source[s.byte_start..s.byte_end]))
        })
        .collect();
    assert!(chord.len() >= 3, "{chord:#?}");
    assert!(
        pages.iter().any(|p| !p.regions.is_empty()),
        "index={index:?}; columns={:?}",
        layout.columns
    );
    for e in elements {
        assert!(
            e.bounds.x1 > e.bounds.x0 && e.bounds.y1 > e.bounds.y0,
            "{e:?}"
        );
        for span in &e.region.spans {
            assert!(!ossia_score::SourceMap::new(source)
                .slice(*span)
                .unwrap()
                .is_empty())
        }
    }
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn default_symbols_reused_voices_and_dynamic_headers_keep_their_source_identity() {
    let root = PathBuf::from(
        std::env::var_os("OSSIA_TEST_LILYPOND_ROOT").expect("set OSSIA_TEST_LILYPOND_ROOT"),
    );
    let work = tempfile::tempdir().unwrap();
    let engine =
        Engine::verify(&root, &work.path().join("scratch"), &RunContext::default()).unwrap();
    let source = r#"\version "2.26.0"
TitleText = "标题变量"
NoAuthor = ##f
\header { title = \TitleText subtitle = #(string-append "动态" "标题") arranger = \NoAuthor copyright = #(if #f "hidden" #f) }
Shared = { c'1( | \break d'1) \bar "|." }
\score { \new PianoStaff << \new Staff = "upper" \Shared \new Staff = "lower" \Shared >> \layout {} }
"#;
    let snapshot = SourceSnapshot::new(DocumentId::new(), source.as_bytes().to_vec()).unwrap();
    let render = engine
        .compile(
            &snapshot,
            &CompileOptions {
                formats: vec![RenderFormat::Svg],
                point_and_click: true,
            },
            &work.path().join("artifacts"),
            &RunContext::default(),
        )
        .unwrap();
    assert!(
        render.format_errors.is_empty(),
        "{:?}",
        render.format_errors
    );
    let svg = fs::read_to_string(
        work.path()
            .join("artifacts")
            .join(&render.pages[0].artifact.path),
    )
    .unwrap();
    assert!(!svg.contains("ossia:") && !svg.contains("/private/"));
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    assert_eq!(pages[0].regions.len(), 4, "{:#?}", layout.columns);
    for kind in ["Clef", "TimeSignature"] {
        let generated = pages[0]
            .elements
            .iter()
            .filter(|e| e.region.element_kind.as_deref() == Some(kind))
            .collect::<Vec<_>>();
        assert!(generated.len() >= 2, "{kind}: {generated:#?}");
        for element in &generated {
            assert!(element.region.spans.is_empty());
            assert_eq!(
                source.as_bytes()[element.region.insertion_anchor.unwrap().byte_start - 1],
                b'{'
            );
            assert_eq!(
                element.region.occurrences.len(),
                if kind == "Clef" { 2 } else { 1 }
            );
        }
        assert_eq!(
            generated
                .iter()
                .map(|element| format!("{:?}", element.region.target))
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            2
        );
    }
    let notes = pages[0]
        .elements
        .iter()
        .filter(|e| e.region.element_kind.as_deref() == Some("NoteHead"))
        .collect::<Vec<_>>();
    assert_eq!(notes.len(), 4);
    assert_eq!(
        notes
            .iter()
            .map(|note| format!("{:?}", note.region.target))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    assert!(notes.iter().all(|note| note.region.occurrences.len() == 1));
    let slurs = pages[0]
        .elements
        .iter()
        .filter(|e| e.region.element_kind.as_deref() == Some("Slur"))
        .collect::<Vec<_>>();
    assert_eq!(
        slurs
            .iter()
            .map(|slur| format!("{:?}", slur.region.target))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2
    );
    assert!(slurs.iter().all(|slur| slur.region.occurrences.len() == 2));
    for kind in ["header-title", "header-subtitle"] {
        let header = pages[0]
            .elements
            .iter()
            .find(|e| e.region.element_kind.as_deref() == Some(kind))
            .unwrap_or_else(|| panic!("missing {kind}"));
        let hit = ossia_score::hit_selection(
            &index,
            &pages,
            pages[0].page,
            (header.bounds.x0 + header.bounds.x1) / 2.,
            (header.bounds.y0 + header.bounds.y1) / 2.,
        )
        .unwrap()
        .unwrap_or_else(|| panic!("{kind} is not selectable"));
        assert_eq!(hit.target, header.region.target);
    }
    assert!(!pages[0].elements.iter().any(|e| matches!(
        e.region.element_kind.as_deref(),
        Some("header-copyright" | "header-arranger")
    )));
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn native_source_selection_matches_static_render() {
    let root = PathBuf::from(
        std::env::var_os("OSSIA_TEST_LILYPOND_ROOT").expect("set OSSIA_TEST_LILYPOND_ROOT"),
    );
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/inputs/polyphonic-pages.ly"),
    )
    .unwrap();
    let work = tempfile::tempdir().unwrap();
    let engine =
        Engine::verify(&root, &work.path().join("scratch"), &RunContext::default()).unwrap();
    let snapshot = SourceSnapshot::new(DocumentId::new(), source.as_bytes().to_vec()).unwrap();
    let mut renders = Vec::new();
    for point_and_click in [false, true] {
        let render = engine
            .compile(
                &snapshot,
                &CompileOptions {
                    formats: vec![RenderFormat::Svg, RenderFormat::Midi],
                    point_and_click,
                },
                &work.path().join("artifacts"),
                &RunContext::default(),
            )
            .unwrap();
        assert!(
            render.format_errors.is_empty(),
            "{:?}",
            render.format_errors
        );
        renders.push(render);
    }
    assert_eq!(
        renders[0].midi.as_ref().unwrap().sha256,
        renders[1].midi.as_ref().unwrap().sha256
    );
    assert_eq!(renders[0].pages.len(), renders[1].pages.len());
    let mut svgs = Vec::new();
    for (plain, interactive) in renders[0].pages.iter().zip(&renders[1].pages) {
        let images = [plain, interactive].map(|page| {
            let svg = fs::read(work.path().join("artifacts").join(&page.artifact.path)).unwrap();
            let png = engine
                .rasterize_svg(
                    &svg,
                    1200,
                    &work.path().join("pixels"),
                    &RunContext::default(),
                )
                .unwrap();
            (svg, png.sha256)
        });
        assert_eq!(images[0].1, images[1].1, "page {} changed", plain.page);
        svgs.push((
            interactive.page,
            String::from_utf8(images[1].0.clone()).unwrap(),
        ));
    }
    let index = build_index(&source);
    let layout = build_layout(
        &index,
        &svgs
            .iter()
            .map(|(page, svg)| SvgPage { page: *page, svg })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let mut notes = 0;
    for page in pages.iter() {
        let note = page
            .elements
            .iter()
            .find(|e| {
                e.region.element_kind.as_deref() == Some("NoteHead")
                    && e.region.staff_index == Some(0)
            })
            .expect("page has no selectable upper-staff notes");
        notes += page
            .elements
            .iter()
            .filter(|e| e.region.element_kind.as_deref() == Some("NoteHead"))
            .count();
        let selected = ossia_score::hit_selection(
            &index,
            &pages,
            page.page,
            (note.bounds.x0 + note.bounds.x1) / 2.,
            (note.bounds.y0 + note.bounds.y1) / 2.,
        )
        .unwrap()
        .unwrap();
        assert_eq!(selected.target, note.region.target);
        let region = page
            .regions
            .iter()
            .find(|r| r.locator.staff_index == 0)
            .unwrap();
        let prefix = page
            .elements
            .iter()
            .filter(|e| {
                e.region.staff_index == Some(0)
                    && matches!(
                        e.region.element_kind.as_deref(),
                        Some("Clef" | "TimeSignature" | "KeySignature")
                    )
                    && e.bounds.x0 < note.bounds.x0
                    && e.bounds.y0 < region.bounds.y1
                    && e.bounds.y1 > region.bounds.y0
            })
            .map(|e| e.bounds.x1)
            .max_by(f64::total_cmp)
            .unwrap();
        assert!(
            region.bounds.x0 > prefix,
            "page {} includes the prefix",
            page.page
        );
    }
    println!(
        "verified {} pages, {notes} selectable note heads; static pixels and MIDI match",
        pages.len()
    );
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn tab_staff_notes_are_independent_and_playable() {
    let root = PathBuf::from(
        std::env::var_os("OSSIA_TEST_LILYPOND_ROOT").expect("set OSSIA_TEST_LILYPOND_ROOT"),
    );
    let work = tempfile::tempdir().unwrap();
    let artifacts = work.path().join("artifacts");
    let engine =
        Engine::verify(&root, &work.path().join("scratch"), &RunContext::default()).unwrap();
    let source = r#"\version "2.26.0"
music = {
  a,8\5 c\5 e\4 a\3 e\4 c\5 e\4 c\5 |
  <a,\5 e\4 a\3 c'\2 e'\1>1 \bar "|."
}
\score {
  \new StaffGroup <<
    \new Staff \with { \clef "treble_8" } { \omit StringNumber \music }
    \new TabStaff { \music }
  >>
  \layout { }
  \midi { \context { \TabStaff \remove "Staff_performer" } }
}
"#;
    let snapshot = SourceSnapshot::new(DocumentId::new(), source.as_bytes().to_vec()).unwrap();
    let render = engine
        .compile(
            &snapshot,
            &CompileOptions {
                formats: vec![RenderFormat::Svg, RenderFormat::Midi],
                point_and_click: true,
            },
            &artifacts,
            &RunContext::default(),
        )
        .unwrap();
    assert!(
        render.format_errors.is_empty(),
        "{:#?}",
        render.format_errors
    );
    let svgs: Vec<_> = render
        .pages
        .iter()
        .map(|page| {
            (
                page.page,
                fs::read_to_string(artifacts.join(&page.artifact.path)).unwrap(),
            )
        })
        .collect();
    let index = build_index(source);
    let layout = build_layout(
        &index,
        &svgs
            .iter()
            .map(|(page, svg)| SvgPage { page: *page, svg })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let notes = pages
        .iter()
        .flat_map(|page| &page.elements)
        .filter(|element| {
            matches!(
                element.region.element_kind.as_deref(),
                Some("NoteHead" | "TabNoteHead")
            )
        })
        .collect::<Vec<_>>();
    let staff_notes = notes
        .iter()
        .copied()
        .filter(|element| element.region.staff_index == Some(0))
        .collect::<Vec<_>>();
    let tab_notes = notes
        .iter()
        .copied()
        .filter(|element| element.region.staff_index == Some(1))
        .collect::<Vec<_>>();
    assert_eq!(tab_notes.len(), 13, "{notes:#?}");
    assert_eq!(index.staves.len(), 2);
    assert_eq!(layout.pages[0].systems.len(), 1);
    assert_eq!(layout.pages[0].systems[0].staves.len(), 2);

    let midi = ossia_score::parse_midi(
        &fs::read(artifacts.join(&render.midi.as_ref().unwrap().path)).unwrap(),
    )
    .unwrap();
    let playback =
        ossia_score::build_playback(&index, &layout, &midi, ossia_score::ContentHash::of(b"tab"))
            .unwrap();
    let system = &layout.pages[0].systems[0];
    let tab_staff = &system.staves[1];
    let staff_space = layout.pages[0].staff_space;
    for tab in &tab_notes {
        assert!(
            tab.hit_bounds.iter().all(|bounds| {
                bounds.y0 >= tab_staff.top - staff_space * 2.
                    && bounds.y1 <= tab_staff.bot + staff_space * 2.
            }),
            "tab hit bounds include another staff: {tab:#?}"
        );
    }
    assert!(playback
        .systems
        .iter()
        .any(|span| { span.y0 <= system.top && span.y1 >= system.bot }));
    for tab in &tab_notes {
        let staff_note = staff_notes
            .iter()
            .find(|note| note.region.spans == tab.region.spans)
            .expect("tab note has no matching staff note");
        assert_ne!(
            staff_note.region.target, tab.region.target,
            "different staves must use different element ids"
        );
        assert_eq!(
            tab.region.occurrences.len(),
            1,
            "tab annotation must stay on the tab staff"
        );
        let selected = ossia_score::hit_selection(
            &index,
            &pages,
            pages[0].page,
            (tab.bounds.x0 + tab.bounds.x1) / 2.,
            (tab.bounds.y0 + tab.bounds.y1) / 2.,
        )
        .unwrap()
        .expect("tab note is not selectable");
        assert_eq!(selected.target, tab.region.target);
        let ossia_types::SelectionTarget::Element { element_id } = &selected.target else {
            panic!("tab note selected as a measure");
        };
        let group = playback
            .groups
            .iter()
            .find(|group| group.element_ids.contains(element_id))
            .expect("tab note is missing from playback groups");
        let ossia_types::SelectionTarget::Element {
            element_id: staff_element_id,
        } = &staff_note.region.target
        else {
            panic!("staff note is not an element");
        };
        assert!(
            group.element_ids.contains(staff_element_id),
            "the tab note and its staff note must share a playback group: tab={tab:#?} group={group:#?} groups={:#?}", playback.groups
        );
    }
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn standalone_tab_staff_is_selectable_and_playable() {
    let root = PathBuf::from(
        std::env::var_os("OSSIA_TEST_LILYPOND_ROOT").expect("set OSSIA_TEST_LILYPOND_ROOT"),
    );
    let work = tempfile::tempdir().unwrap();
    let artifacts = work.path().join("artifacts");
    let engine =
        Engine::verify(&root, &work.path().join("scratch"), &RunContext::default()).unwrap();
    let source = r#"\version "2.26.0"
music = {
  \time 4/4
  \key a \minor
  a,8\5 c\5 e\4 a\3 e\4 c\5 e\4 c\5 |
  <a,\5 e\4 a\3 c'\2 e'\1>1 \bar "|."
}
\score {
  \new TabStaff {
    \set Staff.midiInstrument = "acoustic guitar (nylon)"
    \music
  }
  \layout { }
  \midi { }
}
"#;
    let snapshot = SourceSnapshot::new(DocumentId::new(), source.as_bytes().to_vec()).unwrap();
    let render = engine
        .compile(
            &snapshot,
            &CompileOptions {
                formats: vec![RenderFormat::Svg, RenderFormat::Midi],
                point_and_click: true,
            },
            &artifacts,
            &RunContext::default(),
        )
        .unwrap();
    assert!(
        render.format_errors.is_empty(),
        "{:#?}",
        render.format_errors
    );
    let svgs: Vec<_> = render
        .pages
        .iter()
        .map(|page| {
            (
                page.page,
                fs::read_to_string(artifacts.join(&page.artifact.path)).unwrap(),
            )
        })
        .collect();
    let index = build_index(source);
    assert_eq!(index.staves.len(), 1);
    assert_eq!(index.staves[0].kind, ossia_score::StaffKind::Tablature);
    let layout = build_layout(
        &index,
        &svgs
            .iter()
            .map(|(page, svg)| SvgPage { page: *page, svg })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(layout.pages[0].systems.len(), 1);
    assert_eq!(layout.pages[0].systems[0].staves.len(), 1);
    let pages = selection_geometry(&index, &layout).unwrap();
    let notes = pages
        .iter()
        .flat_map(|page| &page.elements)
        .filter(|element| {
            matches!(
                element.region.element_kind.as_deref(),
                Some("NoteHead" | "TabNoteHead")
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(notes.len(), 13, "{notes:#?}");
    assert!(notes
        .iter()
        .all(|note| note.region.element_kind.as_deref() == Some("TabNoteHead")));
    let midi = ossia_score::parse_midi(
        &fs::read(artifacts.join(&render.midi.as_ref().unwrap().path)).unwrap(),
    )
    .unwrap();
    let playback =
        ossia_score::build_playback(&index, &layout, &midi, ossia_score::ContentHash::of(b"tab"))
            .unwrap();
    let system = &layout.pages[0].systems[0];
    assert!(playback
        .systems
        .iter()
        .any(|span| { span.y0 <= system.top && span.y1 >= system.bot }));
    for note in notes {
        assert_eq!(note.region.staff_index, Some(0));
        let selected = ossia_score::hit_selection(
            &index,
            &pages,
            pages[0].page,
            (note.bounds.x0 + note.bounds.x1) / 2.,
            (note.bounds.y0 + note.bounds.y1) / 2.,
        )
        .unwrap()
        .expect("tab note is not selectable");
        assert_eq!(selected.target, note.region.target);
        let ossia_types::SelectionTarget::Element { element_id } = &selected.target else {
            panic!("tab note selected as a measure");
        };
        assert!(
            playback
                .groups
                .iter()
                .any(|group| group.element_ids.contains(element_id)),
            "tab note is missing from playback groups: {element_id}"
        );
    }
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn simultaneous_notes_on_multiple_staves_reach_playback() {
    let root = PathBuf::from(
        std::env::var_os("OSSIA_TEST_LILYPOND_ROOT").expect("set OSSIA_TEST_LILYPOND_ROOT"),
    );
    let work = tempfile::tempdir().unwrap();
    let artifacts = work.path().join("artifacts");
    let engine =
        Engine::verify(&root, &work.path().join("scratch"), &RunContext::default()).unwrap();
    let source = r#"\version "2.26.0"
music = { c'1 }
\score {
  <<
    \new Staff { \set Staff.midiInstrument = "acoustic grand" \music }
    \new Staff { \set Staff.midiInstrument = "violin" \music }
  >>
  \layout { }
  \midi { }
}
"#;
    let snapshot = SourceSnapshot::new(DocumentId::new(), source.as_bytes().to_vec()).unwrap();
    let render = engine
        .compile(
            &snapshot,
            &CompileOptions {
                formats: vec![RenderFormat::Svg, RenderFormat::Midi],
                point_and_click: true,
            },
            &artifacts,
            &RunContext::default(),
        )
        .unwrap();
    assert!(
        render.format_errors.is_empty(),
        "{:#?}",
        render.format_errors
    );
    let svgs: Vec<_> = render
        .pages
        .iter()
        .map(|page| {
            (
                page.page,
                fs::read_to_string(artifacts.join(&page.artifact.path)).unwrap(),
            )
        })
        .collect();
    let index = build_index(source);
    let layout = build_layout(
        &index,
        &svgs
            .iter()
            .map(|(page, svg)| SvgPage { page: *page, svg })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let notes = pages
        .iter()
        .flat_map(|page| &page.elements)
        .filter(|element| element.region.element_kind.as_deref() == Some("NoteHead"))
        .collect::<Vec<_>>();
    assert_eq!(notes.len(), 2);
    assert_ne!(notes[0].region.target, notes[1].region.target);
    assert!(notes.iter().all(|note| note.region.occurrences.len() == 1));
    let midi = ossia_score::parse_midi(
        &fs::read(artifacts.join(&render.midi.as_ref().unwrap().path)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        midi.tracks
            .iter()
            .flat_map(|track| &track.notes)
            .filter(|note| note.time == 0 && note.note == 60)
            .count(),
        2
    );
    let playback = ossia_score::build_playback(
        &index,
        &layout,
        &midi,
        ossia_score::ContentHash::of(b"staves"),
    )
    .unwrap();
    assert_eq!(
        playback
            .notes
            .iter()
            .filter(|note| note.start_seconds == 0.0 && note.pitch == 60)
            .count(),
        2
    );
    assert_eq!(playback.groups.len(), 2);
    assert!(
        playback
            .parts
            .iter()
            .all(|part| part.id.starts_with("part:")),
        "{:?}",
        playback.parts
    );
    for group in &playback.groups {
        assert_eq!(group.element_ids.len(), 1);
        assert_eq!(group.pitches, vec![60]);
        let note = playback
            .notes
            .iter()
            .find(|note| Some(&note.part_id) == group.part_id.as_ref())
            .unwrap();
        assert_eq!(
            note.instrument,
            if group.staff_index == Some(0) {
                SampleInstrument::Piano
            } else {
                SampleInstrument::Violin
            }
        );
    }
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn graphical_tempo_keeps_interactive_svg_well_formed() {
    let root = PathBuf::from(
        std::env::var_os("OSSIA_TEST_LILYPOND_ROOT").expect("set OSSIA_TEST_LILYPOND_ROOT"),
    );
    let work = tempfile::tempdir().unwrap();
    let engine =
        Engine::verify(&root, &work.path().join("scratch"), &RunContext::default()).unwrap();
    let source = r#"\version "2.26.0"
\score { \new Staff {
  \tempo \markup { \rhythm { 4 } = 81 }
  c'1 | d'1 \bar "|."
} \layout {} }
"#;
    let snapshot = SourceSnapshot::new(DocumentId::new(), source.as_bytes().to_vec()).unwrap();
    let artifacts = work.path().join("artifacts");
    let render = engine
        .compile(
            &snapshot,
            &CompileOptions {
                formats: vec![RenderFormat::Svg],
                point_and_click: true,
            },
            &artifacts,
            &RunContext::default(),
        )
        .unwrap();
    assert!(
        render.format_errors.is_empty(),
        "{:?}",
        render.format_errors
    );
    let svgs: Vec<_> = render
        .pages
        .iter()
        .map(|page| {
            (
                page.page,
                fs::read_to_string(artifacts.join(&page.artifact.path)).unwrap(),
            )
        })
        .collect();
    let index = build_index(source);
    let layout = build_layout(
        &index,
        &svgs
            .iter()
            .map(|(page, svg)| SvgPage { page: *page, svg })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let elements = pages
        .iter()
        .flat_map(|page| &page.elements)
        .collect::<Vec<_>>();
    assert!(
        elements
            .iter()
            .any(|element| element.region.element_kind.as_deref() == Some("MetronomeMark")),
        "{elements:#?}"
    );
    let tempo = elements
        .iter()
        .find(|element| element.region.element_kind.as_deref() == Some("MetronomeMark"))
        .unwrap();
    assert!(tempo.bounds.x1 - tempo.bounds.x0 > 4., "{tempo:#?}");
    let hit = ossia_score::hit_selection(
        &index,
        &pages,
        pages[0].page,
        tempo.bounds.x1 - 0.1,
        (tempo.bounds.y0 + tempo.bounds.y1) / 2.,
    )
    .unwrap()
    .expect("tempo tail is not selectable");
    assert_eq!(hit.target, tempo.region.target);
    assert!(
        elements
            .iter()
            .any(|element| element.region.element_kind.as_deref() == Some("NoteHead")),
        "{elements:#?}"
    );
}
