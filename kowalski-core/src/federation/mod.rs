//! Multi-agent federation: ACL messages, in-process broker, registry.
//!
//! Start with [`MpscBroker`] + [`AgentRegistry`] in one process; Postgres `LISTEN`/`NOTIFY`
//! can mirror the same [`AclEnvelope`] JSON later.

mod acl;
mod broker;
mod orchestrator;
#[cfg(feature = "postgres")]
mod pg_broker;
mod persist;
mod registry;

pub use acl::{
    check_delegate_depth, AclEnvelope, AclMessage, ABSOLUTE_MAX_DELEGATION_DEPTH,
    DEFAULT_MAX_DELEGATION_DEPTH,
};
pub use broker::{MessageBroker, MpscBroker};
pub use orchestrator::{DelegationOutcome, FederationOrchestrator};
#[cfg(feature = "postgres")]
pub use pg_broker::{
    bridge_postgres_notify_to_mpsc, bridge_postgres_notify_to_mpsc_pool, pg_pool_connect, PgBroker,
};
pub use persist::{
    delete_federation_agent, load_registry_into, mark_stale_agents_inactive, set_agent_current_task,
    touch_agent_heartbeat, upsert_agent_state_for_record, upsert_registry_record,
};
#[cfg(feature = "postgres")]
pub use persist::{load_agent_states, AgentStateSnapshot};
pub use registry::{AgentRecord, AgentRegistry};
