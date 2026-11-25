use super::*;
use ossia_score::{ScoreIndex, ScoreLayout, SelectionGeometry};
use ossia_types::ScorePlayback;

const CACHE_ENTRIES: usize = 8;

#[derive(Default)]
pub(super) struct AnalysisCache {
    indexes: VecDeque<(ContentHash, Arc<ScoreIndex>)>,
    layouts: VecDeque<(ContentHash, Arc<ScoreLayout>)>,
    geometries: VecDeque<(ContentHash, Arc<SelectionGeometry>)>,
    playbacks: VecDeque<(ContentHash, Arc<ScorePlayback>)>,
}

impl Runtime {
    pub(crate) fn score_geometry(
        &self,
        snapshot: &SourceSnapshot,
        manifest: &RenderManifest,
    ) -> Result<Arc<SelectionGeometry>> {
        if snapshot.source_hash() != &manifest.source_hash {
            return Err(stale());
        }
        let key = ossia_library::render_key(snapshot.document_id(), manifest);
        {
            let mut cache = self.analysis.lock().expect("analysis cache poisoned");
            if let Some(position) = cache.geometries.iter().position(|(hash, _)| hash == &key) {
                let entry = cache.geometries.remove(position).expect("cached geometry");
                let pages = entry.1.clone();
                cache.geometries.push_back(entry);
                return Ok(pages);
            }
        }
        let index = self.score_index(snapshot);
        let layout = self.score_layout(snapshot, manifest)?;
        let pages = Arc::new(ossia_score::selection_geometry(&index, &layout)?);
        let mut cache = self.analysis.lock().expect("analysis cache poisoned");
        cache.geometries.push_back((key, pages.clone()));
        if cache.geometries.len() > CACHE_ENTRIES {
            cache.geometries.pop_front();
        }
        Ok(pages)
    }

    pub(crate) fn score_index(&self, snapshot: &SourceSnapshot) -> Arc<ScoreIndex> {
        let mut cache = self.analysis.lock().expect("analysis cache poisoned");
        let entry = cache
            .indexes
            .iter()
            .position(|(hash, _)| hash == snapshot.source_hash())
            .and_then(|position| cache.indexes.remove(position))
            .unwrap_or_else(|| {
                (
                    snapshot.source_hash().clone(),
                    Arc::new(ossia_score::build_index(snapshot.source())),
                )
            });
        let index = entry.1.clone();
        cache.indexes.push_back(entry);
        if cache.indexes.len() > CACHE_ENTRIES {
            cache.indexes.pop_front();
        }
        index
    }

    pub(crate) fn score_layout(
        &self,
        snapshot: &SourceSnapshot,
        manifest: &RenderManifest,
    ) -> Result<Arc<ScoreLayout>> {
        if snapshot.source_hash() != &manifest.source_hash {
            return Err(stale());
        }
        let key = ossia_library::render_key(snapshot.document_id(), manifest);
        let index = self.score_index(snapshot);
        let mut cache = self.analysis.lock().expect("analysis cache poisoned");
        if let Some(position) = cache.layouts.iter().position(|(hash, _)| hash == &key) {
            let entry = cache.layouts.remove(position).expect("cached layout");
            let layout = entry.1.clone();
            cache.layouts.push_back(entry);
            return Ok(layout);
        }
        // Render artifacts are immutable; parse once for each document/render identity.
        let sources = manifest
            .pages
            .iter()
            .map(|page| {
                Ok((
                    page.page,
                    String::from_utf8(self.library.read_artifact(&page.artifact)?)
                        .map_err(storage)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let pages = sources
            .iter()
            .map(|(page, svg)| ossia_score::SvgPage { page: *page, svg })
            .collect::<Vec<_>>();
        let layout = Arc::new(ossia_score::build_layout(&index, &pages)?);
        cache.layouts.push_back((key, layout.clone()));
        if cache.layouts.len() > CACHE_ENTRIES {
            cache.layouts.pop_front();
        }
        Ok(layout)
    }

    pub(crate) fn cached_playback(&self, key: &ContentHash) -> Result<Arc<ScorePlayback>> {
        {
            let mut cache = self.analysis.lock().expect("analysis cache poisoned");
            if let Some(position) = cache.playbacks.iter().position(|(hash, _)| hash == key) {
                let entry = cache.playbacks.remove(position).expect("cached playback");
                let playback = entry.1.clone();
                cache.playbacks.push_back(entry);
                return Ok(playback);
            }
        }
        let (_, manifest) = self.library.render(key)?;
        let snapshot = self.library.render_snapshot(key)?;
        let midi_asset = match manifest.midi.as_ref() {
            Some(asset) => asset.clone(),
            None => {
                let generated = self.engine.compile(
                    &snapshot,
                    &ossia_engine::CompileOptions {
                        formats: vec![RenderFormat::Midi],
                        point_and_click: true,
                    },
                    &self.library.artifact_root()?,
                    &ossia_engine::RunContext::default(),
                )?;
                generated.midi.ok_or_else(|| {
                    generated
                        .format_errors
                        .iter()
                        .find(|error| error.format == RenderFormat::Midi)
                        .map(|error| error.error.clone())
                        .unwrap_or_else(|| {
                            AppError::new(
                                ErrorCode::UnsupportedSource,
                                "playback MIDI could not be generated",
                            )
                        })
                })?
            }
        };
        let midi = ossia_score::parse_midi(&self.library.read_artifact(&midi_asset)?)?;
        let index = self.score_index(&snapshot);
        let layout = self.score_layout(&snapshot, &manifest)?;
        let playback = Arc::new(ossia_score::build_playback(
            &index,
            &layout,
            &midi,
            key.clone(),
        )?);
        let mut cache = self.analysis.lock().expect("analysis cache poisoned");
        cache.playbacks.push_back((key.clone(), playback.clone()));
        if cache.playbacks.len() > CACHE_ENTRIES {
            cache.playbacks.pop_front();
        }
        Ok(playback)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/core/analysis.rs"]
mod tests;
