use crate::*;

#[test]
fn active_jobs_do_not_decode_terminal_job_bodies() {
    let root = tempfile::tempdir().unwrap();
    let library = Library::open(root.path()).unwrap();
    let conn = library.db().unwrap();
    for state in ["done", "error", "cancelled", "interrupted"] {
        conn.execute(
            "INSERT INTO jobs(job_id,kind,state,body,created_at,updated_at) VALUES (?1,'test',?2,'not JSON',0,0)",
            params![JobId::new().to_string(), state],
        ).unwrap();
    }
    let mut expected = Vec::new();
    for state in [JobState::Queued, JobState::Running] {
        let job = Job {
            job_id: JobId::new(),
            document_id: None,
            kind: "test".into(),
            state,
            source_hash: None,
            phase: "test".into(),
            result: None,
            error: None,
        };
        db::save_job(&conn, &job).unwrap();
        expected.push(job.job_id);
    }
    drop(conn);
    assert_eq!(
        library
            .list_active_jobs()
            .unwrap()
            .iter()
            .map(|job| job.job_id)
            .collect::<Vec<_>>(),
        expected
    );
}
