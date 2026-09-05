pub(crate) mod antigravity;
pub(crate) mod claude;
pub(crate) mod cli;
pub(crate) mod manager;
pub(crate) mod types;

pub(crate) use manager::ProviderManager;
pub(crate) use types::{ProviderKind, ProviderSnapshot};
