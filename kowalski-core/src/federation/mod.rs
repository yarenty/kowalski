//! Multi-agent federation: ACL messages, in-process broker, registry.
//!
//! Start with [`MpscBroker`] + [`AgentRegistry`] in one process; Postgres `LISTEN`/`NOTIFY`
//! can mirror the same [`AclEnvelope`] JSON later.

mod acl;
mod broker;
mod orchestrator;
mod registry;

pub use acl::{AclEnvelope, AclMessage};
pub use broker::{MessageBroker, MpscBroker};
pub use orchestrator::FederationOrchestrator;
pub use registry::{AgentRecord, AgentRegistry};
