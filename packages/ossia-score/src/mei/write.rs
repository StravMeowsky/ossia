//! Write a walked timeline as MEI for the editing preview.
use super::attrs::{accid, at_or_before, clef_attrs, dur, escape, keysig, measures, Measure};
use super::walk::{GroupKind, Item, Timeline};
use crate::note_edit::events::{values_for, Event, EventKind, Frac};
use crate::note_edit::spelling::key_alter;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

/// Pitches of one written note or chord: diatonic index and alteration per head.
pub(crate) type Heads = Vec<(i32, i32)>;

struct Writer<'a> {
    timeline: &'a Timeline,
    events: &'a [Event],
    heads: &'a HashMap<usize, Heads>,
    ids: HashMap<usize, String>,
    /// Item index to the ids of its heads, for ties.
    head_ids: HashMap<usize, Vec<(String, i32)>>,
    used: HashMap<String, u32>,
    keys: Vec<(Frac, i32)>,
    measure_of: HashMap<usize, usize>,
    /// Control events by the measure of the element they start on.
    controls: BTreeMap<usize, String>,
}

impl Writer<'_> {
    fn id(&mut self, item: usize) -> String {
        if let Some(id) = self.ids.get(&item) {
            return id.clone();
        }
        let base = format!("o{}", self.events[self.timeline.items[item].event].start);
        let count = self.used.entry(base.clone()).or_insert(0);
        *count += 1;
        let id = if *count == 1 {
            base
        } else {
            format!("{base}-{count}")
        };
        self.ids.insert(item, id.clone());
        id
    }

    fn element(&mut self, index: usize, alters: &mut HashMap<(i32, i32), i32>, out: &mut String) {
        let item: &Item = &self.timeline.items[index];
        let event = &self.events[item.event];
        let id = self.id(index);
        if item.measure_rest {
            let _ = write!(out, r#"<mRest xml:id="{id}"/>"#);
            return;
        }
        let value = if event.count > 1 || event.scale != Frac::new(1, 1) {
            item.length
        } else {
            event.value
        };
        let grace = if item.grace { r#" grace="unacc""# } else { "" };
        let artics: String = item
            .post
            .artics
            .iter()
            .map(|a| format!(r#"<artic artic="{a}"/>"#))
            .collect();
        match event.kind {
            EventKind::Rest | EventKind::MeasureRest | EventKind::Skip => {
                let _ = write!(out, r#"<rest xml:id="{id}" {}/>"#, dur(value));
            }
            EventKind::Note | EventKind::Chord => {
                let heads = self.heads.get(&event.start).cloned().unwrap_or_default();
                if heads.is_empty() {
                    let _ = write!(out, r#"<space xml:id="{id}" {}/>"#, dur(value));
                    return;
                }
                let fifths = at_or_before(&self.keys, item.time).unwrap_or(0);
                let chord = heads.len() > 1;
                if chord {
                    let _ = write!(out, r#"<chord xml:id="{id}" {}{grace}>"#, dur(value));
                }
                let mut ids = Vec::new();
                for (k, (di, alter)) in heads.iter().enumerate() {
                    let step = di.rem_euclid(7);
                    let octave = di.div_euclid(7) + 3;
                    let expected = alters
                        .get(&(step, octave))
                        .copied()
                        .unwrap_or_else(|| key_alter(fifths, step));
                    let shown = if *alter != expected {
                        alters.insert((step, octave), *alter);
                        format!(r#" accid="{}""#, accid(*alter))
                    } else if *alter != 0 {
                        let sounding = if *alter == 2 { "ss" } else { accid(*alter) };
                        format!(r#" accid.ges="{sounding}""#)
                    } else {
                        String::new()
                    };
                    // A chord head is named by its own source offset, so it can be selected alone.
                    let head_id = match event.members.get(k).filter(|_| chord) {
                        Some(member) => {
                            let suffix = id.find('-').map_or("", |p| &id[p..]);
                            format!("o{}{suffix}", member.0)
                        }
                        None => id.clone(),
                    };
                    let pname = ["c", "d", "e", "f", "g", "a", "b"][step as usize];
                    let attrs = if chord {
                        String::new()
                    } else {
                        format!(" {}{grace}", dur(value))
                    };
                    let _ = write!(
                        out,
                        r#"<note xml:id="{head_id}" pname="{pname}" oct="{octave}"{shown}{attrs}>"#
                    );
                    if !chord {
                        out.push_str(&artics);
                    }
                    out.push_str("</note>");
                    ids.push((
                        head_id,
                        12 * octave + [0, 2, 4, 5, 7, 9, 11][step as usize] + alter,
                    ));
                }
                if chord {
                    out.push_str(&artics);
                    out.push_str("</chord>");
                }
                self.head_ids.insert(index, ids);
            }
        }
    }

    fn layer(&mut self, measure: &Measure, items: &[usize], clefs: &[(Frac, String)]) -> String {
        let mut out = String::new();
        let mut cursor = measure.start;
        let mut tuplet: Option<u32> = None;
        let mut alters = HashMap::new();
        let mut clefs = clefs.iter().peekable();
        for &index in items {
            let item = &self.timeline.items[index];
            let group = item.tuplet.map(|t| t.0);
            if tuplet.is_some() && tuplet != group {
                out.push_str("</tuplet>");
                tuplet = None;
            }
            if item.time > cursor {
                for value in
                    values_for(item.time.sub(cursor).unwrap_or(Frac::new(0, 1))).unwrap_or_default()
                {
                    let _ = write!(out, "<space {}/>", dur(value.length()));
                }
                cursor = item.time;
            }
            while let Some((_, name)) = clefs.next_if(|(t, _)| *t <= item.time) {
                let _ = write!(out, "<clef {}/>", clef_attrs(name));
            }
            if tuplet.is_none() {
                if let Some((group, num, base)) = item.tuplet {
                    let _ = write!(out, r#"<tuplet num="{num}" numbase="{base}">"#);
                    tuplet = Some(group);
                }
            }
            self.element(index, &mut alters, &mut out);
            if !item.grace {
                cursor = cursor.max(item.time.add(item.length));
            }
        }
        if tuplet.is_some() {
            out.push_str("</tuplet>");
        }
        out
    }

    /// Ties, slurs, hairpins, dynamics and fermatas, after every element has an id.
    fn control(&mut self, item: usize, text: String) {
        let measure = self.measure_of.get(&item).copied().unwrap_or(0);
        self.controls.entry(measure).or_default().push_str(&text);
    }

    fn controls(&mut self, order: &BTreeMap<(usize, u32), Vec<usize>>) {
        for ((staff, _), items) in order {
            let n = staff + 1;
            let mut slurs: Vec<(usize, String)> = Vec::new();
            for (pos, &index) in items.iter().enumerate() {
                let post = self.timeline.items[index].post.clone();
                let Some(id) = self.ids.get(&index).cloned() else {
                    continue;
                };
                for _ in 0..post.slur_ends {
                    if let Some((item, start)) = slurs.pop() {
                        self.control(
                            item,
                            format!(r##"<slur staff="{n}" startid="#{start}" endid="#{id}"/>"##),
                        );
                    }
                }
                for _ in 0..post.slur_starts {
                    slurs.push((index, id.clone()));
                }
                for dynamic in &post.dynamics {
                    self.control(
                        index,
                        format!(r##"<dynam staff="{n}" startid="#{id}">{dynamic}</dynam>"##),
                    );
                }
                if post.fermata {
                    self.control(
                        index,
                        format!(r##"<fermata staff="{n}" startid="#{id}"/>"##),
                    );
                }
                let following = &items[pos + 1..];
                if post.tie {
                    let heads = self.head_ids.get(&index).cloned().unwrap_or_default();
                    let next = following.iter().find_map(|i| self.head_ids.get(i)).cloned();
                    for (start, pitch) in heads {
                        let end = next
                            .as_ref()
                            .and_then(|n| n.iter().find(|h| h.1 == pitch))
                            .cloned();
                        if let Some((end, _)) = end {
                            let text = format!(
                                r##"<tie staff="{n}" startid="#{start}" endid="#{end}"/>"##
                            );
                            self.control(index, text);
                        }
                    }
                }
                for mark in post.hairpins.iter().filter(|m| **m != '!') {
                    let end = following.iter().find(|i| {
                        let p = &self.timeline.items[**i].post;
                        !p.hairpins.is_empty() || !p.dynamics.is_empty()
                    });
                    if let Some(end) = end.and_then(|e| self.ids.get(e)).cloned() {
                        let form = if *mark == '<' { "cres" } else { "dim" };
                        let text = format!(
                            r##"<hairpin staff="{n}" form="{form}" startid="#{id}" endid="#{end}"/>"##
                        );
                        self.control(index, text);
                    }
                }
            }
        }
    }
}

pub(crate) fn write_mei(
    timeline: &Timeline,
    events: &[Event],
    heads: &HashMap<usize, Heads>,
    title: Option<&str>,
) -> String {
    let measures = measures(timeline);
    let mut keys = timeline.keys.clone();
    keys.sort_by_key(|k| k.0);
    let mut writer = Writer {
        timeline,
        events,
        heads,
        ids: HashMap::new(),
        head_ids: HashMap::new(),
        used: HashMap::new(),
        keys: keys.clone(),
        measure_of: HashMap::new(),
        controls: BTreeMap::new(),
    };
    // Items by measure, staff and layer, in time order.
    let mut placed: BTreeMap<(usize, usize, u32), Vec<usize>> = BTreeMap::new();
    let mut order: BTreeMap<(usize, u32), Vec<usize>> = BTreeMap::new();
    let mut sorted: Vec<usize> = (0..timeline.items.len()).collect();
    sorted.sort_by_key(|&i| timeline.items[i].time);
    for index in sorted {
        let item = &timeline.items[index];
        let measure = measures
            .iter()
            .rposition(|m| m.start <= item.time)
            .unwrap_or(0);
        writer.measure_of.insert(index, measure);
        placed
            .entry((measure, item.staff, item.layer))
            .or_default()
            .push(index);
        order
            .entry((item.staff, item.layer))
            .or_default()
            .push(index);
    }
    let first_clef = |staff: usize| {
        timeline
            .clefs
            .iter()
            .rfind(|(s, t, _)| *s == staff && t.is_zero())
            .map_or("treble", |(_, _, name)| name.as_str())
    };
    // One part per measure, still open for its control events.
    let mut parts: Vec<String> = Vec::with_capacity(measures.len());
    let mut last_key = at_or_before(&keys, Frac::new(0, 1)).unwrap_or(0);
    let mut last_meter = measures.first().map_or((4, 4), |m| m.meter);
    for (number, measure) in measures.iter().enumerate() {
        let mut body = String::new();
        let key = at_or_before(&keys, measure.start).unwrap_or(0);
        if number > 0 && (key != last_key || measure.meter != last_meter) {
            let mut change = String::new();
            if key != last_key {
                let _ = write!(change, r#" keysig="{}""#, keysig(key));
            }
            if measure.meter != last_meter {
                let _ = write!(
                    change,
                    r#" meter.count="{}" meter.unit="{}""#,
                    measure.meter.0, measure.meter.1
                );
            }
            let _ = write!(body, "<scoreDef{change}/>");
            (last_key, last_meter) = (key, measure.meter);
        }
        let full = Frac::new(measure.meter.0, measure.meter.1);
        let metcon = if measure.length == full {
            ""
        } else {
            r#" metcon="false""#
        };
        let _ = write!(body, r#"<measure n="{}"{metcon}>"#, number + 1);
        let end = measure.start.add(measure.length);
        for staff in 0..timeline.staves.len() {
            let _ = write!(body, r#"<staff n="{}">"#, staff + 1);
            let layers: Vec<u32> = placed
                .range((number, staff, 0)..=(number, staff, u32::MAX))
                .map(|((_, _, layer), _)| *layer)
                .collect();
            let clefs: Vec<(Frac, String)> = timeline
                .clefs
                .iter()
                .filter(|(s, t, _)| *s == staff && *t > measure.start && *t < end)
                .map(|(_, t, name)| (*t, name.clone()))
                .collect();
            if layers.is_empty() {
                body.push_str(r#"<layer n="1"><mSpace/></layer>"#);
            }
            for (n, layer) in layers.iter().enumerate() {
                let items = placed[&(number, staff, *layer)].clone();
                let clefs = if n == 0 { clefs.as_slice() } else { &[] };
                let content = writer.layer(measure, &items, clefs);
                let _ = write!(body, r#"<layer n="{}">{content}</layer>"#, n + 1);
            }
            body.push_str("</staff>");
        }
        parts.push(body);
    }
    writer.controls(&order);
    let mut body = String::new();
    for (number, part) in parts.iter().enumerate() {
        body.push_str(part);
        body.push_str(writer.controls.get(&number).map_or("", String::as_str));
        body.push_str("</measure>");
    }
    let mut staff_defs = String::new();
    let mut open_group: Option<usize> = None;
    for (index, staff) in timeline.staves.iter().enumerate() {
        if staff.group != open_group {
            if open_group.is_some() {
                staff_defs.push_str("</staffGrp>");
            }
            if let Some(group) = staff.group {
                let symbol = match timeline.groups.get(group) {
                    Some(GroupKind::Bracket) => "bracket",
                    _ => "brace",
                };
                let _ = write!(
                    staff_defs,
                    r#"<staffGrp symbol="{symbol}" bar.thru="true">"#
                );
            }
            open_group = staff.group;
        }
        let _ = write!(
            staff_defs,
            r#"<staffDef n="{}" lines="5"><clef {}/></staffDef>"#,
            index + 1,
            clef_attrs(first_clef(index))
        );
    }
    if open_group.is_some() {
        staff_defs.push_str("</staffGrp>");
    }
    let title = title.map(escape).unwrap_or_default();
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8"?>"#,
            r#"<mei xmlns="http://www.music-encoding.org/ns/mei" meiversion="5.0">"#,
            r#"<meiHead><fileDesc><titleStmt><title>{title}</title></titleStmt><pubStmt/></fileDesc></meiHead>"#,
            r#"<music><body><mdiv><score>"#,
            r#"<scoreDef keysig="{key}" meter.count="{count}" meter.unit="{unit}"><staffGrp>{staves}</staffGrp></scoreDef>"#,
            r#"<section>{body}</section></score></mdiv></body></music></mei>"#
        ),
        title = title,
        key = keysig(at_or_before(&keys, Frac::new(0, 1)).unwrap_or(0)),
        count = measures.first().map_or(4, |m| m.meter.0),
        unit = measures.first().map_or(4, |m| m.meter.1),
        staves = staff_defs,
        body = body,
    )
}
