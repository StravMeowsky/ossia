use super::*;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::time::{Duration, Instant};

const PREVIEW_MEDIA_TYPE: &str = "image/svg+xml";

impl Runtime {
    /// Page through active scores for the management window.
    pub fn management_scores(&self, args: ListArgs) -> Result<ManagementScorePage> {
        self.management_score_page(false, args)
    }

    /// Page through recoverable scores for the management window.
    pub fn management_archived_scores(&self, args: ListArgs) -> Result<ManagementScorePage> {
        self.management_score_page(true, args)
    }

    fn management_score_page(&self, archived: bool, args: ListArgs) -> Result<ManagementScorePage> {
        self.available()?;
        let limit = args.limit.unwrap_or(50);
        if !(1..=200).contains(&limit) {
            return Err(invalid("management score limit must be 1..200"));
        }
        self.library
            .management_score_page(archived, limit, args.offset)
    }

    /// Return a cached first-page preview, generating one only for visible active scores.
    pub fn management_score_preview(
        &self,
        args: ManagementScorePreviewArgs,
    ) -> Result<ManagementScorePreview> {
        self.available()?;
        if let Some(bytes) = self.library.management_score_preview(args.document_id)? {
            return Ok(ManagementScorePreview::Ready {
                data_url: format!("data:{PREVIEW_MEDIA_TYPE};base64,{}", BASE64.encode(bytes)),
            });
        }
        if self.library.is_trashed(args.document_id)? {
            return Ok(ManagementScorePreview::Missing);
        }
        let document = self.library.get_document(args.document_id)?;
        if let Some(job) = &document.last_check {
            match job.state {
                JobState::Queued | JobState::Running => {
                    return Ok(ManagementScorePreview::Pending { job_id: job.job_id });
                }
                JobState::Error | JobState::Cancelled if !args.retry => {
                    return Ok(ManagementScorePreview::Failed {
                        error: job.error.clone().unwrap_or_else(|| {
                            AppError::new(
                                ErrorCode::Cancelled,
                                "preview generation was interrupted",
                            )
                        }),
                    });
                }
                JobState::Done if !args.retry => {
                    return Ok(ManagementScorePreview::Missing);
                }
                _ => {}
            }
        }
        let job = self.check(
            CheckArgs {
                expect: None,
                reference: None,
                document_id: Some(args.document_id),
                edit_id: None,
                expected_source_hash: document.source_hash,
                wait_ms: Some(0),
            },
            true,
        )?;
        Ok(ManagementScorePreview::Pending { job_id: job.job_id })
    }

    /// Move an active score to the recoverable archive after stopping preview-only work.
    pub fn management_archive_score(&self, args: DocumentIdArgs) -> Result<TrashedDocumentSummary> {
        self.available()?;
        let preview_jobs = {
            let state = self.state();
            state
                .pending
                .iter()
                .filter(|pending| {
                    pending.preview && pending.job.document_id == Some(args.document_id)
                })
                .map(|pending| pending.job.job_id)
                .chain(
                    state
                        .running
                        .iter()
                        .filter(|(_, running)| {
                            running.preview && running.document == Some(args.document_id)
                        })
                        .map(|(job_id, _)| *job_id),
                )
                .collect::<Vec<_>>()
        };
        for &job_id in &preview_jobs {
            self.cancel(job_id)?;
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        while preview_jobs_active(&self.library, &preview_jobs)? {
            if Instant::now() >= deadline {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "preview generation did not stop before archiving",
                ));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let document = self.library.get_document(args.document_id)?;
        self.delete_document(DeleteArgs {
            document_id: args.document_id,
            expected_source_hash: document.source_hash,
            force: false,
        })
    }

    /// Return the stable browser URL for an active score.
    pub fn management_score_url(&self, args: DocumentIdArgs) -> Result<String> {
        self.available()?;
        self.library.get_document(args.document_id)?;
        Ok(self.document_url(args.document_id))
    }
}

fn preview_jobs_active(library: &ossia_library::Library, job_ids: &[JobId]) -> Result<bool> {
    job_ids.iter().try_fold(false, |active, job_id| {
        Ok(active
            || matches!(
                library.get_job(*job_id)?.state,
                JobState::Queued | JobState::Running
            ))
    })
}
