use super::*;
use ossia_types::{
    GeometryBounds, GeometryElement, GeometryPage, LocationCapability, SelectionRegion,
};

fn target(id: &str) -> SelectionTarget {
    SelectionTarget::Element {
        element_id: id.into(),
    }
}

fn region(id: &str, kind: &str) -> SelectionRegion {
    SelectionRegion {
        target: target(id),
        label: format!("{kind}: {id}"),
        element_kind: Some(kind.into()),
        spans: Vec::new(),
        insertion_anchor: None,
        staff_index: Some(0),
        voice: Some("music".into()),
        reason: None,
        occurrences: Vec::new(),
    }
}

fn element(id: &str, kind: &str, linked: &[&str]) -> GeometryElement {
    GeometryElement {
        region: region(id, kind),
        system_index: Some(0),
        bounds: GeometryBounds {
            x0: 0.,
            y0: 0.,
            x1: 1.,
            y1: 1.,
        },
        priority: 0,
        hit_bounds: Vec::new(),
        graphic_ids: Vec::new(),
        linked_targets: linked.iter().map(|id| target(id)).collect(),
    }
}

fn group(start: f64, element_ids: &[&str]) -> PlaybackNoteGroup {
    PlaybackNoteGroup {
        element_ids: element_ids.iter().map(|id| (*id).into()).collect(),
        pitches: vec![81],
        staff_index: Some(0),
        voice: Some("music".into()),
        start_seconds: Some(start),
        duration_seconds: Some(0.5),
        playhead_seconds: None,
        part_id: None,
    }
}

#[test]
fn chord_ties_with_shared_source_target_keep_independent_pitch_chains() {
    let pages = vec![GeometryPage {
        page: 1,
        view_box: None,
        capability: LocationCapability::Measure,
        reason: None,
        regions: Vec::new(),
        elements: vec![
            element("a1", "NoteHead", &["a1", "a2", "tie"]),
            element("c1", "NoteHead", &["c1", "c2", "tie"]),
            element("a2", "NoteHead", &["a2", "a1", "tie"]),
            element("c2", "NoteHead", &["c2", "c1", "tie"]),
            element("tie", "Tie", &["tie"]),
        ],
    }];
    let mut groups = vec![group(1., &["a1"]), group(1., &["c1"])];
    groups[1].pitches = vec![85];
    merge_tied_groups(&mut groups, &pages);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].element_ids, ["a1", "a2"]);
    assert_eq!(groups[1].element_ids, ["c1", "c2"]);
}

#[test]
fn tied_continuation_stays_with_its_attack_not_the_following_note() {
    let pages = vec![GeometryPage {
        page: 1,
        view_box: None,
        capability: LocationCapability::Measure,
        reason: None,
        regions: Vec::new(),
        elements: vec![
            element("start", "NoteHead", &["start", "tie", "continuation"]),
            element(
                "continuation",
                "NoteHead",
                &["continuation", "tie", "start"],
            ),
            element("following", "NoteHead", &["following"]),
            element("tie", "Tie", &["tie"]),
        ],
    }];
    let mut groups = vec![
        group(1., &["start"]),
        group(1.5, &["continuation", "following"]),
    ];

    merge_tied_groups(&mut groups, &pages);

    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].element_ids, ["continuation", "start"]);
    assert_eq!(groups[1].element_ids, ["following"]);
}

#[test]
fn a_group_containing_only_a_tied_continuation_is_removed() {
    let pages = vec![GeometryPage {
        page: 1,
        view_box: None,
        capability: LocationCapability::Measure,
        reason: None,
        regions: Vec::new(),
        elements: vec![
            element("start", "NoteHead", &["start", "tie", "continuation"]),
            element(
                "continuation",
                "NoteHead",
                &["continuation", "tie", "start"],
            ),
            element("tie", "Tie", &["tie"]),
        ],
    }];
    let mut groups = vec![group(1., &["start"]), group(1.5, &["continuation"])];

    merge_tied_groups(&mut groups, &pages);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].element_ids, ["continuation", "start"]);
}
