//! Walk the music of a score in time order: which staff and layer every written event lands on.
//!
//! This is a preview interpreter, not LilyPond: unknown commands are skipped, and anything it
//! cannot place is left out of the preview rather than failing.
use crate::note_edit::events::{Event, EventKind, Frac};
use crate::pitch::NoteLanguage;
use crate::{Mode, Token, TokenKind};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupKind {
    Brace,
    Bracket,
}

#[derive(Debug, Default)]
pub(crate) struct Staff {
    pub(crate) group: Option<usize>,
}

/// Notation attached after an event.
#[derive(Debug, Default, Clone)]
pub(crate) struct Post {
    pub(crate) tie: bool,
    pub(crate) slur_starts: u8,
    pub(crate) slur_ends: u8,
    pub(crate) dynamics: Vec<String>,
    /// `<`, `>` or `!` hairpin marks in order.
    pub(crate) hairpins: Vec<char>,
    pub(crate) artics: Vec<&'static str>,
    pub(crate) fermata: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Item {
    pub(crate) staff: usize,
    pub(crate) layer: u32,
    pub(crate) time: Frac,
    /// Sounding length after tuplets and multipliers.
    pub(crate) length: Frac,
    /// Index into the scanned events.
    pub(crate) event: usize,
    pub(crate) grace: bool,
    /// Tuplet group and its written ratio, `num` notes in the time of `numbase`.
    pub(crate) tuplet: Option<(u32, u64, u64)>,
    /// One measure of a full-measure rest.
    pub(crate) measure_rest: bool,
    pub(crate) post: Post,
}

#[derive(Debug, Default)]
pub(crate) struct Timeline {
    pub(crate) staves: Vec<Staff>,
    pub(crate) groups: Vec<GroupKind>,
    pub(crate) items: Vec<Item>,
    pub(crate) clefs: Vec<(usize, Frac, String)>,
    pub(crate) keys: Vec<(Frac, i32)>,
    pub(crate) times: Vec<(Frac, u64, u64)>,
    pub(crate) partials: Vec<(Frac, Frac)>,
}

#[derive(Debug, Clone)]
pub(super) struct Ctx {
    pub(super) staff: Option<usize>,
    pub(super) layer: u32,
    pub(super) time: Frac,
    pub(super) scale: Frac,
    pub(super) grace: bool,
    pub(super) tuplet: Option<(u32, u64, u64)>,
    pub(super) mute: bool,
    pub(super) group: Option<usize>,
}

pub(crate) struct Walker<'a> {
    pub(super) tokens: Vec<Token>,
    pub(super) vars: HashMap<String, usize>,
    pub(super) events: HashMap<usize, usize>,
    pub(super) scanned: &'a [Event],
    pub(super) language: NoteLanguage,
    pub(super) named_staves: HashMap<String, usize>,
    pub(super) named_layers: HashMap<(usize, String), u32>,
    pub(super) next_layer: u32,
    pub(super) implicit: Option<usize>,
    pub(super) next_tuplet: u32,
    pub(super) depth: usize,
    pub(super) out: Timeline,
}

fn is_open(t: &Token) -> bool {
    t.kind == TokenKind::Brace && t.raw == "{"
}

fn is_close(t: &Token) -> bool {
    t.kind == TokenKind::Brace && t.raw == "}"
}

impl<'a> Walker<'a> {
    pub(crate) fn new(source: &'a str, scanned: &'a [Event], language: NoteLanguage) -> Self {
        let tokens: Vec<Token> = crate::tokenize(source)
            .into_iter()
            .filter(|t| !t.trivia())
            .collect();
        let mut vars = HashMap::new();
        for i in 1..tokens.len().saturating_sub(1) {
            if tokens[i].mode == Mode::Top
                && tokens[i].raw == "="
                && tokens[i - 1].kind == TokenKind::Ident
            {
                vars.insert(tokens[i - 1].raw.clone(), i + 1);
            }
        }
        let events = scanned
            .iter()
            .enumerate()
            .map(|(k, e)| (e.start, k))
            .collect();
        Self {
            tokens,
            vars,
            events,
            scanned,
            language,
            named_staves: HashMap::new(),
            named_layers: HashMap::new(),
            next_layer: 1,
            implicit: None,
            next_tuplet: 0,
            depth: 0,
            out: Timeline::default(),
        }
    }

    /// Walk the first `\score`, or top-level music when there is none.
    pub(crate) fn run(mut self) -> Timeline {
        let start = Ctx {
            staff: None,
            layer: 1,
            time: Frac::new(0, 1),
            scale: Frac::new(1, 1),
            grace: false,
            tuplet: None,
            mute: false,
            group: None,
        };
        let score = (0..self.tokens.len())
            .find(|&i| self.tokens[i].mode == Mode::Top && self.tokens[i].raw == "\\score");
        match score {
            Some(i) => {
                let Some(open) = (i + 1..self.tokens.len()).find(|&j| is_open(&self.tokens[j]))
                else {
                    return self.out;
                };
                let mut j = open + 1;
                let mut ctx = start;
                while j < self.tokens.len() && !is_close(&self.tokens[j]) {
                    let skipped = self.skip_block_command(j);
                    j = if skipped > j {
                        skipped
                    } else {
                        let mut branch = ctx.clone();
                        branch.time = Frac::new(0, 1);
                        let next = self.music(j, &mut branch);
                        ctx.staff = None;
                        next
                    };
                }
            }
            None => {
                let mut j = 0;
                let mut ctx = start;
                while j < self.tokens.len() {
                    let t = &self.tokens[j];
                    let top_music = t.mode == Mode::Top
                        && (is_open(t) || matches!(t.command(), Some("relative" | "fixed")));
                    j = if top_music {
                        self.music(j, &mut ctx)
                    } else {
                        j + 1
                    };
                }
            }
        }
        self.out
    }

    /// Skip `\layout { }`, `\midi { }`, `\header { }` and `\with { }`; returns `i` otherwise.
    pub(super) fn skip_block_command(&self, i: usize) -> usize {
        let blocks = ["layout", "midi", "header", "with", "paper"];
        if !self.tokens[i]
            .command()
            .is_some_and(|c| blocks.contains(&c))
        {
            return i;
        }
        match self.tokens.get(i + 1) {
            Some(t) if is_open(t) => self.matching_close(i + 1) + 1,
            _ => i + 1,
        }
    }

    pub(super) fn matching_close(&self, open: usize) -> usize {
        let mut depth = 0i32;
        for j in open..self.tokens.len() {
            let t = &self.tokens[j];
            if t.kind == TokenKind::Brace {
                depth += if t.raw == "{" { 1 } else { -1 };
                if depth == 0 {
                    return j;
                }
            }
        }
        self.tokens.len() - 1
    }

    pub(super) fn staff_for(&mut self, ctx: &mut Ctx) -> usize {
        if let Some(staff) = ctx.staff {
            return staff;
        }
        let staff = *self.implicit.get_or_insert_with(|| {
            self.out.staves.push(Staff::default());
            self.out.staves.len() - 1
        });
        ctx.staff = Some(staff);
        staff
    }

    pub(super) fn new_staff(&mut self, name: Option<String>, ctx: &Ctx) -> usize {
        if let Some(staff) = name.as_ref().and_then(|n| self.named_staves.get(n)) {
            return *staff;
        }
        self.out.staves.push(Staff { group: ctx.group });
        let staff = self.out.staves.len() - 1;
        if let Some(name) = name {
            self.named_staves.insert(name, staff);
        }
        staff
    }

    /// Layer numbers are unique across the score, so branches never share one.
    pub(super) fn fresh_layer(&mut self) -> u32 {
        self.next_layer += 1;
        self.next_layer
    }

    /// Walk one music expression starting at token `i`; returns the index after it.
    pub(super) fn music(&mut self, i: usize, ctx: &mut Ctx) -> usize {
        let Some(t) = self.tokens.get(i).cloned() else {
            return i + 1;
        };
        if t.mode == Mode::Text {
            return i + 1;
        }
        if is_open(&t) {
            let mut j = i + 1;
            while j < self.tokens.len() && !is_close(&self.tokens[j]) {
                j = self.music(j, ctx);
            }
            return j + 1;
        }
        if t.kind == TokenKind::Seq && t.raw == "<<" {
            return self.simultaneous(i, ctx);
        }
        if matches!(
            t.kind,
            TokenKind::Note | TokenKind::Rest | TokenKind::Skip | TokenKind::ChordO
        ) {
            return self.event(i, ctx);
        }
        if let Some(command) = t.command() {
            return self.command(i, command, ctx);
        }
        i + 1
    }

    /// End of the `<< >>` opened at `i`, and whether `\\` separates its voices.
    pub(super) fn simultaneous_extent(&self, i: usize) -> (usize, bool) {
        let mut depth = 0i32;
        let mut separated = false;
        for j in i..self.tokens.len() {
            let t = &self.tokens[j];
            match t.raw.as_str() {
                "<<" | "{" => depth += 1,
                ">>" | "}" => {
                    depth -= 1;
                    if depth == 0 {
                        return (j, separated);
                    }
                }
                "\\\\" if depth == 1 => separated = true,
                _ => {}
            }
        }
        (self.tokens.len(), separated)
    }

    pub(super) fn simultaneous(&mut self, i: usize, ctx: &mut Ctx) -> usize {
        let (close, separated) = self.simultaneous_extent(i);
        let start = ctx.time;
        let mut end = start;
        let mut branch = ctx.clone();
        let mut j = i + 1;
        let mut first = true;
        while j < close {
            let divider = self.tokens[j].command() == Some("\\");
            if divider || (!separated && !first) {
                end = end.max(branch.time);
                branch = ctx.clone();
                branch.layer = self.fresh_layer();
            }
            first = false;
            if divider {
                j += 1;
                continue;
            }
            j = self.music(j, &mut branch);
        }
        end = end.max(branch.time);
        ctx.time = end;
        close + 1
    }

    pub(super) fn event(&mut self, i: usize, ctx: &mut Ctx) -> usize {
        let start = self.tokens[i].byte_start;
        let mut j = i + 1;
        if self.tokens[i].kind == TokenKind::ChordO {
            while j < self.tokens.len() && self.tokens[j].kind != TokenKind::ChordC {
                j += 1;
            }
            j += 1;
        }
        let Some(&index) = self.events.get(&start) else {
            return j;
        };
        let event = &self.scanned[index];
        // Skip a multiplier, then read the notation attached to the event.
        while j < self.tokens.len() && self.tokens[j].byte_start < event.full_end {
            j += 1;
        }
        let (post, next) = self.post(j);
        let one = event.value.mul(event.scale).mul(ctx.scale);
        let staff = self.staff_for(ctx);
        let grace = ctx.grace;
        for k in 0..event.count {
            if !ctx.mute && event.kind != EventKind::Skip {
                self.out.items.push(Item {
                    staff,
                    layer: ctx.layer,
                    time: ctx.time,
                    length: one,
                    event: index,
                    grace,
                    tuplet: ctx.tuplet,
                    measure_rest: event.kind == EventKind::MeasureRest,
                    post: if k == 0 {
                        post.clone()
                    } else {
                        Post::default()
                    },
                });
            }
            if !grace {
                ctx.time = ctx.time.add(one);
            }
        }
        next
    }
}
