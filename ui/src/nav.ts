/** Screen ids; also the `?tab=` deep-link values (unknown values are ignored). */
export const TAB_IDS = [
  "federation-run",
  "runs",
  "chat",
  "rookery",
  "setup",
  "federation-management",
  "mcp",
  "graph",
  "home",
  "about",
] as const;
export type TabId = (typeof TAB_IDS)[number];

export function isTabId(v: string | null | undefined): v is TabId {
  return !!v && (TAB_IDS as readonly string[]).includes(v);
}

/**
 * Where the operator is: a tab, plus (on the Hordes tab) an open horde page and optionally one
 * of its runs. Mirrored into the URL as `?tab=`, `?horde=` and `?run=`.
 */
export type Route = { tab: TabId; horde: string | null; run: string | null };

/**
 * The tableski sign-in result (`?setup=tableski-connected` and friends) the page was opened
 * with, captured once at load: the first navigation rewrites the URL and drops the parameter
 * before the Setup screen mounts. [`takeSetupOutcome`] hands it over once.
 */
let setupOutcome: string | null = new URLSearchParams(window.location.search).get("setup");

export function takeSetupOutcome(): string | null {
  const outcome = setupOutcome;
  setupOutcome = null;
  return outcome;
}

/** Whether the page was opened by a tableski sign-in redirect (not yet handed over). */
export function hasSetupOutcome(): boolean {
  return setupOutcome !== null;
}

export function routeFromUrl(search = window.location.search): Route {
  const q = new URLSearchParams(search);
  const tab = q.get("tab");
  const horde = q.get("horde") || null;
  const run = q.get("run") || null;
  return { tab: isTabId(tab) ? tab : "federation-run", horde, run: horde ? run : null };
}

/** The URL for a route; other query params (e.g. `?theme=`) are kept. */
export function urlForRoute(r: Route, search = window.location.search): string {
  const q = new URLSearchParams(search);
  for (const k of ["tab", "horde", "run", "setup"]) q.delete(k);
  if (r.tab !== "federation-run") q.set("tab", r.tab);
  if (r.tab === "federation-run" && r.horde) {
    q.set("horde", r.horde);
    if (r.run) q.set("run", r.run);
  }
  const qs = q.toString();
  return `${window.location.pathname}${qs ? `?${qs}` : ""}${window.location.hash}`;
}
