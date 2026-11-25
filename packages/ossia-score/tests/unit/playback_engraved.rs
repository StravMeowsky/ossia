use super::*;
use crate::{
    build_index, build_layout, selection_geometry, MidiDivision, MidiNote, MidiTrack, SvgPage,
};

fn fixture(pitches: &[u8]) -> (ScoreLayout, Vec<GeometryPage>, MidiFile) {
    let source = "music = { c'4 d'4 }\n\\score { \\new Staff \\music }";
    let index = build_index(source);
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
      <g data-ossia-kind="NoteHead" data-ossia-id="c" data-ossia-staff="0"
         data-ossia-source="{}:{}" data-ossia-note="72,0,1,0,1" data-ossia-bounds="10 10 12 12"/>
      <g data-ossia-kind="NoteHead" data-ossia-id="d" data-ossia-staff="0"
         data-ossia-source="{}:{}" data-ossia-note="74,1,4,0,1" data-ossia-bounds="20 10 22 12"/>
    </svg>"#,
        source.find("c'4").unwrap(),
        source.find("c'4").unwrap() + 3,
        source.find("d'4").unwrap(),
        source.find("d'4").unwrap() + 3
    );
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let notes = pitches
        .iter()
        .enumerate()
        .map(|(i, pitch)| MidiNote {
            note: *pitch,
            time: (i % 2) as u64 * 480,
            channel: 0,
            duration: Some(480),
            duration_reason: None,
        })
        .collect();
    let midi = MidiFile {
        format: 1,
        division: MidiDivision::Ppqn {
            ticks_per_quarter: 480,
        },
        tracks: vec![MidiTrack {
            track: 0,
            name: None,
            programs: Vec::new(),
            notes,
        }],
        tempos: Vec::new(),
        time_signatures: Vec::new(),
    };
    (layout, pages.pages, midi)
}

#[test]
fn exact_events_bind_each_head_to_one_sounding_pitch_without_coordinates() {
    let (layout, pages, midi) = fixture(&[60, 62]);
    let mut groups = Vec::new();
    note_groups(&layout, &pages, &midi, 480, &HashMap::new(), &mut groups);
    groups.sort_by(|a, b| a.start_seconds.partial_cmp(&b.start_seconds).unwrap());
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].pitches, [60]);
    assert_eq!(groups[1].pitches, [62]);
    assert_eq!(groups[1].start_seconds, Some(0.5));
    assert!(groups.iter().all(|group| group.element_ids.len() == 1));
}

#[test]
fn ambiguous_octave_doubling_does_not_choose_a_performer_by_pitch_proximity() {
    let (layout, pages, midi) = fixture(&[60, 62, 72, 74]);
    let mut groups = Vec::new();
    note_groups(&layout, &pages, &midi, 480, &HashMap::new(), &mut groups);
    assert!(groups.is_empty());
}

#[test]
fn source_pitch_evidence_disambiguates_octave_doubling() {
    let (layout, pages, midi) = fixture(&[60, 62, 72, 74]);
    let hints = pages[0]
        .elements
        .iter()
        .filter_map(|element| {
            let SelectionTarget::Element { element_id } = &element.region.target else {
                return None;
            };
            Some((
                element_id.clone(),
                if element.graphic_ids.contains(&"c".into()) {
                    60
                } else {
                    62
                },
            ))
        })
        .collect();
    let mut groups = Vec::new();
    note_groups(&layout, &pages, &midi, 480, &hints, &mut groups);
    assert_eq!(groups.len(), 2);
    assert!(groups.iter().all(|group| group.pitches[0] < 72));
}

#[test]
fn muted_heads_remain_silent() {
    let (layout, pages, midi) = fixture(&[60, 62]);
    let id = match &pages[0].elements[0].region.target {
        SelectionTarget::Element { element_id } => element_id.clone(),
        _ => unreachable!(),
    };
    let mut groups = vec![PlaybackNoteGroup {
        element_ids: vec![id.clone()],
        pitches: Vec::new(),
        staff_index: Some(0),
        voice: Some("music".into()),
        start_seconds: Some(0.),
        duration_seconds: Some(0.5),
        part_id: None,
    }];
    note_groups(&layout, &pages, &midi, 480, &HashMap::new(), &mut groups);
    assert!(groups
        .iter()
        .find(|group| group.element_ids.contains(&id))
        .unwrap()
        .pitches
        .is_empty());
}

#[test]
fn moments_reject_negative_non_integral_and_zero_denominator_ticks() {
    let note = EngravedNote {
        pitch: 60.,
        numerator: 1,
        denominator: 4,
        grace_numerator: 0,
        grace_denominator: 1,
    };
    assert_eq!(tick(note, 480), Some(480));
    assert_eq!(
        tick(
            EngravedNote {
                numerator: -1,
                ..note
            },
            480
        ),
        None
    );
    assert_eq!(
        tick(
            EngravedNote {
                denominator: 7,
                ..note
            },
            480
        ),
        None
    );
    assert_eq!(
        tick(
            EngravedNote {
                denominator: 0,
                ..note
            },
            480
        ),
        None
    );
}

#[test]
fn accurate_groups_keep_their_joined_notation_targets() {
    let (layout, pages, midi) = fixture(&[60, 62]);
    let id = match &pages[0].elements[0].region.target {
        SelectionTarget::Element { element_id } => element_id.clone(),
        _ => unreachable!(),
    };
    let mut groups = vec![PlaybackNoteGroup {
        element_ids: vec![id.clone(), "paired-tab".into()],
        pitches: vec![60],
        staff_index: Some(0),
        voice: Some("music".into()),
        start_seconds: Some(0.),
        duration_seconds: Some(0.5),
        part_id: Some("0:0".into()),
    }];
    note_groups(&layout, &pages, &midi, 480, &HashMap::new(), &mut groups);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].element_ids, [id, "paired-tab".into()]);
    assert_eq!(groups[0].part_id.as_deref(), Some("0:0"));
}

#[test]
fn unison_attacks_keep_one_pitch_when_performer_release_is_ambiguous() {
    let (layout, pages, mut midi) = fixture(&[60, 62, 60, 62]);
    for note in midi.tracks[0].notes.iter_mut().skip(2) {
        note.duration = Some(240);
    }
    let mut groups = Vec::new();
    note_groups(&layout, &pages, &midi, 480, &HashMap::new(), &mut groups);
    assert_eq!(groups.len(), 2);
    assert!(groups.iter().all(|group| group.pitches.len() == 1
        && group.duration_seconds.is_none()
        && group.part_id.is_none()));
    assert_eq!(
        groups
            .iter()
            .map(|group| group.pitches[0])
            .collect::<HashSet<_>>(),
        HashSet::from([60, 62])
    );
}

#[test]
fn rests_use_engraved_moments_and_durations_across_tempo_changes() {
    let source = "music = { r4 r2. R1*2 }\n\\score { \\new Staff \\music }";
    let index = build_index(source);
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
        <g data-ossia-kind="Rest" data-ossia-id="quarter" data-ossia-staff="0"
           data-ossia-source="{}:{}" data-ossia-timing="0,1,1,4" data-ossia-bounds="10 10 12 12"/>
        <g data-ossia-kind="Rest" data-ossia-id="dotted" data-ossia-staff="0"
           data-ossia-source="{}:{}" data-ossia-timing="1,4,3,4" data-ossia-bounds="20 10 22 12"/>
        <g data-ossia-kind="MultiMeasureRest" data-ossia-id="multi" data-ossia-staff="0"
           data-ossia-source="{}:{}" data-ossia-timing="1,1,2,1" data-ossia-bounds="30 10 32 12"/>
        <g data-ossia-kind="MultiMeasureRest" data-ossia-id="multi-copy" data-ossia-staff="0"
           data-ossia-source="{}:{}" data-ossia-timing="2,1,2,1" data-ossia-bounds="40 10 42 12"/>
        </svg>"#,
        source.find("r4").unwrap(),
        source.find("r4").unwrap() + 2,
        source.find("r2.").unwrap(),
        source.find("r2.").unwrap() + 3,
        source.find("R1*2").unwrap(),
        source.find("R1*2").unwrap() + 4,
        source.find("R1*2").unwrap(),
        source.find("R1*2").unwrap() + 4,
    );
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let (_, _, mut midi) = fixture(&[]);
    midi.tempos = vec![crate::MidiTempo {
        tick: 480,
        microseconds_per_quarter: 1_000_000,
    }];
    let mut groups = Vec::new();
    note_groups(&layout, &pages, &midi, 480, &HashMap::new(), &mut groups);
    groups.sort_by(|a, b| a.start_seconds.partial_cmp(&b.start_seconds).unwrap());
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].start_seconds, Some(0.));
    assert_eq!(groups[0].duration_seconds, Some(0.5));
    assert_eq!(groups[1].start_seconds, Some(0.5));
    assert_eq!(groups[1].duration_seconds, Some(3.));
    assert_eq!(groups[2].start_seconds, Some(3.5));
    assert_eq!(groups[2].duration_seconds, Some(8.));
    assert!(groups
        .iter()
        .all(|group| group.pitches.is_empty() && group.part_id.is_none()));
    let playback = crate::build_playback(
        &index,
        &layout,
        &midi,
        ossia_types::ContentHash::of(b"rests"),
    )
    .unwrap();
    assert_eq!(playback.duration_seconds, 11.5);
}
