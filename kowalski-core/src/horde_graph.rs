//! Horde execution graph: optional `[[edges]]` on manifests / Rookery drafts.
//!
//! When `edges` is absent or empty, the graph is an implicit linear chain following
//! `pipeline` order. Scheduling layers are derived via Kahn topological sort.

use crate::error::KowalskiError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// One scheduling dependency: `from` must complete before `to` runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HordeEdge {
    pub from: String,
    pub to: String,
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
    validate_acyclic(pipeline, &effective)?;
    validate_pipeline_topological_order(pipeline, &effective)?;

    let layers = compute_layers(pipeline, &effective)?;

    Ok(ExecutionGraph {
        edges: effective,
        layers,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(from: &str, to: &str) -> HordeEdge {
        HordeEdge {
            from: from.into(),
            to: to.into(),
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
    fn rejects_cycle() {
        let pipe = pipeline(&["a", "b", "c"]);
        let edges = vec![edge("a", "b"), edge("b", "c"), edge("c", "a")];
        let err = resolve_execution_graph(&pipe, Some(&edges)).unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }

    #[test]
    fn rejects_unknown_endpoint() {
        let pipe = pipeline(&["a", "b"]);
        let edges = vec![edge("a", "missing")];
        let err = resolve_execution_graph(&pipe, Some(&edges)).unwrap_err();
        assert!(err.to_string().contains("not in pipeline"));
    }

    #[test]
    fn rejects_pipeline_order_violation() {
        let pipe = pipeline(&["b", "a"]);
        let edges = vec![edge("a", "b")];
        let err = resolve_execution_graph(&pipe, Some(&edges)).unwrap_err();
        assert!(err.to_string().contains("pipeline order violates"));
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
}
