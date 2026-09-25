pub(crate) mod antigravity;
pub(crate) mod claude;
pub(crate) mod cli;
pub mod executor;
pub(crate) mod manager;
pub(crate) mod types;

pub(crate) use manager::ProviderManager;
pub use types::ProviderKind;
pub(crate) use types::{ProviderSnapshot, SelfTestRecord};
