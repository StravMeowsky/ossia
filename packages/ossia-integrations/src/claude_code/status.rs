use super::*;

pub(super) fn status(
    ctx: &AgentContext,
    record: Option<&ManagedRecord>,
) -> Result<AgentInspection, AppError> {
    let plugin = plugin_entry(ctx)?;
    let marketplace = marketplace_entry(ctx)?;
    let mut conflicts = Vec::new();
    if let Some(marketplace) = marketplace.as_ref() {
        if !marketplace_matches(ctx, marketplace) {
            conflicts.push(conflict(
                MARKETPLACE,
                "user-config",
                "the Claude Code marketplace name belongs to another source",
            ));
        }
    }
    if let Some(plugin) = plugin.as_ref() {
        if plugin.get("scope").and_then(Value::as_str) != Some("user") {
            conflicts.push(conflict(
                PLUGIN_ID,
                "user-config",
                "the Claude Code plugin is installed in another scope",
            ));
        }
        if record.is_none() {
            conflicts.push(conflict(
                PLUGIN_ID,
                "unmanaged",
                "the Claude Code plugin exists without an ownership record",
            ));
        }
    }
    if let Some(record) = record {
        conflicts.extend(record_file_conflicts(&record.entries));
    }
    let installed = plugin
        .as_ref()
        .is_some_and(|plugin| plugin.get("installPath").and_then(Value::as_str).is_some())
        && marketplace
            .as_ref()
            .is_some_and(|marketplace| marketplace_matches(ctx, marketplace));
    let enabled = plugin
        .as_ref()
        .and_then(|plugin| plugin.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let install_path_missing = plugin
        .as_ref()
        .and_then(|plugin| plugin.get("installPath"))
        .and_then(Value::as_str)
        .is_some_and(|path| !Path::new(path).exists());
    let missing_files = record.is_some_and(|record| record_files_missing(&record.entries));
    let managed = record.is_some();
    let state = if !conflicts.is_empty() {
        AgentState::Conflict
    } else if missing_files || install_path_missing || (managed && !installed) {
        AgentState::Broken
    } else if installed && !enabled {
        AgentState::Disabled
    } else if installed {
        AgentState::Installed
    } else {
        AgentState::Available
    };
    let message = match state {
        AgentState::Disabled => Some(AgentNotice::RestartSession {
            agent: "Claude Code".into(),
        }),
        AgentState::Broken => Some(AgentNotice::RestartSession {
            agent: "Claude Code".into(),
        }),
        AgentState::Conflict => Some(AgentNotice::RestartSession {
            agent: "Claude Code".into(),
        }),
        _ => None,
    };
    let path = plugin
        .as_ref()
        .and_then(|plugin| plugin.get("installPath"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| path_text(plugin_dir(ctx)));
    Ok(AgentInspection {
        state,
        installed,
        enabled,
        managed,
        path: Some(path),
        message,
        conflicts,
    })
}
