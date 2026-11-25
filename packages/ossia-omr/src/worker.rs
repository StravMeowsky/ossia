use super::{process, queue::Queue, xml, CancelSignal};
use ossia_types::{AppError, Diagnostic, ErrorCode, JobId, JobState};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

pub(crate) fn spawn(
    queue: Queue,
    process: process::ProcessConfig,
    cancels: Arc<Mutex<HashMap<JobId, CancelSignal>>>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while !stop.load(Ordering::Acquire) {
            let record = match queue.claim_next() {
                Ok(Some(record)) => record,
                Ok(None) => {
                    thread::sleep(Duration::from_millis(200));
                    continue;
                }
                Err(error) => {
                    println!("ossia-omr queue error: {}", error.message);
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            };
            let cancel = Arc::new(AtomicBool::new(false));
            cancels
                .lock()
                .expect("OMR cancellation map poisoned")
                .insert(record.job_id, cancel.clone());
            if queue
                .get(record.job_id)
                .is_ok_and(|job| job.state != JobState::Running || job.phase == "cancelling")
            {
                cancel.store(true, Ordering::Release);
            }
            let result = process::run(
                &queue.job_dir(record.job_id),
                &queue.input_path(&record),
                &process,
                &cancel,
            );
            if stop.load(Ordering::Acquire) {
                break;
            }
            let _ = queue.set_phase(record.job_id, "validating");
            match result {
                Ok(diagnostics) => match xml::collect(&queue.output_dir(record.job_id)) {
                    Ok(output) => {
                        if let Err(error) = queue.finish(
                            record.job_id,
                            JobState::Done,
                            "finished",
                            Some(output.output_hash),
                            &diagnostics,
                            None,
                        ) {
                            println!("ossia-omr queue error: {}", error.message);
                        }
                    }
                    Err(error) => {
                        if let Err(error) = queue.finish(
                            record.job_id,
                            JobState::Error,
                            "finished",
                            None,
                            &diagnostics,
                            Some(&error),
                        ) {
                            println!("ossia-omr queue error: {}", error.message);
                        }
                    }
                },
                Err(error) if error.code == ErrorCode::Cancelled => {
                    let _ = queue.finish(
                        record.job_id,
                        JobState::Cancelled,
                        "cancelled",
                        None,
                        &[],
                        None,
                    );
                }
                Err(error) => {
                    let diagnostics = error_diagnostics(&error);
                    if let Err(error) = queue.finish(
                        record.job_id,
                        JobState::Error,
                        "finished",
                        None,
                        &diagnostics,
                        Some(&error),
                    ) {
                        println!("ossia-omr queue error: {}", error.message);
                    }
                }
            }
            cancels
                .lock()
                .expect("OMR cancellation map poisoned")
                .remove(&record.job_id);
            if let Err(error) = queue.prune_terminal() {
                println!("ossia-omr cleanup error: {}", error.message);
            }
        }
    })
}

fn error_diagnostics(error: &AppError) -> Vec<Diagnostic> {
    error
        .detail
        .as_ref()
        .and_then(|detail| detail.get("diagnostics"))
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::{process::ProcessConfig, queue::Queue};
    use ossia_types::{ContentHash, OmrSourceFormat};
    use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Instant};

    #[test]
    fn mock_audiveris_runs_through_the_worker() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("Audiveris");
        fs::write(
            &executable,
            "#!/bin/sh\n\
             out=\n\
             while [ \"$#\" -gt 0 ]; do\n\
               if [ \"$1\" = \"-output\" ]; then out=\"$2\"; shift 2; else shift; fi\n\
             done\n\
             mkdir -p \"$out\"\n\
             printf '<?xml version=\"1.0\"?><score-partwise/>' > \"$out/result.xml\"\n\
             echo 'WARN mock warning' >&2\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        let queue = Queue::open(root.path()).unwrap();
        let job = queue
            .create(OmrSourceFormat::Pdf, b"%PDF-1", &["eng".into()], "5.11.0")
            .unwrap();
        let cancels = Arc::new(Mutex::new(HashMap::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = spawn(
            queue.clone(),
            ProcessConfig {
                audiveris_bin: executable,
                tessdata_dir: PathBuf::from("/tmp"),
                languages: vec!["eng".into()],
                max_job_seconds: 10,
                log_limit_bytes: 1024 * 1024,
            },
            cancels,
            stop.clone(),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let current = queue.get(job.job_id).unwrap();
            if current.state == JobState::Done {
                assert_eq!(
                    current.output_hash,
                    Some(ContentHash::of(b"<?xml version=\"1.0\"?><score-partwise/>"))
                );
                assert_eq!(current.diagnostics.len(), 1);
                break;
            }
            assert!(Instant::now() < deadline, "worker did not finish");
            thread::sleep(Duration::from_millis(25));
        }
        stop.store(true, Ordering::Release);
        worker.join().unwrap();
    }
}
