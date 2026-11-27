//! Commands, contexts and the notation attached to events, for the preview walker.
use super::walk::{Ctx, GroupKind, Post, Walker};
use crate::note_edit::events::Frac;
use crate::note_edit::spelling::parse_name;
use crate::{Mode, TokenKind};

const MAX_DEPTH: usize = 24;

/// A duration argument such as `4.` or `8*3`.
fn duration_arg(text: &str) -> Option<Frac> {
    let (value, factor) = text.split_once('*').unwrap_or((text, "1"));
    let digits: String = value.chars().take_while(char::is_ascii_digit).collect();
    let dots = value[digits.len()..].chars().filter(|c| *c == '.').count() as u32;
    let base: u64 = digits.parse().ok().filter(|v: &u64| v.is_power_of_two())?;
    let (n, d) = factor.split_once('/').unwrap_or((factor, "1"));
    let length = Frac::new((1 << (dots + 1)) - 1, base << dots);
    Some(length.mul(Frac::new(n.parse().ok()?, d.parse().ok()?)))
}

impl Walker<'_> {
    pub(super) fn post(&self, mut j: usize) -> (Post, usize) {
        let mut post = Post::default();
        while let Some(t) = self.tokens.get(j) {
            match t.kind {
                TokenKind::Tie => post.tie = true,
                TokenKind::Beam => {}
                TokenKind::Fing | TokenKind::Artic => {
                    let artic = match t.raw.as_str() {
                        "-." | "." => Some("stacc"),
                        "->" | ">" | "^>" | "_>" => Some("acc"),
                        "--" => Some("ten"),
                        "-^" => Some("marc"),
                        "-!" | "!" => Some("stacciss"),
                        _ => None,
                    };
                    post.artics.extend(artic);
                    // `^"text"`, `-\markup` and the like carry no musical meaning here.
                    if matches!(t.raw.as_str(), "^" | "_" | "-") {
                        if let Some(next) = self.tokens.get(j + 1) {
                            if next.kind == TokenKind::String {
                                j += 1;
                            }
                        }
                    }
                }
                TokenKind::Raw if t.raw == "(" => post.slur_starts += 1,
                TokenKind::Raw if t.raw == ")" => post.slur_ends += 1,
                TokenKind::Cmd => {
                    let name = &t.raw[1..];
                    match name {
                        "(" => post.slur_starts += 1,
                        ")" => post.slur_ends += 1,
                        "<" => post.hairpins.push('<'),
                        ">" => post.hairpins.push('>'),
                        "!" => post.hairpins.push('!'),
                        "fermata" => post.fermata = true,
                        "staccato" => post.artics.push("stacc"),
                        "accent" => post.artics.push("acc"),
                        "tenuto" => post.artics.push("ten"),
                        "marcato" => post.artics.push("marc"),
                        "markup" => {
                            j += 1;
                            while self.tokens.get(j).is_some_and(|t| t.mode == Mode::Text) {
                                j += 1;
                            }
                            continue;
                        }
                        _ if is_dynamic(name) => post.dynamics.push(name.to_owned()),
                        _ if is_post_command(name) => {}
                        _ => break,
                    }
                }
                _ if t.mode == Mode::Text => {}
                _ => break,
            }
            j += 1;
        }
        (post, j)
    }

    fn next_pitch_token(&self, i: usize) -> usize {
        match self.tokens.get(i) {
            // Outside music, a reference pitch such as `\relative c''` is read as a word.
            Some(t) if matches!(t.kind, TokenKind::Note | TokenKind::Ident) => i + 1,
            _ => i,
        }
    }

    pub(super) fn command(&mut self, i: usize, command: &str, ctx: &mut Ctx) -> usize {
        let at = |k: usize| self.tokens.get(k);
        match command {
            "relative" | "fixed" => {
                let j = self.next_pitch_token(i + 1);
                self.music(j, ctx)
            }
            "absolute" => self.music(i + 1, ctx),
            "transpose" => {
                let j = self.next_pitch_token(self.next_pitch_token(i + 1));
                self.music(j, ctx)
            }
            "new" | "context" => self.context(i, command == "new", ctx),
            "repeat" => {
                let mut j = i + 1;
                while at(j).is_some_and(|t| matches!(t.kind, TokenKind::Ident | TokenKind::Num)) {
                    j += 1;
                }
                self.music(j, ctx)
            }
            "alternative" => self.music(i + 1, ctx),
            "tuplet" | "times" => {
                let ratio = at(i + 1)
                    .filter(|t| t.kind == TokenKind::Num)
                    .and_then(|t| {
                        let (n, d) = t.raw.split_once('/')?;
                        Some((n.trim().parse::<u64>().ok()?, d.trim().parse::<u64>().ok()?))
                    });
                let Some((n, d)) = ratio else {
                    return i + 1;
                };
                let mut j = i + 2;
                if command == "tuplet" && at(j).is_some_and(|t| t.kind == TokenKind::Num) {
                    j += 1;
                }
                // `\times 2/3` scales by 2/3; `\tuplet 3/2` by 2/3 as well.
                let (num, base) = if command == "times" { (d, n) } else { (n, d) };
                let mut inner = ctx.clone();
                inner.scale = ctx.scale.mul(Frac::new(base, num));
                inner.tuplet = Some((self.next_tuplet, num, base));
                self.next_tuplet += 1;
                let next = self.music(j, &mut inner);
                ctx.time = inner.time;
                ctx.staff = inner.staff;
                next
            }
            "grace" | "acciaccatura" | "appoggiatura" | "slashedGrace" => {
                let mut inner = ctx.clone();
                inner.grace = true;
                let next = self.music(i + 1, &mut inner);
                ctx.staff = inner.staff;
                next
            }
            "afterGrace" => {
                let mut j = i + 1;
                if at(j).is_some_and(|t| t.kind == TokenKind::Num) {
                    j += 1;
                }
                j = self.music(j, ctx);
                let mut inner = ctx.clone();
                inner.grace = true;
                self.music(j, &mut inner)
            }
            "partial" => {
                if let Some(length) = at(i + 1).and_then(|t| duration_arg(&t.raw)) {
                    self.out.partials.push((ctx.time, length.mul(ctx.scale)));
                }
                i + 2
            }
            "time" => {
                let ratio = at(i + 1).and_then(|t| {
                    let (n, d) = t.raw.split_once('/')?;
                    Some((n.trim().parse::<u64>().ok()?, d.trim().parse::<u64>().ok()?))
                });
                if let Some((n, d)) = ratio {
                    self.out.times.push((ctx.time, n, d));
                }
                i + 2
            }
            "key" => {
                let tonic = at(i + 1).and_then(|t| parse_name(&t.raw, self.language));
                let minor = at(i + 2).is_some_and(|t| t.raw == "\\minor");
                if let Some((step, alter)) = tonic {
                    let major = [0, 2, 4, -1, 1, 3, 5][step as usize] + 7 * alter;
                    self.out
                        .keys
                        .push((ctx.time, if minor { major - 3 } else { major }));
                }
                i + 3
            }
            "clef" => {
                if let Some(t) = at(i + 1) {
                    let name = t.raw.trim_matches('"').to_owned();
                    let staff = self.staff_for(ctx);
                    self.out.clefs.push((staff, ctx.time, name));
                }
                i + 2
            }
            "change" => {
                // `\change Staff = "name"`
                let name = at(i + 3).map(|t| t.raw.trim_matches('"').to_owned());
                if let Some(staff) = name.and_then(|n| self.named_staves.get(&n).copied()) {
                    ctx.staff = Some(staff);
                }
                i + 4
            }
            "markup" | "mark" | "tempo" => {
                let mut j = i + 1;
                while at(j).is_some_and(|t| t.mode == Mode::Text) {
                    j += 1;
                }
                j
            }
            "with" | "layout" | "midi" | "header" => self.skip_block_command(i),
            _ => match self.vars.get(command).copied() {
                Some(start) if self.depth < MAX_DEPTH => {
                    self.depth += 1;
                    self.music(start, ctx);
                    self.depth -= 1;
                    i + 1
                }
                _ => i + 1,
            },
        }
    }

    pub(super) fn context(&mut self, i: usize, new: bool, ctx: &mut Ctx) -> usize {
        let at = |k: usize| self.tokens.get(k);
        let Some(kind) = at(i + 1).map(|t| t.raw.clone()) else {
            return i + 1;
        };
        let mut j = i + 2;
        let mut name = None;
        if at(j).is_some_and(|t| t.raw == "=") {
            name = at(j + 1).map(|t| t.raw.trim_matches('"').to_owned());
            j += 2;
        }
        j = self.skip_block_command(j);
        let mut inner = ctx.clone();
        match kind.as_str() {
            "Staff" | "RhythmicStaff" | "TabStaff" | "DrumStaff" => {
                let staff = self.new_staff(name, ctx);
                inner.staff = Some(staff);
                inner.layer = 1;
            }
            "PianoStaff" | "GrandStaff" | "StaffGroup" | "ChoirStaff" => {
                self.out
                    .groups
                    .push(if kind == "StaffGroup" || kind == "ChoirStaff" {
                        GroupKind::Bracket
                    } else {
                        GroupKind::Brace
                    });
                inner.group = Some(self.out.groups.len() - 1);
            }
            "Voice" => {
                if let Some(staff) = ctx.staff {
                    let key = name.map(|n| (staff, n));
                    let known = key.as_ref().and_then(|k| self.named_layers.get(k)).copied();
                    inner.layer = match known {
                        Some(layer) => layer,
                        None if new || key.is_some() => {
                            let layer = self.fresh_layer();
                            if let Some(key) = key {
                                self.named_layers.insert(key, layer);
                            }
                            layer
                        }
                        None => ctx.layer,
                    };
                }
            }
            "Score" => {}
            _ => inner.mute = true,
        }
        let next = self.music(j, &mut inner);
        ctx.time = inner.time;
        if kind == "Voice" {
            ctx.staff = inner.staff.or(ctx.staff);
        }
        next
    }
}

fn is_dynamic(name: &str) -> bool {
    matches!(
        name,
        "pppp"
            | "ppp"
            | "pp"
            | "p"
            | "mp"
            | "mf"
            | "f"
            | "ff"
            | "fff"
            | "ffff"
            | "fp"
            | "sf"
            | "sff"
            | "sfz"
            | "sp"
            | "spp"
            | "rfz"
            | "fz"
            | "sfp"
    )
}

fn is_post_command(name: &str) -> bool {
    matches!(
        name,
        "trill"
            | "prall"
            | "mordent"
            | "turn"
            | "reverseturn"
            | "prallprall"
            | "staccatissimo"
            | "portato"
            | "espressivo"
            | "arpeggio"
            | "glissando"
            | "sustainOn"
            | "sustainOff"
            | "sostenutoOn"
            | "sostenutoOff"
            | "unaCorda"
            | "treCorde"
            | "upbow"
            | "downbow"
            | "flageolet"
            | "open"
            | "stopped"
            | "laissezVibrer"
            | "cresc"
            | "decresc"
            | "dim"
            | "harmonic"
            | "breathe"
            | "startTrillSpan"
            | "stopTrillSpan"
            | "tweak"
    )
}
