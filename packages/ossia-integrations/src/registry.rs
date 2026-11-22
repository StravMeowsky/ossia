use crate::agent::AgentAdapter;
use crate::{codex, dsh, workbuddy, HostPaths};

pub(crate) fn adapters(paths: &HostPaths) -> Vec<Box<dyn AgentAdapter>> {
    let mut adapters: Vec<Box<dyn AgentAdapter>> = vec![Box::new(codex::Codex), Box::new(dsh::Dsh)];
    adapters.extend(
        workbuddy::instances(paths)
            .into_iter()
            .map(|instance| Box::new(instance) as Box<dyn AgentAdapter>),
    );
    adapters
}

pub(crate) fn find<'a>(
    adapters: &'a [Box<dyn AgentAdapter>],
    id: &str,
) -> Option<&'a dyn AgentAdapter> {
    adapters
        .iter()
        .map(Box::as_ref)
        .find(|adapter| adapter.id() == id)
}
