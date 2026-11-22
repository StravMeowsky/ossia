use super::*;

pub(super) fn install(
    ctx: &AgentContext,
    previous: Option<&ManagedRecord>,
    overwrite: bool,
) -> Result<Vec<ManagedEntry>, AppError> {
    let (_version, files) = plugin_files(ctx)?;
    let plugin = plugin_entry(ctx)?;
    let marketplace = marketplace_entry(ctx)?;
    let mut conflicts = if overwrite {
        Vec::new()
    } else {
        file_conflicts(&files, previous)
    };
    if !overwrite {
        if let Some(marketplace) = marketplace.as_ref() {
            if !marketplace_matches(ctx, marketplace) {
                conflicts.push(conflict(
                    MARKETPLACE,
                    "user-config",
                    "the WorkBuddy marketplace name belongs to another source",
                ));
            }
        }
        if let Some(plugin) = plugin.as_ref() {
            if plugin.get("scope").and_then(Value::as_str) != Some("user") {
                conflicts.push(conflict(
                    PLUGIN_ID,
                    "user-config",
                    "the WorkBuddy plugin is installed in another scope",
                ));
            } else if previous.is_none() {
                conflicts.push(conflict(
                    PLUGIN_ID,
                    "unmanaged",
                    "the WorkBuddy plugin exists without an ownership record",
                ));
            }
        }
    }
    if !conflicts.is_empty() {
        return Err(conflict_error(&conflicts));
    }
    let previous_version = plugin
        .as_ref()
        .and_then(|plugin| plugin.get("version"))
        .and_then(Value::as_str)
        .map(str::to_string);
    refresh(ctx, &files, previous_version)
}

pub(super) fn sync(
    ctx: &AgentContext,
    record: &ManagedRecord,
) -> Result<Option<Vec<ManagedEntry>>, AppError> {
    let Some(plugin) = plugin_entry(ctx)? else {
        return Ok(None);
    };
    if !plugin
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(None);
    }
    let Some(marketplace) = marketplace_entry(ctx)? else {
        return Ok(None);
    };
    if !marketplace_matches(ctx, &marketplace) {
        return Ok(None);
    }
    if !record_file_conflicts(&record.entries).is_empty() {
        return Ok(None);
    }
    let (version, files) = plugin_files(ctx)?;
    if plugin.get("version").and_then(Value::as_str) == Some(version.as_str()) {
        return Ok(Some(record.entries.clone()));
    }
    let previous_version = plugin
        .get("version")
        .and_then(Value::as_str)
        .map(str::to_string);
    refresh(ctx, &files, previous_version).map(Some)
}

pub(super) fn remove(ctx: &AgentContext, record: &ManagedRecord) -> Result<(), AppError> {
    let plugin = plugin_entry(ctx)?;
    let marketplace = marketplace_entry(ctx)?;
    if let Some(marketplace) = marketplace.as_ref() {
        if !marketplace_matches(ctx, marketplace) {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "the WorkBuddy marketplace name belongs to another source",
            ));
        }
    }
    if let Some(plugin) = plugin.as_ref() {
        if plugin.get("scope").and_then(Value::as_str) != Some("user") {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "the WorkBuddy plugin is installed in another scope",
            ));
        }
        run(
            ctx,
            &["plugin", "uninstall", PLUGIN_ID, "--scope", "user", "-y"],
        )?;
    }
    if marketplace.is_some() {
        run(ctx, &["plugin", "marketplace", "remove", MARKETPLACE])?;
    }
    delete_entries(&record.entries)?;
    cleanup_shared_dir(ctx);
    Ok(())
}

fn refresh(
    ctx: &AgentContext,
    files: &[FilePlan],
    previous_version: Option<String>,
) -> Result<Vec<ManagedEntry>, AppError> {
    let snapshot = snapshot_files(files);
    if let Err(error) = write_files(files).and_then(|()| activate(ctx)) {
        let _ = restore_files(&snapshot);
        if let Some(version) = previous_version {
            let _ = activate_version(ctx, &version);
        }
        return Err(error);
    }
    Ok(records_from_files(files))
}

fn activate(ctx: &AgentContext) -> Result<(), AppError> {
    if marketplace_entry(ctx)?.is_some() {
        run(ctx, &["plugin", "marketplace", "remove", MARKETPLACE])?;
    } else if plugin_entry(ctx)?.is_some() {
        run(
            ctx,
            &["plugin", "uninstall", PLUGIN_ID, "--scope", "user", "-y"],
        )?;
    }
    let marketplace = path_text(marketplace_root(ctx));
    run(
        ctx,
        &[
            "plugin",
            "marketplace",
            "add",
            &marketplace,
            "--name",
            MARKETPLACE,
        ],
    )?;
    run(ctx, &["plugin", "install", PLUGIN_ID, "--scope", "user"])?;
    let plugin = plugin_entry(ctx)?.ok_or_else(|| {
        AppError::new(
            ErrorCode::RuntimeUnavailable,
            "WorkBuddy did not report the installed Ossia plugin",
        )
    })?;
    if !plugin
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err(AppError::new(
            ErrorCode::RuntimeUnavailable,
            "WorkBuddy installed the Ossia plugin but left it disabled",
        ));
    }
    Ok(())
}

fn activate_version(ctx: &AgentContext, version: &str) -> Result<(), AppError> {
    activate(ctx)?;
    let plugin = plugin_entry(ctx)?.ok_or_else(|| {
        AppError::new(
            ErrorCode::RuntimeUnavailable,
            "WorkBuddy did not report the restored Ossia plugin",
        )
    })?;
    if plugin.get("version").and_then(Value::as_str) != Some(version) {
        return Err(AppError::new(
            ErrorCode::RuntimeUnavailable,
            "WorkBuddy restored a different Ossia plugin version",
        ));
    }
    Ok(())
}
