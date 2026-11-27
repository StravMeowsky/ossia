//! Measures and attribute values for the MEI writer.
use super::walk::Timeline;
use crate::note_edit::events::{values_for, Frac};

pub(super) struct Measure {
    pub(super) start: Frac,
    pub(super) length: Frac,
    pub(super) meter: (u64, u64),
}

pub(super) fn at_or_before<T: Copy>(list: &[(Frac, T)], time: Frac) -> Option<T> {
    list.iter()
        .filter(|(t, _)| *t <= time)
        .last()
        .map(|(_, v)| *v)
}

pub(super) fn measures(timeline: &Timeline) -> Vec<Measure> {
    let end = timeline
        .items
        .iter()
        .map(|i| i.time.add(i.length))
        .max()
        .unwrap_or(Frac::new(0, 1));
    let mut meters: Vec<(Frac, (u64, u64))> = timeline
        .times
        .iter()
        .map(|(t, n, d)| (*t, (*n, *d)))
        .collect();
    meters.sort_by_key(|m| m.0);
    let mut out: Vec<Measure> = Vec::new();
    let mut time = Frac::new(0, 1);
    while time < end && out.len() < 10_000 {
        let meter = at_or_before(&meters, time).unwrap_or((4, 4));
        let mut length = timeline
            .partials
            .iter()
            .find(|(t, _)| *t == time)
            .map_or(Frac::new(meter.0, meter.1), |(_, l)| *l);
        // A partial or a meter change inside this measure starts a new one there.
        let next = time.add(length);
        let cut = timeline
            .partials
            .iter()
            .map(|(t, _)| *t)
            .chain(meters.iter().map(|(t, _)| *t))
            .filter(|t| *t > time && *t < next)
            .min();
        if let Some(cut) = cut {
            length = cut.sub(time).unwrap_or(length);
        }
        if length.is_zero() {
            break;
        }
        out.push(Measure {
            start: time,
            length,
            meter,
        });
        time = time.add(length);
    }
    out
}

pub(super) fn clef_attrs(name: &str) -> &'static str {
    match name {
        "bass" | "F" => r#"shape="F" line="4""#,
        "alto" | "C" | "viola" => r#"shape="C" line="3""#,
        "tenor" => r#"shape="C" line="4""#,
        "soprano" => r#"shape="C" line="1""#,
        "mezzosoprano" => r#"shape="C" line="2""#,
        "baritone" => r#"shape="F" line="3""#,
        "treble_8" | "G_8" | "violin_8" => r#"shape="G" line="2" dis="8" dis.place="below""#,
        "bass_8" | "F_8" => r#"shape="F" line="4" dis="8" dis.place="below""#,
        "percussion" => r#"shape="perc" line="3""#,
        _ => r#"shape="G" line="2""#,
    }
}

pub(super) fn accid(alter: i32) -> &'static str {
    match alter {
        -2 => "ff",
        -1 => "f",
        1 => "s",
        2 => "x",
        _ => "n",
    }
}

pub(super) fn keysig(fifths: i32) -> String {
    match fifths {
        0 => "0".into(),
        f if f > 0 => format!("{f}s"),
        f => format!("{}f", -f),
    }
}

/// Written value as `dur` and `dots` attributes.
pub(super) fn dur(value: Frac) -> String {
    let parts = values_for(value).unwrap_or_default();
    let Some(first) = parts.first() else {
        return r#"dur="4""#.into();
    };
    let base = 1u32 << first.log;
    if first.dots > 0 {
        format!(r#"dur="{base}" dots="{}""#, first.dots)
    } else {
        format!(r#"dur="{base}""#)
    }
}

pub(super) fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
