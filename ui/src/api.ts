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
};

export type Doctor = {
  cli_version: string;
  ollama: { url: string; ok: boolean; detail: string };
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

export const api = {
  health: () => json<Health>("/api/health"),
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
};
