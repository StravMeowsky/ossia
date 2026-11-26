use super::{matching_brace, Lexeme};
use ossia_types::RenderFormat;
use std::ops::Range;

// Only split a literal pair that declares distinct output formats explicitly.
pub(super) fn omitted_score(tokens: &[Lexeme], format: RenderFormat) -> Option<Range<usize>> {
    if tokens.iter().any(|token| {
        matches!(
            token.raw.as_str(),
            "\\book" | "\\bookpart" | "\\bookOutputName" | "\\bookOutputSuffix"
        )
    }) {
        return None;
    }
    let scores: Vec<_> = literal_scores(tokens)?
        .into_iter()
        .map(|score| (score.range, score.layout, score.midi))
        .collect();
    if scores.len() != 2 {
        return None;
    }
    let layout = scores
        .iter()
        .position(|(_, layout, midi)| *layout && !*midi);
    let midi = scores
        .iter()
        .position(|(_, layout, midi)| *midi && !*layout);
    let (Some(layout), Some(midi)) = (layout, midi) else {
        return None;
    };
    let omitted = match format {
        RenderFormat::Midi => layout,
        RenderFormat::Svg | RenderFormat::Pdf => midi,
    };
    Some(scores[omitted].0.clone())
}

/// One top-level literal `\score { ... }` block.
pub(super) struct ScoreBlock {
    /// Byte range from `\score` through its closing brace.
    pub range: Range<usize>,
    /// Byte offset of the closing brace.
    pub close: usize,
    pub layout: bool,
    pub midi: bool,
}

/// Top-level literal score blocks in file order; `None` when any is nested or not literal.
pub(super) fn literal_scores(tokens: &[Lexeme]) -> Option<Vec<ScoreBlock>> {
    let mut depth = 0usize;
    let mut scores = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.raw == "\\score" {
            if depth != 0 {
                return None;
            }
            let close = matching_brace(tokens, index + 1)?;
            let mut inner_depth = 0usize;
            let (mut layout, mut midi) = (false, false);
            for (offset, item) in tokens[index + 1..=close].iter().enumerate() {
                if inner_depth == 1 && matches!(item.raw.as_str(), "\\layout" | "\\midi") {
                    if tokens.get(index + 2 + offset).map(|next| next.raw.as_str()) != Some("{") {
                        return None;
                    }
                    layout |= item.raw == "\\layout";
                    midi |= item.raw == "\\midi";
                }
                match item.raw.as_str() {
                    "{" => inner_depth += 1,
                    "}" => inner_depth -= 1,
                    _ => {}
                }
            }
            scores.push(ScoreBlock {
                range: token.range.start..tokens[close].range.end,
                close: tokens[close].range.start,
                layout,
                midi,
            });
        }
        match token.raw.as_str() {
            "{" => depth += 1,
            "}" => depth = depth.checked_sub(1)?,
            _ => {}
        }
    }
    (depth == 0).then_some(scores)
}
