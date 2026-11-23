use crate::{
    cache, failure, io_error, process::Scratch, resources::ResourceFile, AppError, ContentHash,
    Engine, ErrorCode, RunContext,
};
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

fn resource_file(root: &Path, file: &ResourceFile, context: &RunContext) -> Result<(), AppError> {
    context.check()?;
    let path = cache::confined(root, &file.path)?;
    let bytes = fs::read(&path).map_err(io_error)?;
    if bytes.len() as u64 != file.bytes || ContentHash::of(&bytes) != file.sha256 {
        return Err(failure("integrity", "engine resource is damaged"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if file.executable
            && fs::metadata(path).map_err(io_error)?.permissions().mode() & 0o111 == 0
        {
            return Err(failure(
                "integrity",
                "engine resource lost executable permissions",
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = file.executable;
    Ok(())
}

pub(super) fn resources(
    root: &Path,
    files: &[ResourceFile],
    context: &RunContext,
) -> Result<BTreeSet<String>, AppError> {
    let mut paths = BTreeSet::new();
    for file in files {
        if !paths.insert(file.path.clone()) {
            return Err(failure("integrity", "duplicate resource entry"));
        }
    }
    let workers = std::thread::available_parallelism()
        .map_or(1, |count| count.get())
        .min(files.len());
    if workers <= 1 {
        for file in files {
            resource_file(root, file, context)?;
        }
        return Ok(paths);
    }
    // Integrity checks are read-only and cancellation-aware, so large immutable
    // resource sets can be hashed concurrently without changing their order.
    let next = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let first_failure = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            handles.push(scope.spawn(|| {
                let mut first = None;
                loop {
                    if failed.load(Ordering::Acquire) {
                        break;
                    }
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(file) = files.get(index) else {
                        break;
                    };
                    if let Err(error) = resource_file(root, file, context) {
                        failed.store(true, Ordering::Release);
                        first = Some((index, error));
                        break;
                    }
                }
                first
            }));
        }
        handles
            .into_iter()
            .filter_map(|handle| {
                handle
                    .join()
                    .expect("engine resource verifier thread panicked")
            })
            .min_by_key(|(index, _)| *index)
    });
    match first_failure {
        Some((_, error)) => Err(error),
        None => Ok(paths),
    }
}

fn tool_version(
    engine: &Engine,
    name: &str,
    version: &regex::Regex,
    context: &RunContext,
) -> Result<(), AppError> {
    let scratch = Scratch::new(&engine.scratch_root)?;
    let output = engine.run(name, &["--version"], &scratch, context)?;
    if !output.success || !version.is_match(&output.log) {
        return Err(AppError::new(
            ErrorCode::RuntimeIncompatible,
            "engine tool version does not match 2.26.0",
        ));
    }
    Ok(())
}

fn svg_smoke(engine: &Engine, context: &RunContext) -> Result<(), AppError> {
    let scratch = Scratch::new(&engine.scratch_root)?;
    fs::write(
        scratch.path().join("score.ly"),
        "\\version \"2.26.0\"\n\\score { { c'1 } \\layout {} }\n",
    )
    .map_err(io_error)?;
    let output = engine.run(
        "lilypond",
        &["--svg", "-o", "score", "score.ly"],
        &scratch,
        context,
    )?;
    if !output.success || !scratch.path().join("score.svg").is_file() {
        return Err(failure("engine-smoke", "engine SVG smoke test failed"));
    }
    Ok(())
}

pub(super) fn probes(engine: &Engine, context: &RunContext) -> Result<(), AppError> {
    let version = regex::Regex::new(r"\b2\.26\.0(?:\s|$)").unwrap();
    // Every probe has its own scratch directory and can fail independently.
    let first_failure = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(5);
        for (index, name) in ["lilypond", "musicxml2ly", "midi2ly", "convert-ly"]
            .into_iter()
            .enumerate()
        {
            let version = &version;
            handles
                .push(scope.spawn(move || (index, tool_version(engine, name, version, context))));
        }
        handles.push(scope.spawn(|| (4, svg_smoke(engine, context))));
        handles
            .into_iter()
            .filter_map(|handle| {
                let (index, result) = handle
                    .join()
                    .expect("engine verification probe thread panicked");
                result.err().map(|error| (index, error))
            })
            .min_by_key(|(index, _)| *index)
    });
    match first_failure {
        Some((_, error)) => Err(error),
        None => Ok(()),
    }
}
