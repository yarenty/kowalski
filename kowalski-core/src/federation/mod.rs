//! Multi-agent federation: ACL messages, in-process broker, registry.
//!
//! Start with [`MpscBroker`] + [`AgentRegistry`] in one process; Postgres `LISTEN`/`NOTIFY`
//! can mirror the same [`AclEnvelope`] JSON later.

mod acl;
mod broker;
mod orchestrator;
#[cfg(feature = "postgres")]
mod pg_broker;
mod registry;

pub use acl::{check_delegate_depth, AclEnvelope, AclMessage};
pub use broker::{MessageBroker, MpscBroker};
pub use orchestrator::FederationOrchestrator;
#[cfg(feature = "postgres")]
pub use pg_broker::PgBroker;
pub use registry::{AgentRecord, AgentRegistry};
