use crate::{source, *};

#[test]
fn separate_layout_and_midi_scores_select_the_requested_output() {
    let text = "\\header { title = \\markup { \\override #'(baseline-skip . 0) title } }\n\\score { { c'1 } \\layout {} }\n\\score { { d'1 } \\midi { \\tempo 4 = 160 } }";
    let midi = source::derive(text, RenderFormat::Midi, true).unwrap();
    assert!(!midi.text.contains("c'1"));
    assert!(midi.text.contains("d'1"));
    assert!(midi.text.contains("\\tempo 4 = 160"));
    assert_eq!(
        midi.original_byte(midi.text.find("d'1").unwrap()),
        text.find("d'1")
    );
    let svg = source::derive(text, RenderFormat::Svg, true).unwrap();
    assert!(svg.text.contains("c'1"));
    assert!(!svg.text.contains("d'1"));
    assert_eq!(
        svg.original_byte(svg.text.find("c'1").unwrap()),
        text.find("c'1")
    );
    assert!(source::validate_render_structure(&midi.text).is_ok());
    assert!(source::validate_render_structure(&svg.text).is_ok());
    for text in [
        "\\score { c'1 \\layout {} } \\score { d'1 \\layout {} }",
        "\\score { c'1 \\midi {} } \\score { d'1 \\midi {} }",
        "\\book { \\score { c'1 \\layout {} } \\score { d'1 \\midi {} } }",
    ] {
        assert_eq!(
            source::derive(text, RenderFormat::Midi, true)
                .unwrap_err()
                .code,
            ErrorCode::UnsupportedSource
        );
    }
}

#[test]
fn musicxml_source_paths_are_not_carried_into_converted_headers() {
    let input = br#"<?xml version="1.0"?>
<score-partwise>
  <identification>
    <source>/var/lib/ossia-omr/jobs/example/input.pdf</source>
    <miscellaneous>
      <miscellaneous-field name="source-file">/var/lib/ossia-omr/jobs/example/input.pdf</miscellaneous-field>
    </miscellaneous>
  </identification>
</score-partwise>
"#;
    let stripped = source::strip_musicxml_source_paths(input);
    let text = String::from_utf8(stripped).unwrap();
    assert!(!text.contains("/var/lib/ossia-omr"));
    assert!(text.contains("<score-partwise>"));
    assert!(text.contains("<miscellaneous>"));
}

#[test]
fn musicxml_conversion_activates_only_the_generated_tempo_block() {
    let converted = "\\score { { c'1 }\n  % To create MIDI output, uncomment the following line:\n  % \\midi { \\tempo 4 = 96.5 }\n}\n";
    let expected = "\\score { { c'1 }\n  \\midi { \\tempo 4 = 96.5 }\n}\n";
    assert_eq!(
        source::normalize_conversion(converted, Path::new("/scratch"), InputFormat::MusicXml)
            .unwrap(),
        expected
    );
    assert_eq!(
        source::normalize_conversion(converted, Path::new("/scratch"), InputFormat::Midi).unwrap(),
        converted
    );
    for unexpected in ["\\score { c'1 }".to_owned(), converted.repeat(2)] {
        assert!(source::normalize_conversion(
            &unexpected,
            Path::new("/scratch"),
            InputFormat::MusicXml
        )
        .is_err());
    }
}

#[test]
fn converter_markup_does_not_block_anchors_or_midi() {
    let text = r#"\pointAndClickOff
#(define-markup-command (square layout props arg)
   (markup?)
   (interpret-markup layout props
     #{ \markup \box \with-dimension-from #X \rotate #90 #arg #arg #}))
\score { { c'1 } \layout {} }
"#;
    for format in [RenderFormat::Svg, RenderFormat::Pdf, RenderFormat::Midi] {
        let derived = source::derive(text, format, true).unwrap();
        assert!(!derived.text.contains("\\pointAndClickOff"));
        let note = derived.text.find("c'1").unwrap();
        assert_eq!(derived.original_byte(note), text.find("c'1"));
        assert!(derived.text.contains("#{ \\markup \\box"));
        if format == RenderFormat::Midi {
            assert_eq!(derived.text.matches("\\midi {}").count(), 1);
        }
    }
}

#[test]
fn midi_derivation_removes_chord_name_contexts() {
    let text = r#"harmonies = \chordmode { e1 }
\score { <<
  \context ChordNames = "chords" { \chordmode { c1 } }
  \new ChordNames \with { \override ChordName.font-size = #-2 } { \chordmode { d1 } }
  \new ChordNames \harmonies
  \new Staff { c'1 }
>> \layout {} }
"#;
    let derived = source::derive(text, RenderFormat::Midi, true).unwrap();
    assert!(!derived.text.contains("ChordNames"));
    assert!(!derived.text.contains("\\harmonies"));
    assert!(derived.text.contains("\\new Staff"));
    assert!(derived.text.contains("\\midi {}"));
}

#[test]
fn derived_text_respects_comments_strings_scheme_and_actual_score_boundaries() {
    let text = r#"\pointAndClickOff
%{ \score { fake } %{ nested %} %}
name = "\\score { escaped \" quote"
#(display "}")
\score { { c'1 } \layout {} }
\header { title = "tail" }"#;
    let derived = source::derive(text, RenderFormat::Midi, true).unwrap();
    assert_eq!(derived.text.matches("\\midi {}").count(), 1);
    assert!(derived.text.find("\\midi {}").unwrap() < derived.text.find("\\header").unwrap());
    for (byte, ch) in text.char_indices() {
        if let Some(mapped) = derived
            .text
            .char_indices()
            .find_map(|(d, _)| (derived.original_byte(d) == Some(byte)).then_some(d))
        {
            if byte >= "\\pointAndClickOff".len() {
                assert_eq!(derived.text[mapped..].chars().next(), Some(ch));
            }
        }
    }
    assert_eq!(text.matches("\\midi").count(), 0);
    for opaque in [
        "{ c'1 }",
        "\\score { c'1 } \\score { d'1 }",
        "\\score \\Music",
        "\\score { c'1",
        "\\score { #{ c #} }",
    ] {
        assert_eq!(
            source::derive(opaque, RenderFormat::Midi, true)
                .unwrap_err()
                .code,
            ErrorCode::UnsupportedSource
        );
    }
    let active = "\\score { { c'1 } \\midi { \\tempo 4 = 123 } }";
    assert_eq!(
        source::derive(active, RenderFormat::Midi, true)
            .unwrap()
            .text,
        active
    );
}

#[test]
fn svg_links_map_original_unicode_and_never_map_library_files() {
    let original = "\\pointAndClickOff\r\n\\score { { \\mark \\markup { \"中文😀\" }\tc'1 } }\r\n";
    let derived = source::derive(original, RenderFormat::Midi, true).unwrap();
    let byte = derived.text.find("c'1").unwrap();
    let p = ossia_score::SourceMap::new(&derived.text)
        .position(byte)
        .unwrap();
    let start = derived.text[..byte].rfind('\n').unwrap() + 1;
    let column = derived.text[start..byte].chars().fold(1, |n, c| {
        if c == '\t' {
            n + 8 - (n - 1) % 8
        } else {
            n + 1
        }
    });
    let svg = format!("<svg xmlns:xlink='http://www.w3.org/1999/xlink'><a xlink:href='textedit:///private/tmp/job/score.ly:{}:{}:{}'><text>A &amp; B</text></a><a xlink:href='textedit:///engine/english.ly:1:0:1'/></svg>",p.line,p.column-1,column);
    let normalized = source::normalize_svg(&svg, &derived, original).unwrap();
    assert!(normalized.contains(&format!(
        "textedit://score.ly:{}:{}:{}",
        p.line,
        p.column - 1,
        column
    )));
    assert!(!normalized.contains("/private/tmp") && !normalized.contains("english.ly"));
}

#[test]
fn svg_percentage_alpha_is_normalized_without_rewriting_other_content() {
    let original = "\\score { { c'1 } }\n";
    let derived = source::derive(original, RenderFormat::Svg, true).unwrap();
    let svg = r#"<svg><g color="rgba(100%, 100%, 100%, 100.0000%)"><rect fill="rgb(100% 100% 100% / 50%)"/><rect fill="rgba(255, 255, 255, 1)"/></g><text>rgba(1, 2, 3, 50%)</text></svg>"#;
    let normalized = source::normalize_svg(svg, &derived, original).unwrap();
    assert!(normalized.contains(r#"color="rgba(100%, 100%, 100%, 1)""#));
    assert!(normalized.contains(r#"fill="rgba(100%, 100%, 100%, 0.5)""#));
    assert!(normalized.contains(r#"fill="rgba(255, 255, 255, 1)""#));
    assert!(normalized.contains("rgba(1, 2, 3, 50%)"));
}

#[test]
fn malformed_svg_reports_parser_position() {
    let original = "\\score { { c'1 } }\n";
    let derived = source::derive(original, RenderFormat::Svg, true).unwrap();
    let error = source::normalize_svg("<svg><g></a></svg>", &derived, original).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(error.message, "engine produced malformed SVG");
    let detail = error.detail.unwrap();
    assert!(detail["position"].is_number());
    assert!(detail["error"]
        .as_str()
        .is_some_and(|error| !error.is_empty()));
}

#[test]
fn print_derivation_hides_only_the_default_tagline() {
    let without_tagline =
        "\\version \"2.26.0\"\n\\header { title = \"Scale\" }\n\\score { { c'1 } \\layout {} }\n";
    let derived = source::derive(without_tagline, RenderFormat::Svg, false).unwrap();
    assert_eq!(
        derived.text,
        format!("{without_tagline}\n\\paper {{ tagline = ##f }}\n")
    );

    let explicit =
        "\\version \"2.26.0\"\n\\header { tagline = \"Ossia\" }\n\\score { { c'1 } \\layout {} }\n";
    assert_eq!(
        source::derive(explicit, RenderFormat::Pdf, true)
            .unwrap()
            .text,
        explicit
    );
    let midi = source::derive(without_tagline, RenderFormat::Midi, true).unwrap();
    assert!(!midi.text.contains("tagline"));
    assert!(midi.text.contains("\\midi {}"));
}

#[test]
fn interactive_note_metadata_preserves_pitch_and_musical_moments() {
    let original = "\\score { { c'4 } }\n";
    let derived = source::derive(original, RenderFormat::Svg, true).unwrap();
    let svg = r#"<svg><a href="ossia:NoteHead:0:-1:-1:head::related:60.5,3,16,-1,8"><path d="M0 0h1v1h-1z"/></a></svg>"#;
    let normalized = source::normalize_svg(svg, &derived, original).unwrap();
    assert!(normalized.contains(r#"data-ossia-note="60.5,3,16,-1,8""#));
    assert!(normalized.contains(r#"data-ossia-id="head""#));
    assert!(normalized.contains(r#"data-ossia-related="related""#));
}

#[test]
fn interactive_rest_metadata_preserves_start_and_duration() {
    let original = "\\score { { r1 } }\n";
    let derived = source::derive(original, RenderFormat::Svg, true).unwrap();
    let svg =
        r#"<svg><a href="ossia:Rest:0:-1:-1:rest::::5,1,1,1"><path d="M0 0h1v1h-1z"/></a></svg>"#;
    let normalized = source::normalize_svg(svg, &derived, original).unwrap();
    assert!(normalized.contains(r#"data-ossia-timing="5,1,1,1""#));
    assert!(!normalized.contains("data-ossia-note="));
}
