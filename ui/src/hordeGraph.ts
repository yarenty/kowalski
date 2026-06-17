/** Client-side horde DAG layout (mirrors kowalski-core `horde_graph` scheduling layers). */

import type { HordeEdge } from "./api";

export type { HordeEdge };

function implicitChain(pipeline: string[]): HordeEdge[] {
  const out: HordeEdge[] = [];
  for (let i = 0; i + 1 < pipeline.length; i++) {
    out.push({ from: pipeline[i], to: pipeline[i + 1] });
  }
  return out;
}

function effectiveEdges(pipeline: string[], edges: HordeEdge[]): HordeEdge[] {
  return edges.length > 0 ? edges : implicitChain(pipeline);
}

/** True when the operator declared explicit scheduling edges (fork/join). */
export function isDagHorde(_pipeline: string[], edges: HordeEdge[]): boolean {
  return edges.length > 0;
}

/** Topological layers for canvas layout (pipeline order within each layer). */
export function computeLayers(pipeline: string[], edges: HordeEdge[]): string[][] {
  if (pipeline.length === 0) return [];
  const eff = effectiveEdges(pipeline, edges);
  const inDegree = new Map<string, number>();
  const adj = new Map<string, string[]>();
  const index = new Map<string, number>();
  pipeline.forEach((name, i) => {
    inDegree.set(name, 0);
    adj.set(name, []);
    index.set(name, i);
  });
  for (const e of eff) {
    inDegree.set(e.to, (inDegree.get(e.to) ?? 0) + 1);
    adj.get(e.from)?.push(e.to);
  }
  const remaining = new Map(inDegree);
  const layers: string[][] = [];
  let processed = 0;
  while (processed < pipeline.length) {
    const ready = pipeline.filter((name) => remaining.get(name) === 0);
    if (ready.length === 0) break;
    for (const node of ready) {
      for (const succ of adj.get(node) ?? []) {
        remaining.set(succ, (remaining.get(succ) ?? 0) - 1);
      }
      remaining.delete(node);
    }
    processed += ready.length;
    layers.push(ready);
  }
  return layers;
}
