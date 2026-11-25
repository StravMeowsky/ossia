use crate::Result;
use ossia_types::*;
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

const DATABASE: &str = "queue.sqlite3";
const MAX_FINISHED_AGE_MS: i64 = 7 * 24 * 60 * 60 * 1000;
const MAX_FINISHED_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct Queue {
    conn: Arc<Mutex<Connection>>,
    home: PathBuf,
}

#[derive(Clone)]
pub(crate) struct JobRecord {
    pub rowid: i64,
    pub job_id: JobId,
    pub state: JobState,
    pub phase: String,
    pub source_format: OmrSourceFormat,
    pub input_hash: ContentHash,
    pub output_hash: Option<ContentHash>,
    pub audiveris_version: Option<String>,
    pub languages: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub error: Option<AppError>,
    pub created_at: i64,
}

impl Queue {
    pub fn open(home: &Path) -> Result<Self> {
        fs::create_dir_all(home.join("jobs")).map_err(storage)?;
        let conn = Connection::open(home.join(DATABASE)).map_err(storage)?;
        conn.busy_timeout(std::time::Duration::from_secs(10))
            .map_err(storage)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS jobs (
               id TEXT PRIMARY KEY,
               state TEXT NOT NULL,
               phase TEXT NOT NULL,
               source_format TEXT NOT NULL,
               input_hash TEXT NOT NULL,
               output_hash TEXT,
               audiveris_version TEXT,
               languages TEXT NOT NULL,
               diagnostics TEXT NOT NULL,
               error TEXT,
               created_at INTEGER NOT NULL,
               started_at INTEGER,
               finished_at INTEGER
             );
             CREATE INDEX IF NOT EXISTS jobs_queue ON jobs(state, created_at);
             CREATE INDEX IF NOT EXISTS jobs_finished ON jobs(finished_at);",
        )
        .map_err(storage)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            home: home.to_path_buf(),
        })
    }

    pub fn recover_interrupted(&self, audiveris_version: &str) -> Result<()> {
        let now = now_millis();
        let error = serde_json::to_string(&AppError {
            code: ErrorCode::RuntimeUnavailable,
            message: "OMR service restarted while this job was running".into(),
            detail: Some(serde_json::json!({"reason":"service-restarted"})),
        })
        .map_err(storage)?;
        let conn = self.conn.lock().expect("OMR queue poisoned");
        conn.execute(
            "UPDATE jobs SET state=?1, phase='service-restarted', finished_at=?2, error=?3, audiveris_version=?4 WHERE state='running'",
            params![state_name(JobState::Interrupted), now, error, audiveris_version],
        )
        .map_err(storage)?;
        Ok(())
    }

    pub fn create(
        &self,
        source_format: OmrSourceFormat,
        bytes: &[u8],
        languages: &[String],
        audiveris_version: &str,
    ) -> Result<OmrJob> {
        let id = JobId::new();
        let directory = self.job_dir(id);
        fs::create_dir_all(&directory).map_err(storage)?;
        let input = directory.join(input_name(source_format));
        let temporary = directory.join(".input.tmp");
        {
            use std::io::Write;
            let mut file = fs::File::create(&temporary).map_err(storage)?;
            file.write_all(bytes).map_err(storage)?;
            file.sync_all().map_err(storage)?;
        }
        fs::rename(&temporary, &input).map_err(storage)?;
        let result = (|| {
            let conn = self.conn.lock().expect("OMR queue poisoned");
            conn.execute(
                "INSERT INTO jobs(id,state,phase,source_format,input_hash,languages,diagnostics,audiveris_version,created_at) VALUES (?1,?2,?3,?4,?5,?6,'[]',?7,?8)",
                params![
                    id.to_string(),
                    state_name(JobState::Queued),
                    "queued",
                    source_name(source_format),
                    ContentHash::of(bytes).to_string(),
                    serde_json::to_string(languages).map_err(storage)?,
                    audiveris_version,
                    now_millis()
                ],
            )
            .map_err(storage)?;
            drop(conn);
            self.get(id)
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&directory);
        }
        result
    }

    pub fn get(&self, id: JobId) -> Result<OmrJob> {
        let conn = self.conn.lock().expect("OMR queue poisoned");
        let record = stored(&conn, id)?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "OMR job not found"))?;
        let position = if record.state == JobState::Queued {
            Some(
                conn.query_row(
                    "SELECT count(*) FROM jobs WHERE state='queued' AND (created_at < ?1 OR (created_at=?1 AND rowid < ?2))",
                    params![record.created_at, record.rowid],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(storage)? as u32,
            )
        } else {
            None
        };
        record.public(position)
    }

    pub fn claim_next(&self) -> Result<Option<JobRecord>> {
        let mut conn = self.conn.lock().expect("OMR queue poisoned");
        let tx = conn.transaction().map_err(storage)?;
        let rowid = tx
            .query_row(
                "SELECT rowid FROM jobs WHERE state='queued' ORDER BY created_at, rowid LIMIT 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage)?;
        let Some(rowid) = rowid else {
            return Ok(None);
        };
        let changed = tx
            .execute(
                "UPDATE jobs SET state='running', phase='audiveris', started_at=?2 WHERE rowid=?1 AND state='queued'",
                params![rowid, now_millis()],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Ok(None);
        }
        let record =
            record_by_rowid(&tx, rowid)?.ok_or_else(|| storage("claimed OMR job disappeared"))?;
        tx.commit().map_err(storage)?;
        Ok(Some(record))
    }

    pub fn set_phase(&self, id: JobId, phase: &str) -> Result<()> {
        let conn = self.conn.lock().expect("OMR queue poisoned");
        conn.execute(
            "UPDATE jobs SET phase=?2 WHERE id=?1",
            params![id.to_string(), phase],
        )
        .map_err(storage)?;
        Ok(())
    }

    pub fn finish(
        &self,
        id: JobId,
        state: JobState,
        phase: &str,
        output_hash: Option<ContentHash>,
        diagnostics: &[Diagnostic],
        error: Option<&AppError>,
    ) -> Result<()> {
        let conn = self.conn.lock().expect("OMR queue poisoned");
        conn.execute(
            "UPDATE jobs SET state=?2, phase=?3, output_hash=?4, diagnostics=?5, error=?6, finished_at=?7 WHERE id=?1",
            params![
                id.to_string(),
                state_name(state),
                phase,
                output_hash.map(|hash| hash.to_string()),
                serde_json::to_string(diagnostics).map_err(storage)?,
                error
                    .map(serde_json::to_string)
                    .transpose()
                    .map_err(storage)?,
                now_millis()
            ],
        )
        .map_err(storage)?;
        Ok(())
    }

    pub fn cancel(&self, id: JobId) -> Result<(OmrJob, bool)> {
        let mut conn = self.conn.lock().expect("OMR queue poisoned");
        let tx = conn.transaction().map_err(storage)?;
        let current = stored(&tx, id)?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "OMR job not found"))?;
        let mut signal = false;
        if current.state == JobState::Queued {
            tx.execute(
                "UPDATE jobs SET state='cancelled', phase='cancelled', finished_at=?2 WHERE id=?1",
                params![id.to_string(), now_millis()],
            )
            .map_err(storage)?;
        } else if current.state == JobState::Running && current.phase != "cancelling" {
            tx.execute(
                "UPDATE jobs SET phase='cancelling' WHERE id=?1",
                [id.to_string()],
            )
            .map_err(storage)?;
            signal = true;
        }
        tx.commit().map_err(storage)?;
        drop(conn);
        Ok((self.get(id)?, signal))
    }

    pub fn input_path(&self, record: &JobRecord) -> PathBuf {
        self.job_dir(record.job_id)
            .join(input_name(record.source_format))
    }

    pub fn output_dir(&self, id: JobId) -> PathBuf {
        self.job_dir(id).join("out")
    }

    pub fn job_dir(&self, id: JobId) -> PathBuf {
        self.home.join("jobs").join(id.to_string())
    }

    pub fn remove_orphans(&self) -> Result<()> {
        let root = self.home.join("jobs");
        let known = {
            let conn = self.conn.lock().expect("OMR queue poisoned");
            let mut statement = conn.prepare("SELECT id FROM jobs").map_err(storage)?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(storage)?;
            rows.map(|row| {
                row.map_err(storage)?
                    .parse::<JobId>()
                    .map_err(|_| storage("invalid job id in OMR queue"))
            })
            .collect::<Result<HashSet<_>>>()?
        };
        for entry in fs::read_dir(root).map_err(storage)? {
            let entry = entry.map_err(storage)?;
            let name = entry.file_name();
            let Ok(id) = name.to_string_lossy().parse::<JobId>() else {
                remove_path(&entry.path());
                continue;
            };
            if !known.contains(&id) {
                remove_path(&entry.path());
            }
        }
        Ok(())
    }

    pub fn prune_terminal(&self) -> Result<()> {
        let now = now_millis();
        let mut terminal = {
            let conn = self.conn.lock().expect("OMR queue poisoned");
            let mut statement = conn
                .prepare(
                    "SELECT id, finished_at FROM jobs WHERE state IN ('done','error','cancelled','interrupted') ORDER BY finished_at, rowid",
                )
                .map_err(storage)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
                })
                .map_err(storage)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(storage)?
        };
        let mut retained = Vec::new();
        for (id, finished) in terminal.drain(..) {
            if finished.is_some_and(|finished| now.saturating_sub(finished) > MAX_FINISHED_AGE_MS) {
                self.remove_job(&id)?;
            } else {
                retained.push((id, finished.unwrap_or(now)));
            }
        }
        let mut total = 0u64;
        let mut sizes = Vec::new();
        for (id, finished) in retained {
            let size = id
                .parse::<JobId>()
                .map(|id| directory_size(&self.job_dir(id)))
                .unwrap_or_default();
            total = total.saturating_add(size);
            sizes.push((id, finished, size));
        }
        for (id, _, size) in sizes {
            if total <= MAX_FINISHED_BYTES {
                break;
            }
            self.remove_job(&id)?;
            total = total.saturating_sub(size);
        }
        Ok(())
    }

    fn remove_job(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().expect("OMR queue poisoned");
        conn.execute("DELETE FROM jobs WHERE id=?1", [id])
            .map_err(storage)?;
        drop(conn);
        if let Ok(id) = id.parse::<JobId>() {
            let _ = fs::remove_dir_all(self.job_dir(id));
        }
        Ok(())
    }
}

impl JobRecord {
    fn public(&self, queue_position: Option<u32>) -> Result<OmrJob> {
        Ok(OmrJob {
            job_id: self.job_id,
            state: self.state,
            phase: self.phase.clone(),
            queue_position,
            source_format: self.source_format,
            input_hash: self.input_hash.clone(),
            output_hash: self.output_hash.clone(),
            audiveris_version: self.audiveris_version.clone(),
            languages: self.languages.clone(),
            diagnostics: self.diagnostics.clone(),
            error: self.error.clone(),
        })
    }
}

fn stored(conn: &Connection, id: JobId) -> Result<Option<JobRecord>> {
    conn.query_row(
        "SELECT rowid,id,state,phase,source_format,input_hash,output_hash,audiveris_version,languages,diagnostics,error,created_at FROM jobs WHERE id=?1",
        [id.to_string()],
        job_record,
    )
    .optional()
    .map_err(storage)
}

fn record_by_rowid(conn: &Connection, rowid: i64) -> Result<Option<JobRecord>> {
    conn.query_row(
        "SELECT rowid,id,state,phase,source_format,input_hash,output_hash,audiveris_version,languages,diagnostics,error,created_at FROM jobs WHERE rowid=?1",
        [rowid],
        job_record,
    )
    .optional()
    .map_err(storage)
}

fn job_record(row: &Row<'_>) -> rusqlite::Result<JobRecord> {
    let id: String = row.get(1)?;
    let state: String = row.get(2)?;
    let source_format: String = row.get(4)?;
    let input_hash: String = row.get(5)?;
    let output_hash: Option<String> = row.get(6)?;
    let languages: String = row.get(8)?;
    let diagnostics: String = row.get(9)?;
    let error: Option<String> = row.get(10)?;
    Ok(JobRecord {
        rowid: row.get(0)?,
        job_id: id.parse().map_err(|_| conversion("invalid OMR job id"))?,
        state: parse_state(&state).map_err(|_| conversion("invalid OMR job state"))?,
        phase: row.get(3)?,
        source_format: parse_source(&source_format)
            .map_err(|_| conversion("invalid OMR source format"))?,
        input_hash: ContentHash::try_from(input_hash)
            .map_err(|_| conversion("invalid OMR input hash"))?,
        output_hash: output_hash
            .map(|hash| {
                ContentHash::try_from(hash).map_err(|_| conversion("invalid OMR output hash"))
            })
            .transpose()?,
        audiveris_version: row.get(7)?,
        languages: serde_json::from_str(&languages)
            .map_err(|_| conversion("invalid OMR languages"))?,
        diagnostics: serde_json::from_str(&diagnostics)
            .map_err(|_| conversion("invalid OMR diagnostics"))?,
        error: error
            .map(|error| serde_json::from_str(&error).map_err(|_| conversion("invalid OMR error")))
            .transpose()?,
        created_at: row.get(11)?,
    })
}

fn state_name(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Running => "running",
        JobState::Done => "done",
        JobState::Error => "error",
        JobState::Cancelled => "cancelled",
        JobState::Interrupted => "interrupted",
    }
}

fn parse_state(value: &str) -> Result<JobState> {
    match value {
        "queued" => Ok(JobState::Queued),
        "running" => Ok(JobState::Running),
        "done" => Ok(JobState::Done),
        "error" => Ok(JobState::Error),
        "cancelled" => Ok(JobState::Cancelled),
        "interrupted" => Ok(JobState::Interrupted),
        _ => Err(storage("invalid OMR job state")),
    }
}

fn source_name(format: OmrSourceFormat) -> &'static str {
    match format {
        OmrSourceFormat::Pdf => "pdf",
        OmrSourceFormat::Png => "png",
        OmrSourceFormat::Jpeg => "jpeg",
        OmrSourceFormat::Tiff => "tiff",
    }
}

fn parse_source(value: &str) -> Result<OmrSourceFormat> {
    match value {
        "pdf" => Ok(OmrSourceFormat::Pdf),
        "png" => Ok(OmrSourceFormat::Png),
        "jpeg" => Ok(OmrSourceFormat::Jpeg),
        "tiff" => Ok(OmrSourceFormat::Tiff),
        _ => Err(storage("invalid OMR source format")),
    }
}

fn input_name(format: OmrSourceFormat) -> &'static str {
    match format {
        OmrSourceFormat::Pdf => "input.pdf",
        OmrSourceFormat::Png => "input.png",
        OmrSourceFormat::Jpeg => "input.jpg",
        OmrSourceFormat::Tiff => "input.tiff",
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

fn directory_size(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.is_file() {
        return metadata.len();
    }
    if !metadata.is_dir() {
        return 0;
    }
    match fs::read_dir(path) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| directory_size(&entry.path()))
            .fold(0, u64::saturating_add),
        Err(_) => 0,
    }
}

fn remove_path(path: &Path) {
    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else {
        let _ = fs::remove_file(path);
    }
}

fn storage(error: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::StorageError, error.to_string())
}

fn conversion(message: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        )),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_preserves_fifo_order_and_positions() {
        let root = tempfile::tempdir().unwrap();
        let queue = Queue::open(root.path()).unwrap();
        let first = queue
            .create(OmrSourceFormat::Pdf, b"%PDF-1", &["eng".into()], "5.11.0")
            .unwrap();
        let second = queue
            .create(OmrSourceFormat::Png, b"png", &["eng".into()], "5.11.0")
            .unwrap();
        assert_eq!(first.queue_position, Some(0));
        assert_eq!(second.queue_position, Some(1));
        assert_eq!(queue.claim_next().unwrap().unwrap().job_id, first.job_id);
        assert_eq!(queue.get(second.job_id).unwrap().queue_position, Some(0));
    }

    #[test]
    fn restart_keeps_queued_and_interrupts_running() {
        let root = tempfile::tempdir().unwrap();
        let queue = Queue::open(root.path()).unwrap();
        let first = queue
            .create(OmrSourceFormat::Pdf, b"%PDF-1", &["eng".into()], "5.11.0")
            .unwrap();
        let second = queue
            .create(OmrSourceFormat::Pdf, b"%PDF-2", &["eng".into()], "5.11.0")
            .unwrap();
        assert_eq!(queue.claim_next().unwrap().unwrap().job_id, first.job_id);
        queue.recover_interrupted("5.11.0").unwrap();
        assert_eq!(
            queue.get(first.job_id).unwrap().state,
            JobState::Interrupted
        );
        assert_eq!(queue.get(second.job_id).unwrap().state, JobState::Queued);
    }

    #[test]
    fn cancellation_is_idempotent() {
        let root = tempfile::tempdir().unwrap();
        let queue = Queue::open(root.path()).unwrap();
        let job = queue
            .create(OmrSourceFormat::Pdf, b"%PDF-1", &["eng".into()], "5.11.0")
            .unwrap();
        assert_eq!(
            queue.cancel(job.job_id).unwrap().0.state,
            JobState::Cancelled
        );
        assert_eq!(
            queue.cancel(job.job_id).unwrap().0.state,
            JobState::Cancelled
        );
    }

    #[test]
    fn startup_orphan_cleanup_removes_unreferenced_job_directories() {
        let root = tempfile::tempdir().unwrap();
        let queue = Queue::open(root.path()).unwrap();
        let orphan = root.path().join("jobs/orphan");
        fs::create_dir_all(&orphan).unwrap();
        queue.remove_orphans().unwrap();
        assert!(!orphan.exists());
    }
}
