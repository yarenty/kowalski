pub mod agent;
pub mod config;
pub mod conversation;
pub mod db;
pub mod error;
pub mod federation;
pub mod graph;
pub mod horde_graph;
pub mod horde_stages;
pub mod horde_step;
pub mod llm;
pub mod source_bundle;
pub mod logging;
pub mod markdown_pipeline;
pub mod operator_input;
pub mod mcp;
pub mod rookery;
pub mod memory;
pub mod model;
pub mod role;
pub mod template;
pub mod tool_chain;
pub mod tools;
pub mod utils;

pub use agent::repl_trace::ReplTraceGuard;
pub use agent::{Agent, BaseAgent, MessageHandler};
pub use config::*;
// pub use conversation::*; // Remove this to avoid ToolCall ambiguity
pub use error::KowalskiError;
pub use federation::{
    ABSOLUTE_MAX_DELEGATION_DEPTH, AclEnvelope, AclMessage, AgentRecord, AgentRegistry,
    DEFAULT_MAX_DELEGATION_DEPTH, DelegationOutcome, FederationOrchestrator, MessageBroker,
    MpscBroker, check_delegate_depth, delete_federation_agent, load_registry_into,
    mark_stale_agents_inactive, set_agent_current_task, touch_agent_heartbeat,
    upsert_agent_state_for_record, upsert_registry_record,
};
#[cfg(feature = "postgres")]
pub use federation::{AgentStateSnapshot, load_agent_states};
#[cfg(feature = "postgres")]
pub use federation::{
    PgBroker, bridge_postgres_notify_to_mpsc, bridge_postgres_notify_to_mpsc_pool, pg_pool_connect,
};
pub use graph::{postgres_age_cypher, postgres_graph_status};
pub use logging::*;
pub use horde_stages::{
    apply_patches_dry_run, extract_unified_diffs, format_apply_artifact, format_verify_artifact,
    verify_output_excerpt,
    parse_stage_status_from_artifact, resolve_verify_cwd, run_verify_command, ApplyDryRunResult,
    StageStatus, VerifyRunResult, DEFAULT_VERIFY_MAX_OUTPUT_BYTES, DEFAULT_VERIFY_TIMEOUT_SECS,
};
pub use horde_step::{
    ApplyStepHandler, IngestStepHandler, LlmStepHandler, NullEventSink, StepContext, StepError,
    StepEventSink, StepHandler, StepHandlerRegistry, StepOutcome, StepSpec, VerifyStepHandler,
    LLM_STEP_KINDS,
};
pub use horde_graph::{
    all_steps_successful, edge_matches_outcome, execution_order, has_conditional_outbound,
    inbound_predecessors, is_loop_back_edge, is_loop_back_step, loop_edge_key, next_ready_step, next_ready_step_conditional,
    outbound_edges, resolve_execution_graph, retry_span, select_next_from_outcome,
    should_persist_edges, single_forward_predecessor, single_predecessor, ExecutionGraph, HordeEdge,
};
pub use markdown_pipeline::{
    maybe_normalize_markdown, parse_app_manifest, parse_stage_agent, render_context_attachments,
    resolve_manifest_path, AppManifestMeta, StageAgentMeta,
};
pub use operator_input::{
    answers_to_prompt, default_ingest_form_fields, operator_answer, parse_operator_answer_block,
    validate_form_answers, HordeRunFormSpec, OperatorInputField,
};
pub use rookery::{
    assign_penguin_avatars, extract_json_block, horde_root_path, infer_penguin_avatar,
    minimal_dag_draft, minimal_linear_draft, parse_draft_from_assistant, normalize_draft, output_looks_invalid,
    repair_horde_tree_outputs, validate_draft, validate_horde_tree, write_horde_tree,
    HordeBirthSpec, PenguinSpec, RookeryDraft,
};
pub use mcp::{
    CallToolResponse, McpClient, McpConnection, McpHub, McpStdioClient, McpToolBinding,
    McpToolDescription, McpToolProxy,
};
pub use model::ModelManager;
pub use model::*;
pub use role::{Audience, Preset, Role, Style};
pub use tool_chain::*;
pub use tools::policy::ToolExecutionPolicy;
pub use tools::ToolCall;
pub use tools::*;
