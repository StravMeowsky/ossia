use crate::{Token, TokenKind};
use ossia_types::{ContentHash, LocationCapability, SourceSpan};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

mod locator;
mod rhythm;
mod syntax;
pub use locator::resolve_locator;
pub(crate) use syntax::{region, unsupported_token, Assignment, Syntax};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Measure {
    pub measure_start: u32,
    pub measure_end: u32,
    pub span: SourceSpan,
}

/// One source measure in the order it is engraved on the page.
#[derive(Debug, Clone)]
pub(crate) struct RenderedMeasureRef {
    pub(crate) voice: String,
    pub(crate) measure: u32,
}

/// One engraved measure, including every parallel voice that occupies it.
#[derive(Debug, Clone)]
pub(crate) struct RenderedMeasure {
    pub(crate) measures: Vec<RenderedMeasureRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceRegion {
    pub name: String,
    pub span: SourceSpan,
    pub music_span: SourceSpan,
    pub measures: Vec<Measure>,
    #[serde(default)]
    pub measure_groups: Vec<Vec<Measure>>,
    pub capability: LocationCapability,
    pub reason: Option<String>,
}

impl VoiceRegion {
    pub(crate) fn indexed_measures(&self) -> impl Iterator<Item = &Measure> {
        let groups_empty = self.measure_groups.is_empty();
        self.measure_groups
            .iter()
            .flatten()
            .chain(self.measures.iter().filter(move |_| groups_empty))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StaffKind {
    #[default]
    Notation,
    Tablature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffVoices {
    pub staff_index: usize,
    #[serde(default)]
    pub kind: StaffKind,
    pub label: Option<String>,
    /// Literal instrument name declared on this staff, when unambiguous.
    #[serde(skip)]
    pub instrument_name: Option<String>,
    /// Literal MIDI instrument declared on this staff, when unambiguous.
    #[serde(skip)]
    pub midi_instrument_name: Option<String>,
    /// Source context shared by staves of one piano instrument.
    #[serde(skip)]
    pub piano_group: Option<usize>,
    pub voices: Vec<String>,
    /// An explicit context body's insertion point, when present in the source.
    #[serde(skip)]
    pub property_anchor: Option<SourceSpan>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreIndex {
    pub source_hash: ContentHash,
    pub voices: Vec<VoiceRegion>,
    pub staves: Vec<StaffVoices>,
    pub max_measures: u32,
    pub capability: LocationCapability,
    pub reason: Option<String>,
    #[serde(skip)]
    source: String,
    #[serde(skip)]
    rendered_measures: Vec<RenderedMeasure>,
}

impl ScoreIndex {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn voice(&self, name: &str) -> Option<&VoiceRegion> {
        self.voices.iter().find(|v| v.name == name)
    }

    /// Count measures only when every rendered staff has a reliable source index.
    /// Unused assignments do not contribute to the count.
    pub fn measure_count(&self) -> Result<u32, String> {
        if let Some(reason) = &self.reason {
            return Err(reason.clone());
        }
        if self.staves.is_empty() {
            return Err(self
                .reason
                .clone()
                .unwrap_or_else(|| "no reliable staff measure index".into()));
        }
        let mut count = 0;
        for staff in &self.staves {
            if staff.voices.is_empty() {
                return Err(format!(
                    "staff {} has no reliable voice index",
                    staff.staff_index
                ));
            }
            let mut seen = HashSet::new();
            if self.rendered_measures.is_empty()
                && staff.voices.iter().any(|name| !seen.insert(name))
            {
                return Err("repeated voice references have ambiguous measure order".into());
            }
            for name in &staff.voices {
                let voice = self
                    .voice(name)
                    .ok_or_else(|| format!("unresolved voice: {name}"))?;
                if voice.capability != LocationCapability::Measure {
                    return Err(voice
                        .reason
                        .clone()
                        .unwrap_or_else(|| format!("voice {name} cannot be reliably indexed")));
                }
                let end = voice
                    .indexed_measures()
                    .map(|measure| measure.measure_end)
                    .max()
                    .ok_or_else(|| format!("voice {name} has no indexed measures"))?;
                count = count.max(end);
            }
        }
        if !self.rendered_measures.is_empty() {
            return u32::try_from(self.rendered_measures.len())
                .map_err(|_| "rendered measure count exceeds supported range".into());
        }
        Ok(count)
    }

    pub(crate) fn measure_at(&self, byte: usize) -> Option<&Measure> {
        self.voices
            .iter()
            .flat_map(VoiceRegion::indexed_measures)
            .find(|m| byte >= m.span.byte_start && byte < m.span.byte_end)
    }

    /// Measures in engraved order, used to keep playback and the page grid in
    /// the same coordinate system when music variables are concatenated.
    pub(crate) fn rendered_measure_sequence(&self) -> &[RenderedMeasure] {
        &self.rendered_measures
    }

    /// Resolve a variable-local measure to its engraved measure number.
    ///
    /// A variable rendered more than once has several answers; `hint` picks the
    /// closest one when the caller already knows the element's page position.
    pub(crate) fn rendered_measure(
        &self,
        voice: &str,
        measure: u32,
        hint: Option<u32>,
    ) -> Option<u32> {
        let mut first = None;
        let mut closest = None;
        for (index, rendered) in self.rendered_measures.iter().enumerate() {
            if !rendered
                .measures
                .iter()
                .any(|item| item.voice == voice && item.measure == measure)
            {
                continue;
            }
            let global = index as u32 + 1;
            first.get_or_insert(global);
            if let Some(hint) = hint {
                if closest.is_none_or(|best: u32| global.abs_diff(hint) < best.abs_diff(hint)) {
                    closest = Some(global);
                }
            }
        }
        closest.or(first)
    }
}

fn literal_staff_property(
    syntax: &Syntax,
    with_range: Option<std::ops::Range<usize>>,
    body: std::ops::Range<usize>,
    property: &str,
) -> Option<String> {
    let tokens = &syntax.tokens;
    let mut names = Vec::new();
    for range in with_range.into_iter().chain(std::iter::once(body)) {
        let depth = syntax.depth[range.start] + 1;
        for position in range.start + 1..range.end.saturating_sub(2) {
            if syntax.depth[position] == depth
                && tokens[position].raw.rsplit('.').next() == Some(property)
                && tokens[position + 1].raw == "="
                && tokens[position + 2].kind == TokenKind::String
            {
                names.push(tokens[position + 2].raw.trim_matches('"').to_owned());
            }
        }
    }
    names.sort();
    names.dedup();
    (names.len() == 1).then(|| names.remove(0))
}

/// Index a supported subset without interpreting LilyPond or rejecting legal, opaque music.
pub fn build_index(source: &str) -> ScoreIndex {
    let syntax = Syntax::new(source);
    let t = &syntax.tokens;
    let mut assignments = HashMap::<String, Assignment>::new();
    let mut ordered = Vec::new();
    let mut duplicate = false;
    for i in 0..t.len().saturating_sub(2) {
        if syntax.depth[i] == 0 && t[i].kind == TokenKind::Ident && t[i + 1].raw == "=" {
            if let Some((open, end)) = syntax.expression(i + 2, t.len()) {
                let a = Assignment {
                    name: t[i].raw.clone(),
                    start: i,
                    rhs: i + 2,
                    open,
                    end,
                };
                ordered.push(a.clone());
                duplicate |= assignments.insert(a.name.clone(), a).is_some();
            }
        }
    }
    let decorations = decoration_assignments(&assignments, t);
    let mut voices = Vec::new();
    for a in &ordered {
        if t[a.open].raw != "{" {
            continue;
        }
        let body = &t[a.open + 1..a.end - 1];
        if body
            .iter()
            .any(|v| matches!(v.kind, TokenKind::Note | TokenKind::Rest | TokenKind::Skip))
            || t[a.rhs..a.end].iter().any(unsupported_token)
        {
            voices.push(region(
                source,
                a.name.clone(),
                a.start..a.end,
                a.rhs,
                a.open,
                &syntax,
                &decorations,
            ));
        }
    }
    let mut staves: Vec<StaffVoices> = Vec::new();
    let mut by_label = HashMap::<String, usize>::new();
    let mut parallel_staves = HashSet::new();
    let mut reason = syntax
        .malformed
        .then(|| "unbalanced source structure".to_string());
    if duplicate {
        reason = Some("duplicate music assignments require evaluation".into());
    }
    #[derive(Clone)]
    struct Task {
        from: usize,
        end: usize,
        staff: Option<usize>,
        piano_group: Option<usize>,
        path: Vec<String>,
    }
    let mut score_blocks = Vec::new();
    for (i, token) in t.iter().enumerate() {
        if syntax.depth[i] == 0 && token.command() == Some("score") {
            if let Some((open, end)) = syntax.expression(i + 1, t.len()) {
                let mut depth = 0usize;
                let mut layout = false;
                let mut midi = false;
                for item in &t[open..end] {
                    match item.raw.as_str() {
                        "{" => depth += 1,
                        "}" => depth = depth.saturating_sub(1),
                        _ if depth == 1 => {
                            layout |= item.command() == Some("layout");
                            midi |= item.command() == Some("midi");
                        }
                        _ => {}
                    }
                }
                score_blocks.push((open, end, layout, midi));
            }
        }
    }
    let engraving: Vec<_> = score_blocks
        .iter()
        .copied()
        .filter(|(_, _, layout, _)| *layout)
        .collect();
    let selected = if engraving.is_empty() {
        score_blocks
    } else {
        engraving
    };
    let mut tasks = selected
        .iter()
        .map(|(open, end, _, _)| Task {
            from: *open,
            end: *end,
            staff: None,
            piano_group: None,
            path: Vec::new(),
        })
        .collect::<Vec<_>>();
    if tasks.len() > 1 {
        reason.get_or_insert("multiple score blocks have ambiguous engraving order".into());
    }
    if tasks.is_empty() {
        tasks.push(Task {
            from: 0,
            end: t.len(),
            staff: None,
            piano_group: None,
            path: Vec::new(),
        });
    }
    let mut budget = t.len().saturating_mul(64).min(1_000_000);
    'walk: while let Some(task) = tasks.pop() {
        let mut i = task.from;
        while i < task.end {
            if budget == 0 {
                reason.get_or_insert("music reference expansion limit exceeded".into());
                break 'walk;
            }
            budget -= 1;
            let cmd = t[i].command();
            if syntax.depth[i] == 0 && i + 1 < t.len() && t[i + 1].raw == "=" {
                if let Some(a) = assignments.get(&t[i].raw) {
                    i = a.end;
                    continue;
                }
            }
            if matches!(
                cmd,
                Some("layout" | "midi" | "paper" | "header" | "with" | "markup")
            ) {
                if let Some((_, end)) = syntax.expression(i + 1, task.end) {
                    i = end;
                    continue;
                }
            }
            if matches!(cmd, Some("new" | "context")) && i + 1 < task.end {
                let what = t[i + 1].raw.as_str();
                let mut body = i + 2;
                let label = if t.get(body).is_some_and(|v| v.raw == "=") {
                    body += 2;
                    t.get(body - 1).map(|v| v.raw.trim_matches('"').to_string())
                } else {
                    None
                };
                let mut with_range = None;
                if t.get(body).and_then(Token::command) == Some("with") {
                    if let Some((_, end)) = syntax.expression(body + 1, task.end) {
                        with_range = Some(body + 1..end);
                        body = end;
                    }
                }
                if let Some((open, end)) = syntax.expression(body, task.end) {
                    let staff = if matches!(what, "Staff" | "TabStaff") {
                        if cmd == Some("context") && label.is_none() && task.staff.is_some() {
                            task.staff
                        } else if let Some(found) = label
                            .as_ref()
                            .and_then(|l| by_label.get(l))
                            .filter(|_| cmd == Some("context"))
                        {
                            Some(*found)
                        } else {
                            let n = staves.len();
                            staves.push(StaffVoices {
                                staff_index: n,
                                kind: if what == "TabStaff" {
                                    StaffKind::Tablature
                                } else {
                                    StaffKind::Notation
                                },
                                label: label.clone(),
                                piano_group: task.piano_group,
                                instrument_name: literal_staff_property(
                                    &syntax,
                                    with_range.clone(),
                                    open..end,
                                    "instrumentName",
                                ),
                                midi_instrument_name: literal_staff_property(
                                    &syntax,
                                    with_range.clone(),
                                    open..end,
                                    "midiInstrument",
                                ),
                                voices: Vec::new(),
                                property_anchor: matches!(t[open].raw.as_str(), "{" | "<<").then(
                                    || {
                                        crate::SourceMap::new(source)
                                            .span(t[open].byte_end, t[open].byte_end)
                                            .unwrap()
                                    },
                                ),
                            });
                            if let Some(label) = label {
                                by_label.insert(label, n);
                            }
                            Some(n)
                        }
                    } else {
                        task.staff
                    };
                    if matches!(what, "Staff" | "TabStaff")
                        && t[open].raw == "{"
                        && !t[open + 1..end - 1].iter().any(|v| {
                            v.command().is_some_and(|c| {
                                assignments.contains_key(c) || matches!(c, "context" | "new")
                            })
                        })
                    {
                        let name = format!("@inline:{}", t[open].byte_start);
                        let v = region(
                            source,
                            name.clone(),
                            body..end,
                            body,
                            open,
                            &syntax,
                            &decorations,
                        );
                        if let Some(staff) = staff {
                            staves[staff].voices.push(name);
                        }
                        voices.push(v);
                    } else {
                        tasks.push(Task {
                            from: end,
                            ..task.clone()
                        });
                        tasks.push(Task {
                            from: body,
                            end,
                            staff,
                            piano_group: if what == "PianoStaff" {
                                Some(i)
                            } else {
                                task.piano_group
                            },
                            path: task.path.clone(),
                        });
                        break;
                    }
                    i = end;
                    continue;
                }
            }
            if let Some(a) = cmd.and_then(|c| assignments.get(c)) {
                if let Some(staff) = task
                    .staff
                    .filter(|_| voices.iter().any(|v| v.name == a.name))
                {
                    staves[staff].voices.push(a.name.clone());
                } else if task.path.contains(&a.name) || task.path.len() >= 128 {
                    reason.get_or_insert("cyclic or deeply nested music references".into());
                } else {
                    let mut path = task.path.clone();
                    path.push(a.name.clone());
                    tasks.push(Task {
                        from: i + 1,
                        ..task.clone()
                    });
                    tasks.push(Task {
                        from: a.rhs,
                        end: a.end,
                        staff: task.staff,
                        piano_group: task.piano_group,
                        path,
                    });
                    break;
                }
                i += 1;
                continue;
            }
            if (t[i].raw == "{" || t[i].raw == "<<") && syntax.close[i].is_some() {
                if t[i].raw == "<<" {
                    if let Some(staff) = task.staff {
                        parallel_staves.insert(staff);
                    }
                }
                let end = syntax.close[i].unwrap() + 1;
                tasks.push(Task {
                    from: end,
                    ..task.clone()
                });
                tasks.push(Task {
                    from: i + 1,
                    end: end - 1,
                    staff: task.staff,
                    piano_group: task.piano_group,
                    path: task.path.clone(),
                });
                break;
            }
            i += 1;
        }
    }
    for v in &mut voices {
        if let Some(a) = assignments.get(&v.name) {
            if t[a.open + 1..a.end - 1].iter().any(|token| {
                token
                    .command()
                    .is_some_and(|c| assignments.contains_key(c) && !decorations.contains(c))
            }) {
                v.capability = LocationCapability::AnchorOnly;
                v.reason = Some("composed music variable requires expansion".into());
                v.measures.clear();
                v.measure_groups.clear();
            }
        }
    }
    if reason.is_some() {
        for v in &mut voices {
            v.measures.clear();
            v.measure_groups.clear();
            v.capability = LocationCapability::AnchorOnly;
            v.reason = reason.clone();
        }
    }
    let max_measures = voices
        .iter()
        .flat_map(VoiceRegion::indexed_measures)
        .map(|m| m.measure_end)
        .max()
        .unwrap_or(0);
    let capability = if voices
        .iter()
        .any(|v| v.capability == LocationCapability::Measure)
    {
        LocationCapability::Measure
    } else if !source.is_empty() {
        LocationCapability::AnchorOnly
    } else {
        LocationCapability::None
    };
    if capability != LocationCapability::Measure && reason.is_none() {
        reason = Some("no reliable measure index".into());
    }
    let rendered_measures = if capability == LocationCapability::Measure
        && reason.is_none()
        && parallel_staves.is_empty()
    {
        rendered_measure_sequence(&voices, &staves)
    } else {
        Vec::new()
    };
    ScoreIndex {
        source_hash: ContentHash::of(source.as_bytes()),
        voices,
        staves,
        max_measures,
        capability,
        reason,
        source: source.into(),
        rendered_measures,
    }
}

/// Concatenate one single-staff score's voices in engraving order.
///
/// Parallel voices and multi-staff layouts keep the existing local numbering;
/// their counts do not line up with a single rendered sequence.
fn rendered_measure_sequence(
    voices: &[VoiceRegion],
    staves: &[StaffVoices],
) -> Vec<RenderedMeasure> {
    let [staff] = staves else {
        return Vec::new();
    };
    if staff.voices.is_empty() {
        return Vec::new();
    }
    let mut sequence = Vec::new();
    for name in &staff.voices {
        let Some(voice) = voices.iter().find(|voice| &voice.name == name) else {
            return Vec::new();
        };
        if voice.capability != LocationCapability::Measure {
            return Vec::new();
        }
        if voice.measure_groups.len() > 1 {
            return Vec::new();
        }
        let mut found = false;
        for measure in voice.indexed_measures() {
            if measure.measure_start > measure.measure_end {
                return Vec::new();
            }
            for local in measure.measure_start..=measure.measure_end {
                sequence.push(RenderedMeasure {
                    measures: vec![RenderedMeasureRef {
                        voice: name.clone(),
                        measure: local,
                    }],
                });
            }
            found = true;
        }
        if !found {
            return Vec::new();
        }
    }
    sequence
}

fn decoration_assignments(
    assignments: &HashMap<String, Assignment>,
    tokens: &[Token],
) -> HashSet<String> {
    fn is_decoration(
        name: &str,
        assignments: &HashMap<String, Assignment>,
        tokens: &[Token],
        visiting: &mut HashSet<String>,
    ) -> bool {
        if !visiting.insert(name.to_owned()) {
            return false;
        }
        let Some(assignment) = assignments.get(name) else {
            visiting.remove(name);
            return false;
        };
        let body = &tokens[assignment.rhs..assignment.end];
        let affects_music = body.iter().any(|token| {
            matches!(
                token.kind,
                TokenKind::Note
                    | TokenKind::Rest
                    | TokenKind::Skip
                    | TokenKind::ChordO
                    | TokenKind::Bar
            ) || matches!(
                token.command(),
                Some("bar" | "partial" | "relative" | "transpose" | "repeat" | "times" | "tuplet")
            )
        });
        let safe = !affects_music
            && body.iter().all(|token| {
                token.command().is_none_or(|command| {
                    !assignments.contains_key(command)
                        || is_decoration(command, assignments, tokens, visiting)
                })
            });
        visiting.remove(name);
        safe
    }

    assignments
        .keys()
        .filter(|name| is_decoration(name, assignments, tokens, &mut HashSet::new()))
        .cloned()
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/index.rs"]
mod tests;
