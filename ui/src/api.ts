/** Base for API calls. In dev, leave empty so Vite proxies `/api` to the CLI (see vite.config.ts). */
const base = (import.meta.env.VITE_API_BASE as string | undefined) ?? "";

async function json<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${base}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
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
  cli_version: string;
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

export type ChatResponse = { reply: string; mode: string; model: string };

export type ChatResetResponse = {
  conversation_id: string;
  model: string;
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
  | { type: "start"; conversation_id: string; model: string }
  | { type: "token"; content: string }
  | { type: "assistant"; content: string }
  | { type: "error"; message: string }
  | { type: "done" };

export type FederationDelegateResponse = { delegated_to: string | null };

export type FederationRegistryResponse = {
  agents: { id: string; capabilities: string[]; state?: unknown }[];
};

export const api = {
  health: () => json<Health>("/api/health"),
  agents: () => json<AgentsResponse>("/api/agents"),
  sessions: () => json<SessionsResponse>("/api/sessions"),
  doctor: () => json<Doctor>("/api/doctor"),
  mcpServers: () => json<McpServer[]>("/api/mcp/servers"),
  mcpPing: () =>
    json<McpPingResult[]>("/api/mcp/ping", { method: "POST", body: "{}" }),
  chat: (message: string) =>
    json<ChatResponse>("/api/chat", {
      method: "POST",
      body: JSON.stringify({ message }),
    }),
  chatReset: () =>
    json<ChatResetResponse>("/api/chat/reset", {
      method: "POST",
      body: "{}",
    }),
  federationRegistry: () => json<FederationRegistryResponse>("/api/federation/registry"),
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
};

/** `EventSource` for `GET /api/federation/stream` — caller must `close()` when done. */
export function openFederationEventSource(
  topic: string,
  onMessage: (data: string) => void,
  onError?: () => void,
): EventSource {
  const url = `${base}/api/federation/stream?topic=${encodeURIComponent(topic)}`;
  const es = new EventSource(url);
  es.onmessage = (ev) => onMessage(ev.data);
  es.onerror = () => onError?.();
  return es;
}

/** Parses `text/event-stream` body: one JSON object per `data:` line. */
export async function chatStream(
  message: string,
  onEvent: (ev: ChatStreamEvent) => void,
  options?: { toolsStream?: boolean },
): Promise<void> {
  const res = await fetch(`${base}/api/chat/stream`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      message,
      ...(options?.toolsStream ? { tools_stream: true } : {}),
    }),
  });
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
          onEvent(JSON.parse(raw) as ChatStreamEvent);
        } catch {
          /* ignore non-JSON keepalives */
        }
      }
    }
  }
}
