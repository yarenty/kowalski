/** Base for API calls. In dev, leave empty so Vite proxies `/api` to `kowalski` (see vite.config.ts). */
const base = (import.meta.env.VITE_API_BASE as string | undefined) ?? "";

/** localStorage key for the server API token (server generates it at `db/api_token`). */
const TOKEN_KEY = "kowalski.api_token";

/** Current API token: saved value first, then dev-time `VITE_API_TOKEN` injection. */
export function getApiToken(): string {
  return (
    localStorage.getItem(TOKEN_KEY) ??
    ((import.meta.env.VITE_API_TOKEN as string | undefined) ?? "")
  );
}

/** Persist (or clear, with "") the API token used on every request. */
export function setApiToken(token: string): void {
  if (token) localStorage.setItem(TOKEN_KEY, token);
  else localStorage.removeItem(TOKEN_KEY);
}

function authHeaders(): Record<string, string> {
  const token = getApiToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function json<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${base}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...authHeaders(),
      ...(init?.headers ?? {}),
    },
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${res.status} ${res.statusText}: ${text.slice(0, 200)}`);
  }
  return res.json() as Promise<T>;
}

export type Health = {
  status: string;
  service: string;
  version: string;
  model?: string;
  federation?: {
    agents_registered: number;
    postgres_notify_bridge: boolean;
  };
};

export type Doctor = {
  server_version: string;
  ollama: { url: string; ok: boolean; detail: string };
  llm: {
    provider: string;
    model: string;
    openai_api_base: string | null;
  };
};

export type McpServer = {
  name: string;
  url: string;
  transport: string;
};

export type McpPingResult = {
  name: string;
  url: string;
  transport: string;
  ok: boolean;
  tool_count?: number;
  error?: string;
};

export type ChatResponse = {
  reply: string;
  mode: string;
  model: string;
  memory_used: boolean;
  memory_source: string;
  memory_items_count: number;
};
export type MemoryStatus = {
  backend: string;
  episodic_buffer_count: number;
  embeddings_ok: boolean;
  embed_model: string;
  last_embed_error?: string | null;
};

export type ChatResetResponse = {
  conversation_id: string;
  model: string;
};

export type ChatMessage = {
  role: string;
  content: string;
  tool_calls?: unknown[] | null;
};

export type ChatMessagesResponse = {
  conversation_id: string;
  model: string;
  messages: ChatMessage[];
};

export type ChatSyncResponse = {
  conversation_id: string;
  model: string;
  message_count: number;
};

export type AgentsResponse = {
  mode: string;
  agents: { name: string; description: string }[];
  conversation_id: string;
  model: string;
};

export type SessionsResponse = {
  mode: string;
  sessions: { id: string; model: string; agent_name: string }[];
};

/** One SSE `data:` JSON line from `POST /api/chat/stream`. */
export type ChatStreamEvent =
  | {
      type: "start";
      conversation_id: string;
      model: string;
      memory_used?: boolean;
      memory_source?: string;
      memory_items_count?: number;
    }
  | { type: "token"; content: string }
  | { type: "assistant"; content: string }
  | { type: "error"; message: string }
  | { type: "done" };

export type FederationDelegateResponse = { delegated_to: string | null };

export type FederationRegistryResponse = {
  agents: { id: string; capabilities: string[]; state?: unknown }[];
};

export type FederationWorkerProfile = {
  id: string;
  horde_id?: string;
  horde_name?: string;
  step?: string;
  name: string;
  description: string;
  capability: string;
  agent_id: string;
  command: string;
  args: string[];
  cwd: string;
  managed_running: boolean;
  pid?: number | null;
  last_exit?: string | null;
  registered_exact?: boolean;
  stale_registration?: boolean;
  registry_agents: string[];
};

export type FederationWorkersResponse = {
  profiles: FederationWorkerProfile[];
};

export type HordeSubAgent = {
  name: string;
  kind: string;
  capability: string;
  default_agent_id: string;
  display_name: string;
  description: string;
  output?: string | null;
  avatar?: string | null;
};

export type OperatorInputField = {
  id: string;
  type: string;
  label: string;
  required?: boolean;
  placeholder?: string | null;
  options?: string[];
  default?: string | null;
};

export type HordeRunFormSpec = {
  step: string;
  display_name?: string | null;
  inputs: OperatorInputField[];
};

export type HordeEdge = { from: string; to: string };

export type HordeCatalogItem = {
  id: string;
  display_name: string;
  description: string;
  capability_prefix: string;
  pipeline: string[];
  edges?: HordeEdge[];
  default_question: string;
  topic: string;
  root_path: string;
  workdir?: string;
  config_on_startup?: boolean;
  config_on_startup_effective?: boolean;
  delivery_title?: string;
  delivery_note?: string;
  delivery_root_rel?: string;
  delivery_summary_note?: string;
  prompt_tip?: string;
  sub_agents: HordeSubAgent[];
  run_form?: HordeRunFormSpec | null;
};

export type HordeCatalogResponse = {
  hordes: HordeCatalogItem[];
};

export type HordeWorkersResponse = {
  horde_id: string;
  workers: FederationWorkerProfile[];
};

export type HordeRunStepRecord = {
  step: string;
  agent_id: string;
  task_id: string;
  status: string;
  artifact?: string | null;
  summary?: string | null;
  outcome?: string | null;
};

export type HordeRunRecord = {
  run_id: string;
  horde_id: string;
  prompt: string;
  source?: string | null;
  question: string;
  status: string;
  steps: HordeRunStepRecord[];
  events: Array<Record<string, unknown>>;
  loop_counts?: Record<string, number>;
  origin?: string;
  resume_count?: number;
  /** Incomplete in the store with no live orchestrator task (interrupted by a restart or awaiting input). */
  resumable?: boolean;
};

export type OpenPathResponse = {
  ok: boolean;
  path: string;
};

export const api = {
  health: () => json<Health>("/api/health"),
  agents: () => json<AgentsResponse>("/api/agents"),
  sessions: () => json<SessionsResponse>("/api/sessions"),
  doctor: () => json<Doctor>("/api/doctor"),
  models: () =>
    json<{ default_model: string; models: string[] }>("/api/models"),
  mcpServers: () => json<McpServer[]>("/api/mcp/servers"),
  mcpPing: () =>
    json<McpPingResult[]>("/api/mcp/ping", { method: "POST", body: "{}" }),
  memoryStatus: () => json<MemoryStatus>("/api/memory/status"),
  openPath: (path: string) =>
    json<OpenPathResponse>("/api/system/open-path", {
      method: "POST",
      body: JSON.stringify({ path }),
    }),
  chat: (
    message: string,
    options?: { useMemory?: boolean; conversationId?: string | null },
  ) =>
    json<ChatResponse>("/api/chat", {
      method: "POST",
      body: JSON.stringify({
        message,
        ...(options?.useMemory !== undefined ? { use_memory: options.useMemory } : {}),
        ...(options?.conversationId ? { conversation_id: options.conversationId } : {}),
      }),
    }),
  chatReset: () =>
    json<ChatResetResponse>("/api/chat/reset", {
      method: "POST",
      body: "{}",
    }),
  chatSync: (messages: ChatMessage[], conversationId?: string | null) =>
    json<ChatSyncResponse>("/api/chat/sync", {
      method: "POST",
      body: JSON.stringify({
        ...(conversationId ? { conversation_id: conversationId } : {}),
        messages,
      }),
    }),
  chatMessages: (conversationId?: string | null) =>
    json<ChatMessagesResponse>(
      conversationId
        ? `/api/chat/messages?conversation_id=${encodeURIComponent(conversationId)}`
        : "/api/chat/messages",
    ),
  federationRegistry: () => json<FederationRegistryResponse>("/api/federation/registry"),
  federationWorkers: () => json<FederationWorkersResponse>("/api/federation/workers"),
  federationWorkerStart: (profile_id: string) =>
    json<{ ok: boolean; profile_id: string; already_running: boolean; pid?: number | null }>(
      "/api/federation/workers/start",
      {
        method: "POST",
        body: JSON.stringify({ profile_id }),
      },
    ),
  federationWorkerStop: (profile_id: string) =>
    json<{ ok: boolean; profile_id: string; pid?: number | null }>("/api/federation/workers/stop", {
      method: "POST",
      body: JSON.stringify({ profile_id }),
    }),
  hordes: () => json<HordeCatalogResponse>("/api/hordes"),
  horde: (hordeId: string) => json<HordeCatalogItem>(`/api/hordes/${encodeURIComponent(hordeId)}`),
  hordeWorkers: (hordeId: string) =>
    json<HordeWorkersResponse>(`/api/hordes/${encodeURIComponent(hordeId)}/workers`),
  hordeWorkersStart: (hordeId: string, step?: string) =>
    json<{ ok: boolean; started: unknown[] }>(`/api/hordes/${encodeURIComponent(hordeId)}/workers/start`, {
      method: "POST",
      body: JSON.stringify(step ? { step } : {}),
    }),
  hordeWorkersStop: (hordeId: string, step?: string) =>
    json<{ ok: boolean; stopped: unknown[] }>(`/api/hordes/${encodeURIComponent(hordeId)}/workers/stop`, {
      method: "POST",
      body: JSON.stringify(step ? { step } : {}),
    }),
  hordeRepairOutputs: (hordeId: string) =>
    json<{ ok: boolean; horde_id: string; files_fixed: number }>(
      `/api/hordes/${encodeURIComponent(hordeId)}/repair-outputs`,
      { method: "POST", body: "{}" },
    ),
  hordeRun: (
    hordeId: string,
    body: {
      prompt?: string;
      source?: string;
      question?: string;
      form_answers?: Record<string, string>;
    },
  ) =>
    json<{ ok: boolean; run: HordeRunRecord }>(`/api/hordes/${encodeURIComponent(hordeId)}/run`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  hordeCleanWorkdir: (hordeId: string) =>
    json<{ ok: boolean; horde_id: string; workdir: string }>(
      `/api/hordes/${encodeURIComponent(hordeId)}/clean-workdir`,
      { method: "POST", body: "{}" },
    ),
  hordeRuns: (hordeId: string) =>
    json<{ horde_id: string; runs: HordeRunRecord[] }>(`/api/hordes/${encodeURIComponent(hordeId)}/runs`),
  hordeRunResume: (hordeId: string, runId: string) =>
    json<{ ok: boolean; run: HordeRunRecord }>(
      `/api/hordes/${encodeURIComponent(hordeId)}/runs/${encodeURIComponent(runId)}/resume`,
      { method: "POST", body: "{}" },
    ),
  hordeRunCancel: (hordeId: string, runId: string) =>
    json<{ ok: boolean; run: HordeRunRecord }>(
      `/api/hordes/${encodeURIComponent(hordeId)}/runs/${encodeURIComponent(runId)}/cancel`,
      { method: "POST", body: "{}" },
    ),
  hordeRunDetail: (hordeId: string, runId: string) =>
    json<{ run: HordeRunRecord }>(
      `/api/hordes/${encodeURIComponent(hordeId)}/runs/${encodeURIComponent(runId)}`,
    ),
  hordeFollowup: (hordeId: string, body: { run_id: string; message: string }) =>
    json<{
      ok: boolean;
      horde_id: string;
      run_id: string;
      reply: string;
      output_path?: string;
      mode: string;
      decision?: { strategy?: string; selected_step?: string; reason?: string };
      rerun_id?: string;
    }>(
      `/api/hordes/${encodeURIComponent(hordeId)}/followup`,
      {
        method: "POST",
        body: JSON.stringify(body),
      },
    ),
  graphStatus: () => json<Record<string, unknown>>("/api/graph/status"),
  federationDelegate: (body: {
    task_id: string;
    instruction: string;
    capability: string;
  }) =>
    json<FederationDelegateResponse>("/api/federation/delegate", {
      method: "POST",
      body: JSON.stringify(body),
    }),
  federationRegister: (body: { id: string; capabilities: string[] }) =>
    json<{ ok: boolean; id: string }>("/api/federation/register", {
      method: "POST",
      body: JSON.stringify(body),
    }),
  federationDeregister: (agent_id: string) =>
    json<{ ok: boolean; agent_id: string }>("/api/federation/deregister", {
      method: "POST",
      body: JSON.stringify({ agent_id }),
    }),
  federationCleanupStale: (stale_after_secs: number) =>
    json<{ ok: boolean; rows_updated: number }>("/api/federation/cleanup-stale", {
      method: "POST",
      body: JSON.stringify({ stale_after_secs }),
    }),
  rookeryCreateSession: (body?: {
    history?: { role: "user" | "assistant"; content: string }[];
    draft?: RookeryDraft | null;
    summary?: string | null;
    status?: RookerySessionStatus;
  }) =>
    json<RookeryCreateSessionResponse>("/api/rookery/sessions", {
      method: "POST",
      body: JSON.stringify(body ?? {}),
    }),
  rookerySession: (sessionId: string) =>
    json<RookerySessionResponse>(
      `/api/rookery/sessions/${encodeURIComponent(sessionId)}`,
    ),
  rookeryDeleteSession: (sessionId: string) =>
    json<{ ok: boolean; session_id: string }>(
      `/api/rookery/sessions/${encodeURIComponent(sessionId)}`,
      { method: "DELETE" },
    ),
  rookeryChat: (sessionId: string, message: string) =>
    json<RookeryChatResponse>(
      `/api/rookery/sessions/${encodeURIComponent(sessionId)}/chat`,
      {
        method: "POST",
        body: JSON.stringify({ message, stream: false }),
      },
    ),
  rookeryPropose: (sessionId: string) =>
    json<RookeryProposeResponse>(
      `/api/rookery/sessions/${encodeURIComponent(sessionId)}/propose`,
      { method: "POST", body: "{}" },
    ),
  rookeryGiveBirth: (
    sessionId: string,
    body?: { output_root?: string; overwrite?: boolean },
  ) =>
    json<RookeryGiveBirthResponse>(
      `/api/rookery/sessions/${encodeURIComponent(sessionId)}/give-birth`,
      {
        method: "POST",
        body: JSON.stringify(body ?? {}),
      },
    ),
  rookeryPatchPenguin: (
    sessionId: string,
    penguinName: string,
    body: {
      kind?: string;
      display_name?: string;
      description?: string;
      prompt_body?: string;
      agent_body?: string;
      clear_agent_body?: boolean;
      output?: string;
      context_paths?: string[];
      tool_ids?: string[];
      model_id?: string;
      clear_model_id?: boolean;
      avatar?: string;
      clear_avatar?: boolean;
    },
  ) =>
    json<{ session: RookerySessionResponse }>(
      `/api/rookery/sessions/${encodeURIComponent(sessionId)}/penguins/${encodeURIComponent(penguinName)}`,
      { method: "PATCH", body: JSON.stringify(body) },
    ),
  rookerySaveHorde: (sessionId: string) =>
    json<RookerySaveHordeResponse>(
      `/api/rookery/sessions/${encodeURIComponent(sessionId)}/save-horde`,
      { method: "POST", body: "{}" },
    ),
  rookeryValidateDraft: (sessionId: string) =>
    json<{ ok: boolean; errors: string | null; session: RookerySessionResponse }>(
      `/api/rookery/sessions/${encodeURIComponent(sessionId)}/validate`,
      { method: "POST", body: "{}" },
    ),
};

export type RookeryPenguinSpec = {
  name: string;
  kind: string;
  display_name: string;
  description: string;
  prompt_body: string;
  agent_body?: string | null;
  output: string;
  context_paths?: string[];
  tool_ids?: string[];
  model_id?: string | null;
  avatar?: string | null;
};

export type RookeryDraft = {
  id: string;
  display_name: string;
  description: string;
  capability_prefix?: string | null;
  pipeline: string[];
  edges?: HordeEdge[];
  penguins: RookeryPenguinSpec[];
  default_question?: string | null;
  default_topic?: string | null;
  workdir?: string | null;
  delivery_title?: string | null;
  delivery_note?: string | null;
  delivery_root_rel?: string | null;
  delivery_summary_note?: string | null;
  prompt_tip?: string | null;
};

export type RookerySessionStatus = "interviewing" | "proposed" | "born";

export type RookerySessionResponse = {
  session_id: string;
  conversation_id: string;
  status: RookerySessionStatus;
  draft: RookeryDraft | null;
  summary: string | null;
  pipeline: string[];
  horde_root: string | null;
  output_root: string;
  created_at_ms: number;
  updated_at_ms: number;
};

export type RookeryCreateSessionResponse = {
  session: RookerySessionResponse;
};

export type RookeryChatResponse = {
  reply: string;
  session: RookerySessionResponse;
};

export type RookeryProposeResponse = {
  session: RookerySessionResponse;
  parse_error: string | null;
};

export type RookeryGiveBirthResponse = {
  ok: boolean;
  horde_root: string;
  horde_id: string;
  validate_ok: boolean;
  validate_errors: string | null;
  session: RookerySessionResponse;
};

export type RookerySaveHordeResponse = {
  ok: boolean;
  horde_root: string;
  horde_id: string;
  validate_ok: boolean;
  validate_errors: string | null;
  session: RookerySessionResponse;
};

export type RookeryStreamEvent =
  | { type: "start"; session_id: string; model: string }
  | { type: "token"; content: string }
  | { type: "assistant"; content: string }
  | { type: "error"; message: string }
  | { type: "done" };

/** `EventSource` for `GET /api/federation/stream` — caller must `close()` when done. */
export function openFederationEventSource(
  topic: string,
  onMessage: (data: string) => void,
  onError?: () => void,
): EventSource {
  // EventSource cannot set headers — the server accepts `?token=` for SSE.
  const token = getApiToken();
  const tokenPart = token ? `&token=${encodeURIComponent(token)}` : "";
  const url = `${base}/api/federation/stream?topic=${encodeURIComponent(topic)}${tokenPart}`;
  const es = new EventSource(url);
  es.onmessage = (ev) => onMessage(ev.data);
  es.onerror = () => onError?.();
  return es;
}

/**
 * Pump a `text/event-stream` response body, invoking `onEvent` with the parsed JSON of each
 * `data:` line. Shared by all request-scoped chat streams (`/api/chat/stream`,
 * `/api/rookery/.../chat`). Non-JSON keepalive lines are ignored.
 */
async function streamSse<T>(res: Response, onEvent: (ev: T) => void): Promise<void> {
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${res.status} ${res.statusText}: ${text.slice(0, 200)}`);
  }
  const reader = res.body?.getReader();
  if (!reader) throw new Error("No response body");
  const dec = new TextDecoder();
  let buf = "";
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += dec.decode(value, { stream: true });
    let idx: number;
    while ((idx = buf.indexOf("\n\n")) >= 0) {
      const block = buf.slice(0, idx);
      buf = buf.slice(idx + 2);
      for (const line of block.split("\n")) {
        const m = line.match(/^data:\s*(.*)$/);
        if (!m) continue;
        const raw = m[1]?.trim();
        if (!raw) continue;
        try {
          onEvent(JSON.parse(raw) as T);
        } catch {
          /* ignore non-JSON keepalives */
        }
      }
    }
  }
}

/** SSE from `POST /api/chat/stream`: one JSON `ChatStreamEvent` per `data:` line. */
export async function chatStream(
  message: string,
  onEvent: (ev: ChatStreamEvent) => void,
  options?: { toolsStream?: boolean; useMemory?: boolean; conversationId?: string | null },
): Promise<void> {
  const res = await fetch(`${base}/api/chat/stream`, {
    method: "POST",
    headers: { "Content-Type": "application/json", ...authHeaders() },
    body: JSON.stringify({
      message,
      ...(options?.toolsStream ? { tools_stream: true } : {}),
      ...(options?.useMemory !== undefined ? { use_memory: options.useMemory } : {}),
      ...(options?.conversationId ? { conversation_id: options.conversationId } : {}),
    }),
  });
  await streamSse<ChatStreamEvent>(res, onEvent);
}

/** SSE from `POST /api/rookery/sessions/{id}/chat` with `{ stream: true }`. */
export async function rookeryChatStream(
  sessionId: string,
  message: string,
  onEvent: (ev: RookeryStreamEvent) => void,
): Promise<void> {
  const res = await fetch(
    `${base}/api/rookery/sessions/${encodeURIComponent(sessionId)}/chat`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json", ...authHeaders() },
      body: JSON.stringify({ message, stream: true }),
    },
  );
  await streamSse<RookeryStreamEvent>(res, onEvent);
}
