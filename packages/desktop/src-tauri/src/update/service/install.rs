//! Installing a verified download and handing it back to the caller.

use super::*;
use crate::update::install::{self, Installation};

#[derive(Debug)]
pub enum InstallOutcome {
    NotStarted(UpdateSnapshot),
    Failed(UpdateSnapshot),
    Completed(UpdateSnapshot, Installation),
}

impl UpdateService {
    pub async fn install(&self) -> InstallOutcome {
        self.install_with(|staged| {
            let installation = install::prepare()?;
            installation.apply(staged, true)?;
            Ok(installation)
        })
        .await
    }

    async fn install_with(
        &self,
        apply: impl FnOnce(&Path) -> Result<Installation> + Send + 'static,
    ) -> InstallOutcome {
        let staged = match self.begin_install() {
            Ok(staged) => staged,
            Err(outcome) => return *outcome,
        };
        let outcome = tokio::task::spawn_blocking(move || apply(&staged)).await;
        self.settle_install(outcome)
    }

    fn begin_install(&self) -> std::result::Result<PathBuf, Box<InstallOutcome>> {
        if !self.begin_with(true) {
            return Err(Box::new(InstallOutcome::NotStarted(self.snapshot())));
        }
        let Some(staged) = self.staged() else {
            self.clear_installing();
            let failed = self.fail(&Error::other("no downloaded update to install"));
            self.set(failed.clone());
            return Err(Box::new(InstallOutcome::Failed(failed)));
        };
        self.set(UpdateSnapshot {
            state: UpdateState::Installing,
            error: None,
            ..self.base_snapshot()
        });
        Ok(staged)
    }

    fn settle_install(
        &self,
        outcome: std::result::Result<Result<Installation>, tokio::task::JoinError>,
    ) -> InstallOutcome {
        self.clear_installing();
        match outcome {
            Ok(Ok(installation)) => {
                if !cfg!(target_os = "windows") {
                    if let Some(dir) = self
                        .staged()
                        .and_then(|path| path.parent().map(Path::to_path_buf))
                    {
                        let _ = std::fs::remove_dir_all(dir);
                    }
                }
                *self.staged.lock().unwrap() = None;
                self.set(UpdateSnapshot {
                    state: UpdateState::Installing,
                    error: None,
                    ..self.base_snapshot()
                });
                InstallOutcome::Completed(self.snapshot(), installation)
            }
            failed => {
                let error = match failed {
                    Ok(Err(error)) => error,
                    Err(error) => Error::other(format!("install: {error}")),
                    Ok(Ok(_)) => unreachable!(),
                };
                let failed = self.fail(&error);
                self.set(failed.clone());
                InstallOutcome::Failed(failed)
            }
        }
    }
}
