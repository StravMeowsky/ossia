use super::*;

#[test]
fn unmatched_visual_note_is_not_attached_to_a_different_onset() {
    let onset = |id: &str, x| VisualOnset {
        page: 1,
        system_index: 0,
        x,
        expected: 0.,
        element_ids: vec![id.into()],
        pitches: HashSet::new(),
        staff_index: Some(0),
        voice: Some("voice".into()),
    };
    let visual = vec![onset("first", 10.), onset("unmatched", 20.)];
    let performed = vec![PerformanceOnset {
        start_seconds: 0.,
        end_seconds: 1.,
        pitches: vec![60],
    }];
    let mut groups = Vec::new();
    push_visual_groups(&mut groups, &visual, &performed, &[Some(0), None]);
    assert_eq!(groups[0].element_ids, vec!["first"]);
}

#[test]
fn conflicting_visual_onsets_do_not_share_one_performance() {
    let onset = |id: &str, x| VisualOnset {
        page: 1,
        system_index: 0,
        x,
        expected: 0.,
        element_ids: vec![id.into()],
        pitches: HashSet::new(),
        staff_index: Some(0),
        voice: Some("voice".into()),
    };
    let a = onset("a", 10.);
    let b = onset("b", 20.);
    let mut matches = BTreeMap::new();
    add_unique_onset(&mut matches, 0, &a);
    assert_eq!(matches[&0].unwrap().element_ids, vec!["a"]);
    add_unique_onset(&mut matches, 0, &b);
    assert!(matches[&0].is_none());
}

#[test]
fn tied_continuations_do_not_consume_or_join_a_new_midi_attack() {
    use crate::{MidiDivision, MidiFile, MidiNote, MidiTrack};
    use ossia_types::{GeometryBounds, GeometryPage, LocationCapability, SelectionRegion};
    for continuation_x in [20., 30.] {
        let target = |id: &str| SelectionTarget::Element {
            element_id: id.into(),
        };
        let element = |id: &str, kind: &str, x, linked: &[&str]| GeometryElement {
            region: SelectionRegion {
                target: target(id),
                label: id.into(),
                element_kind: Some(kind.into()),
                spans: Vec::new(),
                insertion_anchor: None,
                staff_index: Some(0),
                voice: None,
                reason: None,
                occurrences: Vec::new(),
            },
            system_index: Some(0),
            bounds: GeometryBounds {
                x0: x,
                x1: x + 1.,
                y0: 10.,
                y1: 11.,
            },
            priority: 0,
            hit_bounds: Vec::new(),
            graphic_ids: Vec::new(),
            linked_targets: linked.iter().map(|id| target(id)).collect(),
        };
        let pages = vec![GeometryPage {
            page: 1,
            view_box: None,
            capability: LocationCapability::AnchorOnly,
            reason: None,
            regions: Vec::new(),
            elements: vec![
                element(
                    "attack",
                    "NoteHead",
                    10.,
                    &["attack", "continuation", "tie"],
                ),
                element(
                    "continuation",
                    "NoteHead",
                    continuation_x,
                    &["continuation", "attack", "tie"],
                ),
                element("next", "NoteHead", 30., &["next"]),
                element("tie", "Tie", 15., &["tie"]),
            ],
        }];
        let midi = MidiFile {
            format: 1,
            division: MidiDivision::Ppqn {
                ticks_per_quarter: 480,
            },
            tracks: vec![MidiTrack {
                track: 0,
                name: None,
                programs: Vec::new(),
                notes: vec![
                    MidiNote {
                        note: 60,
                        time: 0,
                        channel: 0,
                        duration: Some(960),
                        duration_reason: None,
                    },
                    MidiNote {
                        note: 62,
                        time: 960,
                        channel: 0,
                        duration: Some(480),
                        duration_reason: None,
                    },
                ],
            }],
            time_signatures: Vec::new(),
            tempos: Vec::new(),
        };
        let systems = vec![PlaybackSystemSpan {
            page: 1,
            system_index: 0,
            measure_start: 1,
            measure_end: 1,
            start_seconds: 0.,
            end_seconds: 1.5,
            x0: 10.,
            x1: 40.,
            y0: 0.,
            y1: 20.,
        }];
        let mut groups = Vec::new();
        let pitches = HashMap::new();
        let staves = HashMap::new();
        visual_note_groups(
            &pages,
            &midi,
            480,
            &mut groups,
            &ElementHints {
                pitches: &pitches,
                staves: &staves,
            },
            &systems,
        );
        assert_eq!(groups.len(), 2);
        let attack = groups
            .iter()
            .find(|group| group.start_seconds == Some(0.))
            .unwrap();
        assert_eq!(attack.element_ids, ["attack"]);
        let next = groups
            .iter()
            .find(|group| group.start_seconds == Some(1.))
            .unwrap();
        assert_eq!(next.element_ids, ["next"]);
        assert_eq!(next.pitches, [62]);
    }
}
