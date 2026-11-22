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
    let mut depth = 0usize;
    let mut scores = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.raw == "\\score" {
            if depth != 0 {
                return None;
            }
            let close = matching_brace(tokens, index + 1)?;
            let mut inner_depth = 0usize;
            let mut layout = false;
            let mut midi = false;
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
            scores.push((token.range.start..tokens[close].range.end, layout, midi));
        }
        match token.raw.as_str() {
            "{" => depth += 1,
            "}" => depth = depth.checked_sub(1)?,
            _ => {}
        }
    }
    if scores.len() != 2 || depth != 0 {
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
