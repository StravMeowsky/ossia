use crate::agent::AgentAdapter;
use crate::{antigravity, claude_code, codex, dsh, pi, workbuddy, HostPaths};

pub(crate) fn adapters(paths: &HostPaths) -> Vec<Box<dyn AgentAdapter>> {
    let mut adapters: Vec<Box<dyn AgentAdapter>> = vec![
        Box::new(codex::Codex),
        Box::new(dsh::Dsh),
        Box::new(pi::Pi),
        Box::new(antigravity::Antigravity),
        Box::new(claude_code::ClaudeCode),
    ];
    adapters.extend(
        workbuddy::instances(paths)
            .into_iter()
            .map(|instance| Box::new(instance) as Box<dyn AgentAdapter>),
    );
    adapters
}

pub(crate) fn find_profile<'a>(
    adapters: &'a [Box<dyn AgentAdapter>],
    id: &str,
    profile: &str,
) -> Option<&'a dyn AgentAdapter> {
    adapters
        .iter()
        .map(Box::as_ref)
        .find(|adapter| adapter.id() == id && adapter.profile() == profile)
}
