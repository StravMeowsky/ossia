mod api;
mod config;
mod formats;
mod process;
mod queue;
mod worker;
mod xml;

pub use config::Config;
use ossia_types::{AppError, ErrorCode, JobId, OmrCapabilities, OmrJob};
use queue::Queue;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

pub(crate) type CancelSignal = Arc<AtomicBool>;
pub(crate) type Result<T> = std::result::Result<T, AppError>;

#[derive(Clone)]
pub(crate) struct Service {
    pub(crate) queue: Queue,
    pub(crate) token: Arc<str>,
    pub(crate) capabilities: OmrCapabilities,
    pub(crate) cancels: Arc<Mutex<HashMap<JobId, CancelSignal>>>,
    pub(crate) process: process::ProcessConfig,
}

impl Service {
    pub(crate) fn cancel(&self, id: JobId) -> Result<OmrJob> {
        let (job, signal) = self.queue.cancel(id)?;
        if signal {
            if let Some(cancel) = self
                .cancels
                .lock()
                .expect("OMR cancellation map poisoned")
                .get(&id)
            {
                cancel.store(true, Ordering::Release);
            }
        }
        Ok(job)
    }
}

pub async fn serve(config: Config) -> Result<()> {
    config.prepare()?;
    let queue = Queue::open(&config.home)?;
    queue.recover_interrupted(&config.audiveris_version)?;
    queue.remove_orphans()?;
    queue.prune_terminal()?;
    let capabilities = OmrCapabilities {
        service_version: env!("CARGO_PKG_VERSION").into(),
        audiveris_version: config.audiveris_version.clone(),
        formats: vec![
            ossia_types::OmrSourceFormat::Pdf,
            ossia_types::OmrSourceFormat::Png,
            ossia_types::OmrSourceFormat::Jpeg,
            ossia_types::OmrSourceFormat::Tiff,
        ],
        languages: config.languages.clone(),
        max_upload_bytes: config.max_upload_bytes,
        max_job_seconds: config.max_job_seconds,
    };
    let service = Service {
        queue: queue.clone(),
        token: Arc::from(config.token.as_str()),
        capabilities,
        cancels: Arc::new(Mutex::new(HashMap::new())),
        process: process::ProcessConfig {
            audiveris_bin: config.audiveris_bin.clone(),
            tessdata_dir: config.tessdata_dir.clone(),
            languages: config.languages.clone(),
            max_job_seconds: config.max_job_seconds,
            log_limit_bytes: config.log_limit_bytes,
        },
    };
    let stop = Arc::new(AtomicBool::new(false));
    let worker = worker::spawn(
        queue,
        service.process.clone(),
        service.cancels.clone(),
        stop.clone(),
    );
    let app = api::router(service.clone());
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .map_err(|error| {
            AppError::new(
                ErrorCode::RuntimeUnavailable,
                format!("cannot bind {}: {error}", config.listen),
            )
        })?;
    println!("Installed OCR languages: {}", config.languages.join(","));
    println!(
        "ossia-omr {} listening on {} (Audiveris {})",
        env!("CARGO_PKG_VERSION"),
        config.listen,
        config.audiveris_version
    );
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;
    stop.store(true, Ordering::Release);
    if let Ok(cancels) = service.cancels.lock() {
        for cancel in cancels.values() {
            cancel.store(true, Ordering::Release);
        }
    }
    let _ = worker.join();
    result.map_err(|error| {
        AppError::new(
            ErrorCode::RuntimeUnavailable,
            format!("OMR HTTP service failed: {error}"),
        )
    })
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
