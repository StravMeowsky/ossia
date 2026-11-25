use crate::*;
use ossia_score::{ScoreIndex, SvgPage};
use std::ops::Range;

impl Library {
    pub(crate) fn audio_render(
        &self,
        engine: &dyn EngineBackend,
        snapshot: &SourceSnapshot,
        checked: Option<&RenderManifest>,
        context: &RunContext,
    ) -> Result<RenderManifest> {
        if let Some(render) = checked {
            if !render.pages.is_empty()
                && render.midi.is_some()
                && render.engine_build_id == engine.build_id()
                && render.source_hash == *snapshot.source_hash()
                && render.options_hash == ossia_engine::render_options_hash(true)
            {
                return Ok(render.clone());
            }
        }
        let render = engine.compile(
            snapshot,
            &CompileOptions {
                formats: vec![RenderFormat::Svg, RenderFormat::Midi],
                point_and_click: true,
            },
            &self.artifact_root()?,
            context,
        )?;
        if render.source_hash != *snapshot.source_hash()
            || render.engine_build_id != engine.build_id()
        {
            return Err(storage_error("audio render does not match its snapshot"));
        }
        if render.pages.is_empty() {
            return Err(render
                .format_errors
                .iter()
                .find(|error| error.format == RenderFormat::Svg)
                .map(|error| error.error.clone())
                .unwrap_or_else(|| compile_error(&render)));
        }
        if render.midi.is_none() {
            return Err(render
                .format_errors
                .iter()
                .find(|error| error.format == RenderFormat::Midi)
                .map(|error| error.error.clone())
                .unwrap_or_else(|| compile_error(&render)));
        }
        Ok(render)
    }

    pub(crate) fn audio_playback(
        &self,
        snapshot: &SourceSnapshot,
        render: &RenderManifest,
        range: Option<MeasureRange>,
    ) -> Result<ScorePlayback> {
        let index: ScoreIndex = ossia_score::build_index(snapshot.source());
        let mut sources = Vec::with_capacity(render.pages.len());
        for page in &render.pages {
            sources.push((
                page.page,
                String::from_utf8(self.read_artifact(&page.artifact)?).map_err(storage_error)?,
            ));
        }
        let pages = sources
            .iter()
            .map(|(page, svg)| SvgPage { page: *page, svg })
            .collect::<Vec<_>>();
        let layout = ossia_score::build_layout(&index, &pages)?;
        let midi_asset = render
            .midi
            .as_ref()
            .ok_or_else(|| storage_error("audio render has no MIDI artifact"))?;
        let midi = ossia_score::parse_midi(&self.read_artifact(midi_asset)?)?;
        let playback = ossia_score::build_playback(
            &index,
            &layout,
            &midi,
            render_key(snapshot.document_id(), render),
        )?;
        let Some(range) = range else {
            return Ok(playback);
        };
        let bounds = ossia_score::playback_measure_bounds(&index, &layout, &midi, range)?;
        clip_playback(playback, bounds, range.end)
    }
}

fn clip_playback(
    playback: ScorePlayback,
    bounds: Range<f64>,
    end_measure: u32,
) -> Result<ScorePlayback> {
    let last_measure = playback
        .systems
        .iter()
        .map(|system| system.measure_end)
        .max()
        .unwrap_or(0);
    if last_measure == 0 || end_measure > last_measure {
        return Err(AppError::invalid(format!(
            "measure range exceeds the score's {last_measure} measures"
        )));
    }
    let notes = playback
        .notes
        .into_iter()
        .filter_map(|note| {
            let start = note.start_seconds.max(bounds.start);
            let end = (note.start_seconds + note.duration_seconds).min(bounds.end);
            (start < end).then_some(PlaybackNote {
                start_seconds: start - bounds.start,
                duration_seconds: end - start,
                ..note
            })
        })
        .collect();
    Ok(ScorePlayback {
        render_key: playback.render_key,
        source_hash: playback.source_hash,
        duration_seconds: bounds.end - bounds.start,
        parts: playback.parts,
        notes,
        groups: Vec::new(),
        systems: Vec::new(),
    })
}

#[cfg(test)]
#[path = "../tests/unit/audio_output.rs"]
mod tests;
