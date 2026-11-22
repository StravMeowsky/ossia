use super::*;

impl Runtime {
    pub fn selection(&self, selection: Selection) -> Result<DocumentContext> {
        self.selection_with_client(selection, None)
    }

    pub(crate) fn selection_with_client(
        &self,
        mut selection: Selection,
        client_id: Option<String>,
    ) -> Result<DocumentContext> {
        let document = self.library.get_document(selection.document_id)?;
        let snapshot = self.library.source_snapshot(selection.document_id)?;
        if snapshot.source_hash() != &selection.source_hash {
            return Err(stale());
        }
        validate_annotations(&mut selection.annotations)?;
        if selection.targets.is_empty() && selection.annotations.is_empty() {
            let mut state = self.state();
            state.selections.remove(&selection.document_id);
            self.emit_locked(
                &mut state,
                "selection-changed",
                Some(selection.document_id),
                Some(selection.source_hash.clone()),
                selection_payload(&document.title, 0, 0, client_id.as_deref(), None),
            );
            drop(state);
            return self.context(selection.document_id);
        }
        let index = self.score_index(&snapshot);
        let geometry = if selection
            .targets
            .iter()
            .chain(
                selection
                    .annotations
                    .iter()
                    .flat_map(|annotation| &annotation.targets),
            )
            .any(|target| matches!(target, SelectionTarget::Element { .. }))
        {
            let document = self.library.get_document(selection.document_id)?;
            let manifest = document
                .last_successful_render
                .filter(|r| r.source_hash == selection.source_hash)
                .ok_or_else(|| {
                    AppError::unsupported("element selection requires a current render")
                })?;
            self.score_geometry(&snapshot, &manifest)?
        } else {
            Arc::new(Vec::new())
        };
        let identity = ossia_score::music_identity_with_index(&index, snapshot.source());
        let (targets, regions) = build_regions(&index, &identity, &geometry, &selection.targets)?;
        let mut annotation_regions = Vec::new();
        for annotation in &selection.annotations {
            for region in
                annotation_regions_for_targets(&index, &identity, &geometry, &annotation.targets)?
            {
                if !annotation_regions
                    .iter()
                    .any(|existing: &SelectionRegion| existing.target == region.target)
                {
                    annotation_regions.push(region);
                }
            }
        }
        selection.targets = targets;
        {
            let target_count = regions.len();
            let annotation_count = selection.annotations.len();
            let mut state = self.state();
            state.selections.insert(
                selection.document_id,
                SelectionState {
                    selection: selection.clone(),
                    regions,
                    annotation_regions,
                },
            );
            state
                .observed
                .insert(selection.document_id, selection.source_hash.clone());
            self.emit_locked(
                &mut state,
                "selection-changed",
                Some(selection.document_id),
                Some(selection.source_hash.clone()),
                selection_payload(
                    &document.title,
                    target_count,
                    annotation_count,
                    client_id.as_deref(),
                    None,
                ),
            );
        }
        let context = self.context(selection.document_id)?;
        if context
            .selection
            .as_ref()
            .is_none_or(|selected| selected.source_hash != selection.source_hash)
        {
            return Err(stale());
        }
        Ok(context)
    }

    pub fn clear_selection(&self, document_id: DocumentId) -> Result<DocumentContext> {
        let document = self.library.get_document(document_id)?;
        let mut state = self.state();
        state.selections.remove(&document_id);
        self.emit_locked(
            &mut state,
            "selection-changed",
            Some(document_id),
            None,
            selection_payload(&document.title, 0, 0, None, None),
        );
        drop(state);
        self.context(document_id)
    }
}

fn validate_annotations(annotations: &mut Vec<SelectionAnnotation>) -> Result<()> {
    let mut normalized = Vec::with_capacity(annotations.len());
    let mut ids = Vec::new();
    for annotation in std::mem::take(annotations) {
        let id = annotation.id.trim();
        if id.is_empty() {
            return Err(invalid("annotation id must not be empty"));
        }
        if ids.iter().any(|existing| existing == id) {
            return Err(invalid("annotation ids must be unique"));
        }
        ids.push(id.to_owned());
        let text = annotation.text.trim();
        if text.is_empty() {
            return Err(invalid("annotation text must not be empty"));
        }
        let mut targets = Vec::new();
        for target in annotation.targets {
            if targets.iter().any(|existing| existing == &target) {
                continue;
            }
            targets.push(target);
        }
        if targets.is_empty() {
            return Err(invalid("annotation requires at least one target"));
        }
        let mut boxes = Vec::with_capacity(annotation.boxes.len());
        for box_region in annotation.boxes {
            let bounds = &box_region.bounds;
            if box_region.page == 0 {
                return Err(invalid("annotation box page must be positive"));
            }
            if !bounds.x0.is_finite()
                || !bounds.y0.is_finite()
                || !bounds.x1.is_finite()
                || !bounds.y1.is_finite()
                || bounds.x0 >= bounds.x1
                || bounds.y0 >= bounds.y1
            {
                return Err(invalid(
                    "annotation box bounds must be finite and non-empty",
                ));
            }
            boxes.push(box_region);
        }
        if !boxes.is_empty() && !boxes.iter().any(|box_region| box_region.pin) {
            boxes[0].pin = true;
        }
        normalized.push(SelectionAnnotation {
            id: id.to_owned(),
            text: text.to_owned(),
            targets,
            boxes,
        });
    }
    *annotations = normalized;
    Ok(())
}
