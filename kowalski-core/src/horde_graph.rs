//! Horde execution graph: optional `[[edges]]` on manifests / Rookery drafts.
//!
//! When `edges` is absent or empty, the graph is an implicit linear chain following
//! `pipeline` order. Scheduling layers are derived via Kahn topological sort.

use crate::error::KowalskiError;
use crate::horde_stages::StageStatus;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// One scheduling dependency: `from` must complete before `to` runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HordeEdge {
    pub from: String,
    pub to: String,
    /// Route only when upstream outcome matches: `pass`, `fail`, `always`, or omitted (= always).
    #[serde(default)]
    pub when: Option<String>,
    /// Required on loop-back edges (`to` before `from` in `pipeline`); max traversals.
    #[serde(default)]
    pub max_loops: Option<u32>,
}

/// Validated execution graph ready for orchestrator scheduling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionGraph {
    /// Normalized edges (explicit or implicit linear chain).
    pub edges: Vec<HordeEdge>,
    /// Topological layers: layer 0 has no inbound deps; within a layer, steps may run in parallel.
    pub layers: Vec<Vec<String>>,
}

/// Resolve and validate the execution graph for a horde pipeline.
///
/// `edges`: `None` or empty → implicit chain `pipeline[i] → pipeline[i+1]`.
pub fn resolve_execution_graph(
    pipeline: &[String],
    edges: Option<&[HordeEdge]>,
) -> Result<ExecutionGraph, KowalskiError> {
    if pipeline.is_empty() {
        return Err(KowalskiError::Validation(
            "pipeline must contain at least one step".into(),
        ));
    }

    let pipeline_set: BTreeSet<_> = pipeline.iter().cloned().collect();
    if pipeline_set.len() != pipeline.len() {
        return Err(KowalskiError::Validation(
            "pipeline contains duplicate step names".into(),
        ));
    }

    let effective = effective_edges(pipeline, edges)?;
    validate_edge_endpoints(&effective, &pipeline_set)?;
    validate_no_self_loops(&effective)?;
    validate_no_duplicate_edges(&effective)?;
    validate_all_steps_connected(pipeline, &effective)?;
    validate_forward_acyclic(pipeline, &effective)?;
    validate_edge_topology(pipeline, &effective)?;

    let (forward, _) = partition_edges(pipeline, &effective);
    let layers = compute_layers(pipeline, &forward)?;

    Ok(ExecutionGraph {
        edges: effective,
        layers,
    })
}

/// True when explicit `edges` differ from the implicit linear chain (emit `[[edges]]` on birth).
pub fn should_persist_edges(pipeline: &[String], edges: &[HordeEdge]) -> bool {
    if edges.is_empty() {
        return false;
    }
    match (
        resolve_execution_graph(pipeline, None),
        resolve_execution_graph(pipeline, Some(edges)),
    ) {
        (Ok(linear), Ok(dag)) => linear.edges != dag.edges,
        _ => true,
    }
}

/// Inbound scheduling predecessors of `step` (empty for graph sources).
pub fn inbound_predecessors(edges: &[HordeEdge], step: &str) -> Vec<String> {
    predecessor_map(edges)
        .get(step)
        .cloned()
        .unwrap_or_default()
}

fn effective_edges(
    pipeline: &[String],
    edges: Option<&[HordeEdge]>,
) -> Result<Vec<HordeEdge>, KowalskiError> {
    match edges {
        Some(explicit) if !explicit.is_empty() => Ok(explicit.to_vec()),
        _ => Ok(implicit_chain_edges(pipeline)),
    }
}

fn implicit_chain_edges(pipeline: &[String]) -> Vec<HordeEdge> {
    pipeline
        .windows(2)
        .map(|w| HordeEdge {
            from: w[0].clone(),
            to: w[1].clone(),
            when: None,
            max_loops: None,
        })
        .collect()
}

fn validate_edge_endpoints(
    edges: &[HordeEdge],
    pipeline_set: &BTreeSet<String>,
) -> Result<(), KowalskiError> {
    for e in edges {
        if !pipeline_set.contains(&e.from) {
            return Err(KowalskiError::Validation(format!(
                "edge from `{from}` references step not in pipeline",
                from = e.from
            )));
        }
        if !pipeline_set.contains(&e.to) {
            return Err(KowalskiError::Validation(format!(
                "edge to `{to}` references step not in pipeline",
                to = e.to
            )));
        }
    }
    Ok(())
}

fn validate_no_self_loops(edges: &[HordeEdge]) -> Result<(), KowalskiError> {
    for e in edges {
        if e.from == e.to {
            return Err(KowalskiError::Validation(format!(
                "self-loop edge `{from}` → `{to}` is not allowed",
                from = e.from,
                to = e.to
            )));
        }
    }
    Ok(())
}

fn validate_no_duplicate_edges(edges: &[HordeEdge]) -> Result<(), KowalskiError> {
    let mut seen = HashSet::new();
    for e in edges {
        let key = (&e.from, &e.to);
        if !seen.insert(key) {
            return Err(KowalskiError::Validation(format!(
                "duplicate edge `{from}` → `{to}`",
                from = e.from,
                to = e.to
            )));
        }
    }
    Ok(())
}

fn pipeline_index(pipeline: &[String]) -> BTreeMap<String, usize> {
    pipeline
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i))
        .collect()
}

pub fn is_loop_back_step(pipeline: &[String], from: &str, to: &str) -> bool {
    let pos = pipeline_index(pipeline);
    pos.get(from)
        .zip(pos.get(to))
        .is_some_and(|(a, b)| b <= a)
}

pub fn is_loop_back_edge(pipeline: &[String], edge: &HordeEdge) -> bool {
    is_loop_back_step(pipeline, &edge.from, &edge.to)
}

fn partition_edges(pipeline: &[String], edges: &[HordeEdge]) -> (Vec<HordeEdge>, Vec<HordeEdge>) {
    let pos = pipeline_index(pipeline);
    let mut forward = Vec::new();
    let mut back = Vec::new();
    for e in edges {
        if pos.get(&e.to).unwrap_or(&0) > pos.get(&e.from).unwrap_or(&0) {
            forward.push(e.clone());
        } else {
            back.push(e.clone());
        }
    }
    (forward, back)
}

fn validate_forward_acyclic(pipeline: &[String], edges: &[HordeEdge]) -> Result<(), KowalskiError> {
    let (forward, _) = partition_edges(pipeline, edges);
    validate_acyclic(pipeline, &forward)
}

fn validate_edge_topology(pipeline: &[String], edges: &[HordeEdge]) -> Result<(), KowalskiError> {
    let (forward, back) = partition_edges(pipeline, edges);
    validate_pipeline_topological_order(pipeline, &forward)?;
    for e in &back {
        if e.when.as_ref().is_none_or(|w| w.trim().is_empty()) {
            return Err(KowalskiError::Validation(format!(
                "loop-back edge `{from}` → `{to}` requires `when` (pass/fail)",
                from = e.from,
                to = e.to
            )));
        }
        if e.max_loops.unwrap_or(0) == 0 {
            return Err(KowalskiError::Validation(format!(
                "loop-back edge `{from}` → `{to}` requires `max_loops` > 0",
                from = e.from,
                to = e.to
            )));
        }
    }
    Ok(())
}

pub fn loop_edge_key(from: &str, to: &str) -> String {
    format!("{from}->{to}")
}

pub fn edge_matches_outcome(edge: &HordeEdge, outcome: StageStatus) -> bool {
    match edge
        .when
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
    {
        None => true,
        Some(ref s) if s.is_empty() || s == "always" => true,
        Some(ref s) if s == "pass" => outcome == StageStatus::Pass,
        Some(ref s) if s == "fail" => outcome == StageStatus::Fail,
        _ => false,
    }
}

pub fn outbound_edges<'a>(edges: &'a [HordeEdge], from: &str) -> Vec<&'a HordeEdge> {
    edges.iter().filter(|e| e.from == from).collect()
}

pub fn has_conditional_outbound(edges: &[HordeEdge], from: &str) -> bool {
    outbound_edges(edges, from)
        .iter()
        .any(|e| e.when.as_ref().is_some_and(|w| !w.trim().is_empty()))
}

/// After `from_step` completes with `outcome`, pick the next pipeline step (conditional + loop caps).
pub fn select_next_from_outcome(
    pipeline: &[String],
    edges: &[HordeEdge],
    from_step: &str,
    outcome: StageStatus,
    loop_counts: &BTreeMap<String, u32>,
) -> Option<String> {
    let pos = pipeline_index(pipeline);
    let from_pos = *pos.get(from_step)?;
    let mut matching: Vec<_> = outbound_edges(edges, from_step)
        .into_iter()
        .filter(|e| edge_matches_outcome(e, outcome))
        .collect();
    if matching.is_empty() {
        return None;
    }
    matching.sort_by_key(|e| pos.get(&e.to).copied().unwrap_or(usize::MAX));
    for edge in matching.iter().filter(|e| pos.get(&e.to).copied().unwrap_or(0) > from_pos) {
        return Some(edge.to.clone());
    }
    for edge in matching.iter().filter(|e| pos.get(&e.to).copied().unwrap_or(0) <= from_pos) {
        let key = loop_edge_key(&edge.from, &edge.to);
        let count = loop_counts.get(&key).copied().unwrap_or(0);
        let max = edge.max_loops.unwrap_or(1);
        if count < max {
            return Some(edge.to.clone());
        }
    }
    None
}

/// Steps to reset when traversing a loop-back edge (inclusive span in `pipeline`).
pub fn retry_span(pipeline: &[String], from: &str, through: &str) -> Vec<String> {
    let pos = pipeline_index(pipeline);
    let Some(a) = pos.get(from).copied() else {
        return vec![from.to_string()];
    };
    let Some(b) = pos.get(through).copied() else {
        return vec![from.to_string()];
    };
    let (start, end) = if a <= b { (a, b) } else { (b, a) };
    pipeline[start..=end].to_vec()
}

/// Next pending step whose inbound predecessors completed and conditional edges match outcomes.
pub fn next_ready_step_conditional(
    pipeline: &[String],
    graph: &ExecutionGraph,
    step_status: &BTreeMap<String, &str>,
    step_outcome: &BTreeMap<String, StageStatus>,
) -> Option<String> {
    let preds = forward_predecessor_map(pipeline, &graph.edges);
    for step in pipeline {
        if step_status.get(step.as_str()).copied() != Some("pending") {
            continue;
        }
        let prerequisites = preds.get(step).map(|v| v.as_slice()).unwrap_or(&[]);
        if prerequisites.is_empty() {
            return Some(step.clone());
        }
        let mut satisfied = true;
        for pred in prerequisites {
            if step_status.get(pred.as_str()) != Some(&"success") {
                satisfied = false;
                break;
            }
            let edges_in: Vec<_> = graph
                .edges
                .iter()
                .filter(|e| e.from == *pred && e.to == *step)
                .collect();
            if edges_in.is_empty() {
                continue;
            }
            if edges_in.iter().any(|e| e.when.is_some()) {
                let outcome = step_outcome
                    .get(pred.as_str())
                    .copied()
                    .unwrap_or(StageStatus::Pass);
                if !edges_in.iter().any(|e| edge_matches_outcome(e, outcome)) {
                    satisfied = false;
                    break;
                }
            }
        }
        if satisfied {
            return Some(step.clone());
        }
    }
    None
}

/// Detect cycles via Kahn's algorithm (independent of pipeline order).
fn validate_acyclic(pipeline: &[String], edges: &[HordeEdge]) -> Result<(), KowalskiError> {
    let mut in_degree: HashMap<String, usize> =
        pipeline.iter().map(|s| (s.clone(), 0)).collect();
    let mut adj: HashMap<String, Vec<String>> =
        pipeline.iter().map(|s| (s.clone(), vec![])).collect();

    for e in edges {
        *in_degree.get_mut(&e.to).unwrap() += 1;
        adj.get_mut(&e.from).unwrap().push(e.to.clone());
    }

    let mut remaining = in_degree.clone();
    let mut processed = 0usize;

    loop {
        let ready: Vec<String> = remaining
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(name, _)| name.clone())
            .collect();

        if ready.is_empty() {
            if processed < pipeline.len() {
                return Err(KowalskiError::Validation(
                    "execution graph contains a cycle".into(),
                ));
            }
            break;
        }

        for node in &ready {
            for succ in &adj[node] {
                if let Some(deg) = remaining.get_mut(succ) {
                    *deg -= 1;
                }
            }
        }
        processed += ready.len();
        for node in &ready {
            remaining.remove(node);
        }
    }

    Ok(())
}

/// Every edge must respect `pipeline` order (pipeline is *a* valid topological sort).
fn validate_pipeline_topological_order(
    pipeline: &[String],
    edges: &[HordeEdge],
) -> Result<(), KowalskiError> {
    let pos: BTreeMap<_, _> = pipeline
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i))
        .collect();
    for e in edges {
        let from_pos = pos[&e.from];
        let to_pos = pos[&e.to];
        if from_pos >= to_pos {
            return Err(KowalskiError::Validation(format!(
                "pipeline order violates edge `{from}` → `{to}` (from must appear before to in pipeline)",
                from = e.from,
                to = e.to
            )));
        }
    }
    Ok(())
}

/// Each pipeline step must participate in at least one edge (when len > 1).
fn validate_all_steps_connected(
    pipeline: &[String],
    edges: &[HordeEdge],
) -> Result<(), KowalskiError> {
    if pipeline.len() <= 1 {
        return Ok(());
    }
    let mut touched = HashSet::new();
    for e in edges {
        touched.insert(e.from.clone());
        touched.insert(e.to.clone());
    }
    for step in pipeline {
        if !touched.contains(step) {
            return Err(KowalskiError::Validation(format!(
                "pipeline step `{step}` is not connected by any edge"
            )));
        }
    }
    Ok(())
}

fn compute_layers(
    pipeline: &[String],
    edges: &[HordeEdge],
) -> Result<Vec<Vec<String>>, KowalskiError> {
    let mut in_degree: HashMap<String, usize> =
        pipeline.iter().map(|s| (s.clone(), 0)).collect();
    let mut adj: HashMap<String, Vec<String>> =
        pipeline.iter().map(|s| (s.clone(), vec![])).collect();

    for e in edges {
        *in_degree.get_mut(&e.to).unwrap() += 1;
        adj.get_mut(&e.from).unwrap().push(e.to.clone());
    }

    let pipeline_index: BTreeMap<_, _> = pipeline
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i))
        .collect();

    let mut remaining = in_degree.clone();
    let mut layers = Vec::new();
    let mut processed = 0usize;

    loop {
        let mut ready: Vec<String> = remaining
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(name, _)| name.clone())
            .collect();

        if ready.is_empty() {
            if processed < pipeline.len() {
                return Err(KowalskiError::Validation(
                    "execution graph contains a cycle".into(),
                ));
            }
            break;
        }

        ready.sort_by_key(|name| pipeline_index[name]);
        for node in &ready {
            for succ in &adj[node] {
                if let Some(deg) = remaining.get_mut(succ) {
                    *deg -= 1;
                }
            }
        }
        processed += ready.len();
        for node in &ready {
            remaining.remove(node);
        }
        layers.push(ready);
    }

    Ok(layers)
}

/// Flatten topological layers into execution order (sequential within each layer, MVP).
pub fn execution_order(graph: &ExecutionGraph) -> Vec<String> {
    graph.layers.iter().flatten().cloned().collect()
}

fn predecessor_map(edges: &[HordeEdge]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for e in edges {
        map.entry(e.to.clone()).or_default().push(e.from.clone());
    }
    map
}

fn forward_predecessor_map(
    pipeline: &[String],
    edges: &[HordeEdge],
) -> HashMap<String, Vec<String>> {
    let pos = pipeline_index(pipeline);
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for e in edges {
        if pos.get(&e.to).copied().unwrap_or(0) > pos.get(&e.from).copied().unwrap_or(0) {
            map.entry(e.to.clone()).or_default().push(e.from.clone());
        }
    }
    map
}

/// Next `pending` step whose inbound edges are all `success` (first in `pipeline` order).
pub fn next_ready_step(
    pipeline: &[String],
    graph: &ExecutionGraph,
    step_status: &BTreeMap<String, &str>,
) -> Option<String> {
    let preds = predecessor_map(&graph.edges);
    for step in pipeline {
        if step_status.get(step.as_str()).copied() != Some("pending") {
            continue;
        }
        let prerequisites = preds.get(step).map(|v| v.as_slice()).unwrap_or(&[]);
        if prerequisites
            .iter()
            .all(|p| step_status.get(p.as_str()) == Some(&"success"))
        {
            return Some(step.clone());
        }
    }
    None
}

pub fn all_steps_successful(pipeline: &[String], step_status: &BTreeMap<String, &str>) -> bool {
    pipeline
        .iter()
        .all(|s| step_status.get(s.as_str()) == Some(&"success"))
}

/// When a step has exactly one inbound edge, return that predecessor (for `@artifact@`).
pub fn single_predecessor(graph: &ExecutionGraph, step: &str) -> Option<String> {
    let preds = predecessor_map(&graph.edges);
    let ps = preds.get(step)?;
    if ps.len() == 1 {
        Some(ps[0].clone())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(from: &str, to: &str) -> HordeEdge {
        HordeEdge {
            from: from.into(),
            to: to.into(),
            when: None,
            max_loops: None,
        }
    }

    fn pipeline(steps: &[&str]) -> Vec<String> {
        steps.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn implicit_linear_chain_when_edges_absent() {
        let pipe = pipeline(&["ingest", "compile", "lint"]);
        let g = resolve_execution_graph(&pipe, None).unwrap();
        assert_eq!(
            g.edges,
            vec![edge("ingest", "compile"), edge("compile", "lint")]
        );
        assert_eq!(
            g.layers,
            vec![
                vec!["ingest".to_string()],
                vec!["compile".to_string()],
                vec!["lint".to_string()],
            ]
        );
    }

    #[test]
    fn implicit_linear_when_edges_empty_slice() {
        let pipe = pipeline(&["a", "b"]);
        let g = resolve_execution_graph(&pipe, Some(&[])).unwrap();
        assert_eq!(g.edges, vec![edge("a", "b")]);
    }

    #[test]
    fn fork_join_layers() {
        let pipe = pipeline(&["ingest", "branch-a", "branch-b", "join", "lint"]);
        let edges = vec![
            edge("ingest", "branch-a"),
            edge("ingest", "branch-b"),
            edge("branch-a", "join"),
            edge("branch-b", "join"),
            edge("join", "lint"),
        ];
        let g = resolve_execution_graph(&pipe, Some(&edges)).unwrap();
        assert_eq!(g.layers[0], vec!["ingest"]);
        assert_eq!(
            g.layers[1].iter().cloned().collect::<BTreeSet<_>>(),
            BTreeSet::from(["branch-a".to_string(), "branch-b".to_string()])
        );
        assert_eq!(g.layers[2], vec!["join"]);
        assert_eq!(g.layers[3], vec!["lint"]);
    }

    #[test]
    fn rejects_cycle_without_loop_metadata() {
        let pipe = pipeline(&["a", "b", "c"]);
        let edges = vec![edge("a", "b"), edge("b", "c"), edge("c", "a")];
        let err = resolve_execution_graph(&pipe, Some(&edges)).unwrap_err();
        assert!(err.to_string().contains("requires `when`"));
    }

    #[test]
    fn rejects_loop_back_without_max_loops() {
        let pipe = pipeline(&["dev", "verify", "review"]);
        let edges = vec![
            edge("dev", "verify"),
            edge("verify", "review"),
            HordeEdge {
                from: "verify".into(),
                to: "dev".into(),
                when: Some("fail".into()),
                max_loops: None,
            },
        ];
        let err = resolve_execution_graph(&pipe, Some(&edges)).unwrap_err();
        assert!(err.to_string().contains("requires `max_loops`"));
    }

    #[test]
    fn rejects_unknown_endpoint() {
        let pipe = pipeline(&["a", "b"]);
        let edges = vec![edge("a", "missing")];
        let err = resolve_execution_graph(&pipe, Some(&edges)).unwrap_err();
        assert!(err.to_string().contains("not in pipeline"));
    }

    #[test]
    fn rejects_loop_back_without_when() {
        let pipe = pipeline(&["b", "a"]);
        let edges = vec![edge("a", "b")];
        let err = resolve_execution_graph(&pipe, Some(&edges)).unwrap_err();
        assert!(err.to_string().contains("requires `when`"));
    }

    #[test]
    fn rejects_dangling_step() {
        let pipe = pipeline(&["a", "b", "c"]);
        let edges = vec![edge("a", "b")];
        let err = resolve_execution_graph(&pipe, Some(&edges)).unwrap_err();
        assert!(err.to_string().contains("not connected"));
    }

    #[test]
    fn rejects_duplicate_edge() {
        let pipe = pipeline(&["a", "b", "c"]);
        let edges = vec![edge("a", "b"), edge("a", "b"), edge("b", "c")];
        let err = resolve_execution_graph(&pipe, Some(&edges)).unwrap_err();
        assert!(err.to_string().contains("duplicate edge"));
    }

    #[test]
    fn single_step_no_edges() {
        let pipe = pipeline(&["only"]);
        let g = resolve_execution_graph(&pipe, None).unwrap();
        assert!(g.edges.is_empty());
        assert_eq!(g.layers, vec![vec!["only".to_string()]]);
    }

    #[test]
    fn next_ready_step_fork_join() {
        let pipe = pipeline(&["ingest", "branch-a", "branch-b", "join", "lint"]);
        let edges = vec![
            edge("ingest", "branch-a"),
            edge("ingest", "branch-b"),
            edge("branch-a", "join"),
            edge("branch-b", "join"),
            edge("join", "lint"),
        ];
        let g = resolve_execution_graph(&pipe, Some(&edges)).unwrap();
        let mut status = BTreeMap::new();
        for s in &pipe {
            status.insert(s.clone(), "pending");
        }
        assert_eq!(
            next_ready_step(&pipe, &g, &status),
            Some("ingest".into())
        );
        status.insert("ingest".into(), "success");
        assert_eq!(
            next_ready_step(&pipe, &g, &status),
            Some("branch-a".into())
        );
        status.insert("branch-a".into(), "success");
        assert_eq!(
            next_ready_step(&pipe, &g, &status),
            Some("branch-b".into())
        );
        status.insert("branch-b".into(), "success");
        assert_eq!(next_ready_step(&pipe, &g, &status), Some("join".into()));
        status.insert("join".into(), "success");
        assert_eq!(next_ready_step(&pipe, &g, &status), Some("lint".into()));
        status.insert("lint".into(), "success");
        assert!(next_ready_step(&pipe, &g, &status).is_none());
        assert!(all_steps_successful(&pipe, &status));
    }

    fn loop_edge(from: &str, to: &str, when: &str, max_loops: u32) -> HordeEdge {
        HordeEdge {
            from: from.into(),
            to: to.into(),
            when: Some(when.into()),
            max_loops: Some(max_loops),
        }
    }

    #[test]
    fn select_next_on_verify_fail() {
        let pipe = pipeline(&["dev-1", "test-verify", "review"]);
        let edges = vec![
            edge("dev-1", "test-verify"),
            loop_edge("test-verify", "review", "pass", 1),
            loop_edge("test-verify", "dev-1", "fail", 2),
        ];
        let g = resolve_execution_graph(&pipe, Some(&edges)).unwrap();
        let counts = BTreeMap::new();
        assert_eq!(
            select_next_from_outcome(
                &pipe,
                &g.edges,
                "test-verify",
                StageStatus::Fail,
                &counts
            ),
            Some("dev-1".into())
        );
        assert_eq!(
            select_next_from_outcome(
                &pipe,
                &g.edges,
                "test-verify",
                StageStatus::Pass,
                &counts
            ),
            Some("review".into())
        );
    }

    #[test]
    fn next_ready_step_conditional_after_fail_retry() {
        let pipe = pipeline(&["dev-1", "test-verify", "review"]);
        let edges = vec![
            edge("dev-1", "test-verify"),
            loop_edge("test-verify", "review", "pass", 1),
            loop_edge("test-verify", "dev-1", "fail", 2),
        ];
        let g = resolve_execution_graph(&pipe, Some(&edges)).unwrap();
        let status = BTreeMap::from([
            ("dev-1".into(), "pending"),
            ("test-verify".into(), "pending"),
            ("review".into(), "pending"),
        ]);
        let outcomes = BTreeMap::new();
        assert_eq!(
            next_ready_step_conditional(&pipe, &g, &status, &outcomes),
            Some("dev-1".into())
        );
    }
}
