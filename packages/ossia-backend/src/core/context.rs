use super::*;
use serde_json::json;

mod selection;

impl Runtime {
    pub(crate) fn document_url(&self, document: DocumentId) -> String {
        format!("{}/scores?documentId={document}", self.info.web_base_url)
    }

    pub fn observe(&self, document: DocumentId) -> Result<DocumentState> {
        let doc = self.library.observe_source(document)?;
        let mut state = self.state();
        if state.observed.get(&document) == Some(&doc.source_hash) {
            return Ok(doc);
        }
        let had_selection = state.selections.remove(&document).is_some();
        state.observed.insert(document, doc.source_hash.clone());
        self.emit_locked(
            &mut state,
            "source-changed",
            Some(document),
            Some(doc.source_hash.clone()),
            json!({}),
        );
        if had_selection {
            self.emit_locked(
                &mut state,
                "selection-changed",
                Some(document),
                Some(doc.source_hash.clone()),
                selection_payload(&doc.title, 0, 0, None, Some("source-changed")),
            );
        }
        Ok(doc)
    }

    pub fn context(&self, document_id: DocumentId) -> Result<DocumentContext> {
        let document = self.observe(document_id)?;
        let (mut selection, mut regions, mut annotation_regions, mut seq) = {
            let mut state = self.state();
            if state
                .selections
                .get(&document_id)
                .is_some_and(|s| s.selection.source_hash != document.source_hash)
            {
                state.selections.remove(&document_id);
            }
            let selected = state.selections.get(&document_id);
            (
                selected.map(|s| s.selection.clone()),
                selected.map(|s| s.regions.clone()).unwrap_or_default(),
                selected
                    .map(|s| s.annotation_regions.clone())
                    .unwrap_or_default(),
                state.seq,
            )
        };
        if regions
            .iter()
            .chain(&annotation_regions)
            .any(|r| matches!(r.target, SelectionTarget::Element { .. }))
        {
            if let Some(manifest) = document
                .last_successful_render
                .as_ref()
                .filter(|r| r.source_hash == document.source_hash)
            {
                let snapshot = self.library.source_snapshot(document_id)?;
                let geometry = self.score_geometry(&snapshot, manifest)?;
                let prior_selection = selection.clone();
                let (targets, next_regions) = refresh_regions(&geometry, &regions);
                regions = next_regions;
                let (_, next_annotation_regions) = refresh_regions(&geometry, &annotation_regions);
                annotation_regions = next_annotation_regions;
                if let Some(current) = &mut selection {
                    current.targets = targets;
                }
                if selection
                    .as_ref()
                    .is_some_and(|s| s.targets.is_empty() && s.annotations.is_empty())
                {
                    selection = None;
                }
                let mut state = self.state();
                if state.selections.get(&document_id).map(|s| &s.selection)
                    == prior_selection.as_ref()
                {
                    if let Some(current) = &selection {
                        if let Some(stored) = state.selections.get_mut(&document_id) {
                            stored.selection = current.clone();
                            stored.regions = regions.clone();
                            stored.annotation_regions = annotation_regions.clone();
                        }
                    } else {
                        state.selections.remove(&document_id);
                    }
                    if selection != prior_selection {
                        self.emit_locked(
                            &mut state,
                            "selection-changed",
                            Some(document_id),
                            Some(document.source_hash.clone()),
                            selection_payload(
                                &document.title,
                                regions.len(),
                                selection
                                    .as_ref()
                                    .map_or(0, |selected| selected.annotations.len()),
                                None,
                                None,
                            ),
                        );
                        seq = state.seq;
                    }
                }
            }
        }
        self.ensure_interaction_render(&document)?;
        let render = document
            .last_successful_render
            .as_ref()
            .map(|manifest| RenderReference {
                render_key: ossia_library::render_key(document_id, manifest),
                manifest: manifest.clone(),
            });
        let edit_activity = self.library.edit_activity(document_id)?;
        Ok(DocumentContext {
            instance_id: self.info.instance_id.clone(),
            seq,
            document,
            selection,
            regions,
            annotation_regions,
            render,
            edit_activity,
        })
    }

    fn ensure_interaction_render(&self, document: &DocumentState) -> Result<()> {
        let outdated = document.last_successful_render.as_ref().is_some_and(|r| {
            r.source_hash == document.source_hash
                && r.options_hash != ossia_engine::render_options_hash(true)
        });
        if !outdated {
            return Ok(());
        }
        let (pending, running) = {
            let state = self.state();
            (
                state
                    .pending
                    .iter()
                    .any(|p| p.job.document_id == Some(document.document_id)),
                state.running.keys().copied().collect::<Vec<_>>(),
            )
        };
        let busy = pending
            || running.into_iter().any(|id| {
                self.library
                    .get_job(id)
                    .is_ok_and(|job| job.document_id == Some(document.document_id))
            });
        if !busy {
            self.check(
                CheckArgs {
                    document_id: Some(document.document_id),
                    edit_id: None,
                    expected_source_hash: document.source_hash.clone(),
                    expect: None,
                    reference: None,
                    wait_ms: None,
                },
                true,
            )?;
        }
        Ok(())
    }

    pub fn locate(&self, document: DocumentId, args: LocateArgs) -> Result<LocateOutcome> {
        let doc = self.observe(document)?;
        let snapshot = self.library.source_snapshot(document)?;
        if args.source_hash != doc.source_hash || snapshot.source_hash() != &args.source_hash {
            return Err(stale());
        }
        let manifest = doc
            .last_successful_render
            .filter(|render| render.source_hash == args.source_hash)
            .ok_or_else(|| AppError::unsupported("no current successful render"))?;
        let index = self.score_index(&snapshot);
        let geometry = self.score_geometry(&snapshot, &manifest)?;
        let mut region = ossia_score::hit_selection(&index, &geometry, args.page, args.x, args.y)?;
        let reason = if region
            .as_ref()
            .is_some_and(|r| r.spans.is_empty() && r.insertion_anchor.is_none())
        {
            region.take().and_then(|r| r.reason)
        } else {
            region.is_none().then(|| "no-hit".into())
        };
        Ok(LocateOutcome {
            capability: match region.as_ref().map(|r| &r.target) {
                Some(SelectionTarget::Measure { .. }) => LocationCapability::Measure,
                Some(SelectionTarget::Element { .. }) => LocationCapability::AnchorOnly,
                None => LocationCapability::None,
            },
            reason,
            region,
        })
    }

    pub fn events(
        &self,
        document: Option<DocumentId>,
        instance: &str,
        after: u64,
    ) -> Result<Vec<RuntimeEvent>> {
        let state = self.state();
        if instance != self.info.instance_id
            || after > state.seq
            || state
                .events
                .front()
                .is_some_and(|event| after < event.seq.saturating_sub(1))
        {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "event cursor expired; fetch a new snapshot",
            ));
        }
        Ok(state
            .events
            .iter()
            .filter(|event| event.seq > after)
            .filter(|event| {
                document.is_none() || event.document_id.is_none() || event.document_id == document
            })
            .cloned()
            .collect())
    }
}

fn selection_payload(
    document_title: &str,
    target_count: usize,
    annotation_count: usize,
    client_id: Option<&str>,
    reason: Option<&str>,
) -> Value {
    let mut payload = json!({
        "documentTitle": document_title,
        "targetCount": target_count,
        "annotationCount": annotation_count
    });
    if let Some(client_id) = client_id {
        payload["clientId"] = Value::String(client_id.to_owned());
    }
    if let Some(reason) = reason {
        payload["reason"] = Value::String(reason.to_owned());
    }
    payload
}

fn build_regions(
    index: &ossia_score::ScoreIndex,
    identity: &ossia_score::MusicIdentity,
    geometry: &[GeometryPage],
    requested: &[SelectionTarget],
) -> Result<(Vec<SelectionTarget>, Vec<SelectionRegion>)> {
    let targets = ossia_score::expand_selection(geometry, requested);
    let mut regions = Vec::new();
    for target in &targets {
        if regions
            .iter()
            .any(|region: &SelectionRegion| &region.target == target)
        {
            continue;
        }
        let region = match target {
            SelectionTarget::Measure { locator } => {
                if !identity.reliable {
                    return Err(AppError::unsupported(
                        "measure selection requires reliable musical identity",
                    ));
                }
                ossia_score::measure_selection(index, locator)?
            }
            SelectionTarget::Element { .. } => geometry
                .iter()
                .flat_map(|page| &page.elements)
                .find(|element| &element.region.target == target)
                .map(|element| element.region.clone())
                .ok_or_else(|| {
                    AppError::new(ErrorCode::NotFound, "element not found in current render")
                })?,
        };
        if region.spans.is_empty() && region.insertion_anchor.is_none() {
            return Err(AppError::unsupported(
                region
                    .reason
                    .as_deref()
                    .unwrap_or("element has no source mapping"),
            ));
        }
        regions.push(region);
    }
    Ok((targets, regions))
}

fn annotation_regions_for_targets(
    index: &ossia_score::ScoreIndex,
    identity: &ossia_score::MusicIdentity,
    geometry: &[GeometryPage],
    targets: &[SelectionTarget],
) -> Result<Vec<SelectionRegion>> {
    build_regions(index, identity, geometry, targets).map(|(_, regions)| regions)
}

fn refresh_regions(
    geometry: &[GeometryPage],
    prior: &[SelectionRegion],
) -> (Vec<SelectionTarget>, Vec<SelectionRegion>) {
    let valid: Vec<_> = prior
        .iter()
        .filter(|region| {
            matches!(region.target, SelectionTarget::Measure { .. })
                || geometry
                    .iter()
                    .flat_map(|page| &page.elements)
                    .any(|element| element.region.target == region.target)
        })
        .map(|region| region.target.clone())
        .collect();
    let targets = ossia_score::expand_selection(geometry, &valid);
    let regions = targets
        .iter()
        .filter_map(|target| {
            geometry
                .iter()
                .flat_map(|page| &page.elements)
                .find(|element| &element.region.target == target)
                .map(|element| element.region.clone())
                .or_else(|| {
                    prior
                        .iter()
                        .find(|region| &region.target == target)
                        .cloned()
                })
        })
        .collect();
    (targets, regions)
}
