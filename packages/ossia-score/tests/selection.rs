use ossia_score::*;

#[test]
fn noteheads_outside_system_bounds_keep_their_source_measure_system() {
    let source = "Music = { c'1 | d'1 | } \\score { \\new Staff \\Music }";
    let first = source.find("c'1").unwrap();
    let second = source.find("d'1").unwrap();
    let svg = format!(
        r#"<svg viewBox="0 0 100 100">
      <g><line x1="5" x2="90" y1="20" y2="20"/><line x1="5" x2="90" y1="21" y2="21"/>
      <line x1="5" x2="90" y1="22" y2="22"/><line x1="5" x2="90" y1="23" y2="23"/>
      <line x1="5" x2="90" y1="24" y2="24"/></g>
      <g><line x1="5" x2="90" y1="60" y2="60"/><line x1="5" x2="90" y1="61" y2="61"/>
      <line x1="5" x2="90" y1="62" y2="62"/><line x1="5" x2="90" y1="63" y2="63"/>
      <line x1="5" x2="90" y1="64" y2="64"/></g>
      <g data-ossia-kind="NoteHead" data-ossia-staff="2" data-ossia-id="high" data-ossia-bounds="20 10 22 12" data-ossia-source="{first}:{}"/>
      <g data-ossia-kind="NoteHead" data-ossia-staff="0" data-ossia-id="low" data-ossia-bounds="20 72 22 74" data-ossia-source="{second}:{}"/>
      <g data-ossia-kind="BarLine" data-ossia-bounds="5 20 5.2 24"/>
      <g data-ossia-kind="BarLine" data-ossia-bounds="89 20 90 24"/>
      <g data-ossia-kind="BarLine" data-ossia-bounds="5 60 5.2 64"/>
      <g data-ossia-kind="BarLine" data-ossia-bounds="89 60 90 64"/>
    </svg>"#,
        first + 3,
        second + 3
    );
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let geometry = selection_geometry(&index, &layout).unwrap();
    let note = |text: &str| {
        geometry[0]
            .elements
            .iter()
            .find(|e| e.region.label.contains(text))
            .unwrap()
    };
    assert_eq!(note("c'1").system_index, Some(0));
    assert_eq!(note("d'1").system_index, Some(1));

    let inside_svg = svg.replace("20 10 22 12", "20 21 22 23");
    let mut no_measures = build_layout(
        &index,
        &[SvgPage {
            page: 1,
            svg: &inside_svg,
        }],
    )
    .unwrap();
    no_measures.columns.clear();
    let geometry = selection_geometry(&index, &no_measures).unwrap();
    let high = geometry[0]
        .elements
        .iter()
        .find(|e| e.region.label.contains("c'1"))
        .unwrap();
    assert_eq!(high.system_index, Some(0));

    let mut mirrored = index.clone();
    let mut tab_staff = mirrored.staves[0].clone();
    tab_staff.kind = StaffKind::Tablature;
    mirrored.staves.push(tab_staff);
    let geometry = selection_geometry(&mirrored, &no_measures).unwrap();
    let high = geometry[0]
        .elements
        .iter()
        .find(|e| e.region.label.contains("c'1"))
        .unwrap();
    assert_eq!(high.system_index, Some(0));

    no_measures.pages[0].systems[1].staves[0] = no_measures.pages[0].systems[0].staves[0].clone();
    let geometry = selection_geometry(&index, &no_measures).unwrap();
    let high = geometry[0]
        .elements
        .iter()
        .find(|e| e.region.label.contains("c'1"))
        .unwrap();
    assert_eq!(high.system_index, None);

    let mut single_system = layout.clone();
    single_system.pages[0].systems.truncate(1);
    single_system.columns.clear();
    let geometry = selection_geometry(&index, &single_system).unwrap();
    let high = geometry[0]
        .elements
        .iter()
        .find(|e| e.region.label.contains("c'1"))
        .unwrap();
    assert_eq!(high.system_index, Some(0));

    let mut ambiguous = layout.clone();
    for column in &mut ambiguous.columns {
        column.measure_start = Some(1);
        column.measure_end = Some(1);
    }
    let geometry = selection_geometry(&index, &ambiguous).unwrap();
    let high = geometry[0]
        .elements
        .iter()
        .find(|e| e.region.label.contains("c'1"))
        .unwrap();
    assert_eq!(high.system_index, None);
}

#[test]
fn unique_instrument_commands_fill_missing_lilypond_source_spans() {
    let source =
        "Music = { \\set Staff.instrumentName = \"Piano\" c'1 | } \\score { \\new Staff \\Music }";
    let svg = r#"<svg viewBox="0 0 100 60">
      <g><line x1="5" x2="90" y1="20" y2="20"/><line x1="5" x2="90" y1="21" y2="21"/>
      <line x1="5" x2="90" y1="22" y2="22"/><line x1="5" x2="90" y1="23" y2="23"/>
      <line x1="5" x2="90" y1="24" y2="24"/></g>
      <g data-ossia-kind="InstrumentName" data-ossia-bounds="1 18 7 20"/>
    </svg>"#;
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg }]).unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let name = pages[0]
        .elements
        .iter()
        .find(|element| element.region.element_kind.as_deref() == Some("InstrumentName"))
        .unwrap();
    assert_eq!(
        SourceMap::new(source).slice(name.region.spans[0]).unwrap(),
        "\\set Staff.instrumentName = \"Piano\""
    );
    assert!(name.region.insertion_anchor.is_none());
}

#[test]
fn generated_instrument_names_keep_a_staff_insertion_anchor() {
    let source = "Music = { c'1 | } \\score { \\new Staff \\Music }";
    let svg = r#"<svg viewBox="0 0 100 60">
      <g><line x1="5" x2="90" y1="20" y2="20"/><line x1="5" x2="90" y1="21" y2="21"/>
      <line x1="5" x2="90" y1="22" y2="22"/><line x1="5" x2="90" y1="23" y2="23"/>
      <line x1="5" x2="90" y1="24" y2="24"/></g>
      <g data-ossia-kind="InstrumentName" data-ossia-bounds="1 18 7 20"/>
    </svg>"#;
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg }]).unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let name = pages[0]
        .elements
        .iter()
        .find(|element| element.region.element_kind.as_deref() == Some("InstrumentName"))
        .unwrap();
    assert!(name.region.spans.is_empty());
    assert_eq!(
        source.as_bytes()[name.region.insertion_anchor.unwrap().byte_start - 1],
        b'{'
    );
    assert!(name.region.reason.is_some());
}

#[test]
fn adjacent_dynamics_articulations_and_hairpins_keep_the_measure_index() {
    let source = r#"Music = { c'4\p-. d'4\< e'4 f'4 | g'1\! | } \score { \new Staff \Music }"#;
    let tokens = tokenize(source);
    for command in ["\\p", "\\<", "\\!"] {
        assert!(tokens.iter().any(|t| t.raw == command), "{tokens:?}");
    }
    assert_eq!(build_index(source).capability, LocationCapability::Measure);
}

#[test]
fn measure_selection_excludes_prefix_symbols_and_stays_inside_barlines() {
    let source = "Music = { c'1 | d'1 | } \\score { \\new Staff \\Music }";
    let note = source.find("c'1").unwrap();
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
      <g><line x1="5" x2="90" y1="20" y2="20"/><line x1="5" x2="90" y1="21" y2="21"/>
      <line x1="5" x2="90" y1="22" y2="22"/><line x1="5" x2="90" y1="23" y2="23"/>
      <line x1="5" x2="90" y1="24" y2="24"/></g>
      <g data-ossia-kind="Clef" data-ossia-bounds="7 18 11 26"/>
      <g data-ossia-kind="TimeSignature" data-ossia-bounds="13 20 16 24"/>
      <g data-ossia-kind="NoteHead" data-ossia-bounds="20 21 22 23" data-ossia-source="{note}:{}"/>
      <rect x="49.9" y="20" width="0.2" height="4"/>
      <rect x="89.8" y="20" width="0.2" height="4"/>
      <a href="textedit://score.ly:1:{note}:0"><path d="M20 21 L22 23"/></a>
    </svg>"#,
        note + 3
    );
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let geometry = selection_geometry(&index, &layout).unwrap();
    let first = &geometry[0].regions[0];
    assert!(first.bounds.x0 > 16. && first.bounds.x0 < 20., "{first:?}");
    assert!((first.bounds.x1 - 49.9).abs() < 1e-8, "{first:?}");
    let second = &geometry[0].regions[1];
    assert!((second.bounds.x0 - 50.1).abs() < 1e-8, "{second:?}");
    assert!((second.bounds.x1 - 89.8).abs() < 1e-8, "{second:?}");
}

#[test]
fn svg_element_bounds_follow_visible_ink_and_the_svg_y_axis() {
    let source = "Music = { c'1 } \\score { \\new Staff \\Music }";
    let svg = r#"<svg viewBox="0 0 100 60">
      <g transform="translate(10, 30)">
        <a data-ossia-kind="Clef" data-ossia-id="clef" data-ossia-source="0:1">
          <rect x="0" y="-2" width="2" height="4" fill="none"/>
        </a>
      </g>
      <g data-ossia-graphic="clef">
        <g transform="translate(10, 30)"><path d="M0 -4 L2 2" fill="black"/></g>
      </g>
      <g transform="translate(20, 30)">
        <a data-ossia-kind="Tie" data-ossia-id="tie" data-ossia-owners="a,b" data-ossia-hit="true">
          <rect x="0" y="1" width="3" height="1" fill="none"/>
        </a>
      </g>
      <g data-ossia-graphic="tie">
        <g transform="translate(20, 30)"><path d="M0 -2 L3 -1" fill="none"/></g>
      </g>
    </svg>"#;
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg }]).unwrap();
    let page = &layout.pages[0];
    let clef = page
        .objects
        .iter()
        .find(|object| object.kind == "Clef")
        .unwrap();
    assert_eq!(
        clef.bbox,
        BBox {
            x0: 10.,
            y0: 26.,
            x1: 12.,
            y1: 32.
        }
    );
    let hit = page
        .objects
        .iter()
        .find(|object| object.kind == "Tie")
        .unwrap();
    assert_eq!(
        hit.bbox,
        BBox {
            x0: 20.,
            y0: 28.,
            x1: 23.,
            y1: 29.
        }
    );
}

#[test]
fn text_objects_use_the_complete_link_rect_instead_of_text_anchor_points() {
    let source = r#"\header { title = "标题" } Music = { c'1 } \score { \new Staff \Music }"#;
    let start = source.find("\"标题\"").unwrap();
    let end = start + "\"标题\"".len();
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
      <g><line x1="5" x2="90" y1="20" y2="20"/><line x1="5" x2="90" y1="21" y2="21"/>
      <line x1="5" x2="90" y1="22" y2="22"/><line x1="5" x2="90" y1="23" y2="23"/>
      <line x1="5" x2="90" y1="24" y2="24"/></g>
      <g transform="translate(20, 10)">
        <a data-ossia-kind="header-title" data-ossia-id="title" data-ossia-source="{start}:{end}">
          <rect x="0" y="-6" width="30" height="10" fill="none"/>
        </a>
      </g>
      <g data-ossia-graphic="title">
        <g transform="translate(20, 10)"><path d="M0 -4 L2 2"/></g>
        <g transform="translate(20, 10)"><text x="0" y="0"><tspan>标题</tspan></text></g>
        <g transform="translate(22, 14)"><text x="0" y="0"><tspan>副标题</tspan></text></g>
      </g>
    </svg>"#
    );
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let title = pages[0]
        .elements
        .iter()
        .find(|element| element.region.element_kind.as_deref() == Some("header-title"))
        .unwrap();
    assert_eq!(title.bounds.x0, 20.);
    assert_eq!(title.bounds.x1, 50.);
    let hit = hit_selection(&index, &pages, 1, 45., 10.).unwrap().unwrap();
    assert_eq!(hit.target, title.region.target);
}
