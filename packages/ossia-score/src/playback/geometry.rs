use crate::{MeasureColumn, ScoreLayout};
use ossia_types::{GeometryElement, GeometryPage};

#[derive(Clone, Copy)]
pub(super) struct MeasurePosition {
    pub measure: u32,
    pub within: f64,
    pub system_index: usize,
}

/// Start a measure at its first note instead of the preceding barline or clef space.
pub(super) fn first_note_x(layout: &ScoreLayout, column: &MeasureColumn) -> f64 {
    let Some(page) = layout.pages.iter().find(|page| page.page == column.page) else {
        return column.x0;
    };
    let Some(system) = page.systems.get(column.system_index) else {
        return column.x0;
    };
    page.objects
        .iter()
        .filter(|object| {
            !object.hit_only
                && object.staff_index.is_some()
                && matches!(object.kind.as_str(), "NoteHead" | "TabNoteHead")
        })
        .filter(|object| {
            let (x, y) = object.bbox.center();
            x >= column.x0
                && x <= column.x1
                && y >= system.top - page.staff_space * 2.
                && y <= system.bot + page.staff_space * 2.
        })
        .map(|object| object.bbox.center().0)
        .filter(|x| *x > column.x0 && *x < column.x1)
        .min_by(f64::total_cmp)
        .unwrap_or(column.x0)
}

/// Infer a rendered element's measure from its position in one measure column.
pub(super) fn measure_position(
    layout: &ScoreLayout,
    page: &GeometryPage,
    element: &GeometryElement,
) -> Option<MeasurePosition> {
    let layout_page = layout.pages.iter().find(|item| item.page == page.page)?;
    let x = (element.bounds.x0 + element.bounds.x1) / 2.;
    let y = (element.bounds.y0 + element.bounds.y1) / 2.;
    for column in layout.columns.iter().filter(|column| {
        column.page == page.page
            && x >= column.x0
            && x <= column.x1
            && layout_page
                .systems
                .get(column.system_index)
                .is_some_and(|system| {
                    y >= system.top - layout_page.staff_space * 3.
                        && y <= system.bot + layout_page.staff_space * 3.
                })
    }) {
        let (Some(start), Some(end)) = (column.measure_start, column.measure_end) else {
            continue;
        };
        let count = end.saturating_sub(start).saturating_add(1);
        if count == 0 {
            continue;
        }
        let start_x = first_note_x(layout, column);
        let progress = ((x - start_x) / (column.x1 - start_x)).clamp(0., 0.999999);
        let scaled = progress * f64::from(count);
        let offset = scaled.floor() as u32;
        return Some(MeasurePosition {
            measure: start.saturating_add(offset.min(count - 1)),
            within: scaled - f64::from(offset),
            system_index: column.system_index,
        });
    }
    None
}
