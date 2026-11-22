use super::*;
use crate::db;
use rusqlite::Connection;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::Path,
};

pub(super) fn render_rows(conn: &Connection) -> Result<Vec<RenderRow>> {
    let mut statement = conn
        .prepare("SELECT rowid,document_id,body,created_at,last_used_at FROM renders ORDER BY last_used_at DESC,rowid DESC")
        .map_err(storage_error)?;
    let values: Vec<(i64, String, String, i64, i64)> = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(storage_error)?
        .collect::<std::result::Result<_, _>>()
        .map_err(storage_error)?;
    drop(statement);
    values
        .into_iter()
        .map(|(rowid, document_id, body, created_at, last_used_at)| {
            Ok(RenderRow {
                rowid,
                document_id,
                body: db::decode(body)?,
                created_at,
                last_used_at,
            })
        })
        .collect()
}

pub(super) fn protected_render_rows(
    conn: &Connection,
    rows: &[RenderRow],
    current: &HashMap<String, String>,
    policy: StoragePolicy,
) -> Result<BTreeSet<i64>> {
    let mut protected = receipt_renders(conn)?;
    let mut last_renders = HashSet::new();
    let mut statement = conn
        .prepare("SELECT document_id,last_render FROM documents WHERE last_render IS NOT NULL")
        .map_err(storage_error)?;
    let documents = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(storage_error)?;
    for document in documents {
        let (document, body) = document.map_err(storage_error)?;
        let manifest: RenderManifest = db::decode(body)?;
        last_renders.insert((
            document,
            manifest.source_hash.to_string(),
            manifest.engine_build_id,
            manifest.options_hash.to_string(),
        ));
    }
    drop(statement);
    let mut seen: HashMap<&str, usize> = HashMap::new();
    let mut current_seen: HashSet<&str> = HashSet::new();
    for row in rows {
        if last_renders.contains(&(
            row.document_id.clone(),
            row.body.source_hash.to_string(),
            row.body.engine_build_id.clone(),
            row.body.options_hash.to_string(),
        )) {
            protected.insert(row.rowid);
        }
        let rank = seen.entry(&row.document_id).or_default();
        if *rank < policy.latest_per_document {
            protected.insert(row.rowid);
        }
        *rank += 1;
        if current
            .get(&row.document_id)
            .is_some_and(|hash| hash == row.body.source_hash.as_str())
            && current_seen.insert(&row.document_id)
        {
            protected.insert(row.rowid);
        }
    }
    Ok(protected)
}

pub(super) fn receipt_renders(conn: &Connection) -> Result<BTreeSet<i64>> {
    let mut statement = conn
        .prepare(
            "SELECT r.rowid FROM check_receipts c
             JOIN edits e ON e.edit_id=c.edit_id
             JOIN renders r ON r.document_id=e.document_id
               AND r.source_hash=c.source_hash
               AND r.engine_build_id=c.engine_build_id
               AND r.options_hash=c.options_hash",
        )
        .map_err(storage_error)?;
    let values = statement
        .query_map([], |row| row.get(0))
        .map_err(storage_error)?
        .collect::<std::result::Result<_, _>>()
        .map_err(storage_error)?;
    drop(statement);
    Ok(values)
}

pub(super) fn receipt_and_active_request_jobs(conn: &Connection) -> Result<HashSet<String>> {
    let mut result = HashSet::new();
    for query in [
        "SELECT job_id FROM check_receipts",
        "SELECT r.job_id FROM check_requests r
         JOIN edits e ON e.edit_id=r.edit_id
         WHERE e.status IN ('active','interrupted')",
    ] {
        let mut statement = conn.prepare(query).map_err(storage_error)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage_error)?;
        for row in rows {
            result.insert(row.map_err(storage_error)?);
        }
    }
    Ok(result)
}

pub(super) fn manifest_assets(manifest: &RenderManifest) -> Vec<String> {
    manifest
        .pages
        .iter()
        .map(|page| page.artifact.path.clone())
        .chain(manifest.pdf.iter().map(|asset| asset.path.clone()))
        .chain(manifest.midi.iter().map(|asset| asset.path.clone()))
        .collect()
}

pub(super) fn asset_sizes(root: &Path) -> Result<HashMap<String, u64>> {
    let mut result = HashMap::new();
    for entry in read_dir_optional(&root.join("assets"))? {
        if entry.file_type().map_err(storage_error)?.is_file() {
            result.insert(
                format!("assets/{}", entry.file_name().to_string_lossy()),
                entry.metadata().map_err(storage_error)?.len(),
            );
        }
    }
    Ok(result)
}
