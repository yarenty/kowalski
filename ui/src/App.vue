<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  api,
  chatStream,
  openFederationEventSource,
  type AgentsResponse,
  type Doctor,
  type FederationRegistryResponse,
  type Health,
  type McpPingResult,
  type McpServer,
  type MemoryStatus,
  type SessionsResponse,
} from "./api";

const tab = ref<"home" | "mcp" | "chat" | "federation" | "graph">("chat");

const health = ref<Health | null>(null);
const healthErr = ref<string | null>(null);

const agents = ref<AgentsResponse | null>(null);
const agentsErr = ref<string | null>(null);

const sessions = ref<SessionsResponse | null>(null);
const sessionsErr = ref<string | null>(null);

const doctor = ref<Doctor | null>(null);
const doctorErr = ref<string | null>(null);
const memoryStatus = ref<MemoryStatus | null>(null);
const memoryErr = ref<string | null>(null);

const servers = ref<McpServer[]>([]);
const serversErr = ref<string | null>(null);

const pingBusy = ref(false);
const pingResults = ref<McpPingResult[] | null>(null);
const pingErr = ref<string | null>(null);

const chatIn = ref("");
const chatOut = ref<string | null>(null);
const chatMeta = ref<string | null>(null);
const sessionId = ref<string | null>(null);
const chatBusy = ref(false);
const resetBusy = ref(false);
const chatErr = ref<string | null>(null);
/** When true, SSE uses `tools_stream`: stream tokens only for the LLM turn after tool execution(s). */
const chatToolsStream = ref(false);
type ChatTurn = { role: "user" | "assistant"; content: string };
const chatTurns = ref<ChatTurn[]>([]);
const CHAT_STORAGE_KEY = "kowalski.ui.chat.v1";

const fedTopic = ref("federation");
const fedTaskId = ref("demo-1");
const fedInstruction = ref("Smoke test");
const fedCap = ref("chat");
const fedResult = ref<string | null>(null);
const fedErr = ref<string | null>(null);
const fedBusy = ref(false);
const fedCleanupBusy = ref(false);
const fedStaleSecs = ref(300);
const fedDeregisterId = ref("");
const fedRegId = ref("");
const fedRegCaps = ref("search, chat");
const fedLines = ref<string[]>([]);
const fedEs = ref<EventSource | null>(null);
const fedRegistry = ref<FederationRegistryResponse | null>(null);
const fedRegistryErr = ref<string | null>(null);

const graphStatus = ref<Record<string, unknown> | null>(null);
const graphErr = ref<string | null>(null);
const federationAgents = computed(() => fedRegistry.value?.agents ?? []);

function statusLabel(ok: boolean): string {
  return ok ? "OK" : "ERROR";
}

function statusClass(ok: boolean): string {
  return ok ? "status-ok" : "status-error";
}

function persistChatState() {
  const payload = {
    turns: chatTurns.value.slice(-100),
    sessionId: sessionId.value,
    chatMeta: chatMeta.value,
  };
  localStorage.setItem(CHAT_STORAGE_KEY, JSON.stringify(payload));
}

function restoreChatState() {
  const raw = localStorage.getItem(CHAT_STORAGE_KEY);
  if (!raw) return;
  try {
    const parsed = JSON.parse(raw) as {
      turns?: ChatTurn[];
      sessionId?: string | null;
      chatMeta?: string | null;
    };
    if (Array.isArray(parsed.turns)) {
      chatTurns.value = parsed.turns.filter(
        (t) =>
          (t.role === "user" || t.role === "assistant") &&
          typeof t.content === "string",
      );
    }
    sessionId.value = parsed.sessionId ?? null;
    chatMeta.value = parsed.chatMeta ?? null;
  } catch {
    /* ignore invalid localStorage payload */
  }
}

async function loadGraphStatus() {
  graphErr.value = null;
  try {
    graphStatus.value = await api.graphStatus();
  } catch (e) {
    graphStatus.value = null;
    graphErr.value = e instanceof Error ? e.message : String(e);
  }
}

async function loadFederationRegistry() {
  fedRegistryErr.value = null;
  try {
    fedRegistry.value = await api.federationRegistry();
  } catch (e) {
    fedRegistry.value = null;
    fedRegistryErr.value = e instanceof Error ? e.message : String(e);
  }
}

function fedDisconnect() {
  fedEs.value?.close();
  fedEs.value = null;
}

function fedConnect() {
  fedDisconnect();
  fedLines.value = [];
  const es = openFederationEventSource(
    fedTopic.value,
    (data) => {
      try {
        const j = JSON.parse(data) as unknown;
        fedLines.value = [
          ...fedLines.value.slice(-99),
          JSON.stringify(j, null, 2),
        ];
      } catch {
        fedLines.value = [...fedLines.value.slice(-99), data];
      }
    },
    () => {},
  );
  fedEs.value = es;
}

async function fedDelegate() {
  fedBusy.value = true;
  fedErr.value = null;
  fedResult.value = null;
  try {
    const r = await api.federationDelegate({
      task_id: fedTaskId.value.trim() || "task",
      instruction: fedInstruction.value.trim() || "…",
      capability: fedCap.value.trim() || "chat",
    });
    fedResult.value = JSON.stringify(r, null, 2);
    void loadFederationRegistry();
  } catch (e) {
    fedErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    fedBusy.value = false;
  }
}

async function fedCleanupStale() {
  fedCleanupBusy.value = true;
  fedErr.value = null;
  try {
    const r = await api.federationCleanupStale(
      Math.max(30, Math.floor(fedStaleSecs.value)),
    );
    fedResult.value = JSON.stringify(r, null, 2);
    void loadFederationRegistry();
  } catch (e) {
    fedErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    fedCleanupBusy.value = false;
  }
}

async function fedDeregister() {
  const id = fedDeregisterId.value.trim();
  if (!id) return;
  fedBusy.value = true;
  fedErr.value = null;
  try {
    const r = await api.federationDeregister(id);
    fedResult.value = JSON.stringify(r, null, 2);
    fedDeregisterId.value = "";
    void loadFederationRegistry();
  } catch (e) {
    fedErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    fedBusy.value = false;
  }
}

async function fedRegister() {
  const id = fedRegId.value.trim();
  if (!id) return;
  const caps = fedRegCaps.value
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  fedBusy.value = true;
  fedErr.value = null;
  try {
    const r = await api.federationRegister({ id, capabilities: caps });
    fedResult.value = JSON.stringify(r, null, 2);
    void loadFederationRegistry();
  } catch (e) {
    fedErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    fedBusy.value = false;
  }
}

watch(tab, (t) => {
  if (t !== "federation") fedDisconnect();
  else void loadFederationRegistry();
});

async function loadHealth() {
  healthErr.value = null;
  try {
    health.value = await api.health();
  } catch (e) {
    health.value = null;
    healthErr.value = e instanceof Error ? e.message : String(e);
  }
}

async function loadAgents() {
  agentsErr.value = null;
  try {
    agents.value = await api.agents();
  } catch (e) {
    agents.value = null;
    agentsErr.value = e instanceof Error ? e.message : String(e);
  }
}

async function loadSessions() {
  sessionsErr.value = null;
  try {
    sessions.value = await api.sessions();
  } catch (e) {
    sessions.value = null;
    sessionsErr.value = e instanceof Error ? e.message : String(e);
  }
}

async function loadDoctor() {
  doctorErr.value = null;
  try {
    doctor.value = await api.doctor();
  } catch (e) {
    doctor.value = null;
    doctorErr.value = e instanceof Error ? e.message : String(e);
  }
}

async function loadMemoryStatus() {
  memoryErr.value = null;
  try {
    memoryStatus.value = await api.memoryStatus();
  } catch (e) {
    memoryStatus.value = null;
    memoryErr.value = e instanceof Error ? e.message : String(e);
  }
}

async function loadServers() {
  serversErr.value = null;
  try {
    servers.value = await api.mcpServers();
  } catch (e) {
    servers.value = [];
    serversErr.value = e instanceof Error ? e.message : String(e);
  }
}

async function runMcpPing() {
  pingBusy.value = true;
  pingErr.value = null;
  pingResults.value = null;
  try {
    pingResults.value = await api.mcpPing();
  } catch (e) {
    pingErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    pingBusy.value = false;
  }
}

async function sendChat() {
  const msg = chatIn.value.trim();
  if (!msg) return;
  chatTurns.value.push({ role: "user", content: msg });
  chatBusy.value = true;
  chatErr.value = null;
  chatOut.value = null;
  chatMeta.value = null;
  try {
    const r = await api.chat(msg);
    chatOut.value = r.reply;
    chatMeta.value = `${r.mode} · ${r.model}`;
    chatTurns.value.push({ role: "assistant", content: r.reply });
    chatIn.value = "";
    void loadSessions();
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    chatErr.value = message;
    chatTurns.value.push({ role: "assistant", content: `[error] ${message}` });
  } finally {
    chatBusy.value = false;
  }
}

async function sendChatStream() {
  const msg = chatIn.value.trim();
  if (!msg) return;
  chatTurns.value.push({ role: "user", content: msg });
  const assistantTurn: ChatTurn = { role: "assistant", content: "" };
  chatTurns.value.push(assistantTurn);
  chatBusy.value = true;
  chatErr.value = null;
  chatOut.value = "";
  chatMeta.value = null;
  try {
    await chatStream(msg, (ev) => {
      if (ev.type === "start") {
        sessionId.value = ev.conversation_id;
        chatMeta.value = chatToolsStream.value
          ? `SSE · tools_stream · ${ev.model}`
          : `SSE · ${ev.model}`;
      } else if (ev.type === "token") {
        chatOut.value = (chatOut.value ?? "") + ev.content;
        assistantTurn.content += ev.content;
      } else if (ev.type === "assistant") {
        chatOut.value = ev.content;
        assistantTurn.content = ev.content;
      } else if (ev.type === "error") {
        chatErr.value = ev.message;
        assistantTurn.content = `[error] ${ev.message}`;
      }
    }, { toolsStream: chatToolsStream.value });
    if (!assistantTurn.content.trim()) {
      assistantTurn.content = "(no assistant output)";
    }
    chatIn.value = "";
    void loadSessions();
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    chatErr.value = message;
    assistantTurn.content = `[error] ${message}`;
  } finally {
    chatBusy.value = false;
  }
}

async function resetChat() {
  resetBusy.value = true;
  chatErr.value = null;
  try {
    const r = await api.chatReset();
    sessionId.value = r.conversation_id;
    chatOut.value = null;
    chatMeta.value = `new session · ${r.model}`;
    chatIn.value = "";
    chatTurns.value = [];
    persistChatState();
    void loadSessions();
  } catch (e) {
    chatErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    resetBusy.value = false;
  }
}

onMounted(() => {
  restoreChatState();
  void loadHealth();
  void loadAgents();
  void loadSessions();
  void loadMemoryStatus();
});

onUnmounted(() => {
  fedDisconnect();
});

watch([chatTurns, sessionId, chatMeta], () => {
  persistChatState();
}, { deep: true });
</script>

<template>
  <div class="app">
    <header class="header">
      <h1>Kowalski</h1>
      <p class="tagline">Rust-native agents · MCP · optional Postgres</p>
      <nav class="nav">
        <button
          type="button"
          :class="{ active: tab === 'home' }"
          @click="tab = 'home'"
        >
          Home
        </button>
        <button
          type="button"
          :class="{ active: tab === 'mcp' }"
          @click="
            tab = 'mcp';
            loadServers();
          "
        >
          MCP
        </button>
        <button type="button" :class="{ active: tab === 'chat' }" @click="tab = 'chat'">
          Chat
        </button>
        <button
          type="button"
          :class="{ active: tab === 'federation' }"
          @click="tab = 'federation'"
        >
          Federation
        </button>
        <button type="button" :class="{ active: tab === 'graph' }" @click="tab = 'graph'">
          Graph
        </button>
      </nav>
    </header>

    <main class="main">
      <section v-if="tab === 'home'" class="panel">
        <h2>API status</h2>
        <p class="hint">
          Run
          <code>kowalski-cli serve</code>
          (default <code>127.0.0.1:3000</code>), then
          <code>bun run dev</code>
          in <code>ui/</code> — Vite proxies <code>/api</code> to the CLI.
        </p>
        <p>
          <button type="button" class="primary" @click="loadHealth">Refresh health</button>
          <button type="button" @click="loadAgents">Refresh agents</button>
          <button type="button" @click="loadSessions">Refresh sessions</button>
          <button type="button" @click="loadDoctor">Load doctor</button>
          <button type="button" @click="loadMemoryStatus">Load memory status</button>
        </p>
        <h3>Memory</h3>
        <article v-if="memoryStatus" class="card">
          <header>
            <strong>Embeddings</strong>
            <span
              class="status-badge"
              :class="statusClass(memoryStatus.embeddings_ok)"
            >{{ statusLabel(memoryStatus.embeddings_ok) }}</span>
          </header>
          <p class="muted">Backend: {{ memoryStatus.backend }}</p>
          <p class="muted">Episodic buffer count: {{ memoryStatus.episodic_buffer_count }}</p>
          <p class="muted">Embed model: {{ memoryStatus.embed_model }}</p>
          <p v-if="memoryStatus.last_embed_error" class="err">
            Last embed error: {{ memoryStatus.last_embed_error }}
          </p>
        </article>
        <p v-if="memoryErr" class="err">{{ memoryErr }}</p>
        <details>
          <summary>Raw health JSON</summary>
          <pre v-if="health" class="json json-scroll">{{ JSON.stringify(health, null, 2) }}</pre>
        </details>
        <p v-if="healthErr" class="err">{{ healthErr }}</p>
        <h3>Agents</h3>
        <details>
          <summary>Raw agents JSON</summary>
          <pre v-if="agents" class="json json-scroll">{{ JSON.stringify(agents, null, 2) }}</pre>
        </details>
        <p v-if="agentsErr" class="err">{{ agentsErr }}</p>
        <h3>Sessions</h3>
        <details>
          <summary>Raw sessions JSON</summary>
          <pre v-if="sessions" class="json json-scroll">{{ JSON.stringify(sessions, null, 2) }}</pre>
        </details>
        <p v-if="sessionsErr" class="err">{{ sessionsErr }}</p>
        <h3>Ollama probe</h3>
        <details>
          <summary>Raw doctor JSON</summary>
          <pre v-if="doctor" class="json json-scroll">{{ JSON.stringify(doctor, null, 2) }}</pre>
        </details>
        <p v-if="doctorErr" class="err">{{ doctorErr }}</p>
      </section>

      <section v-else-if="tab === 'graph'" class="panel">
        <h2>Graph</h2>
        <div class="guide">
          <h3>How to use</h3>
          <ol>
            <li>Build and run CLI with <code>--features postgres</code>.</li>
            <li>Set <code>memory.database_url</code> in <code>config.toml</code>.</li>
            <li>Click <strong>Load graph status</strong> to verify extensions.</li>
          </ol>
        </div>
        <p class="hint">
          <code>GET /api/graph/status</code> probes Postgres for <code>vector</code> and
          <code>age</code> extensions when <code>memory.database_url</code> is set and the CLI is built
          with <code>--features postgres</code>. Full Cypher / AGE integration is WP3.
        </p>
        <p>
          <button type="button" class="primary" @click="loadGraphStatus">Load graph status</button>
        </p>
        <details>
          <summary>Raw graph JSON</summary>
          <pre v-if="graphStatus" class="json json-scroll">{{ JSON.stringify(graphStatus, null, 2) }}</pre>
        </details>
        <p v-if="graphErr" class="err">{{ graphErr }}</p>
      </section>

      <section v-else-if="tab === 'federation'" class="panel">
        <h2>Federation</h2>
        <div class="guide">
          <h3>How to use</h3>
          <ol>
            <li>Register one or more test agents and click <strong>Refresh registry</strong>.</li>
            <li>Connect stream to watch delegated events live.</li>
            <li>Delegate by capability (for example <code>chat</code>).</li>
          </ol>
        </div>
        <p class="hint">
          In-process ACL via <code>GET /api/federation/stream</code> (SSE) and
          <code>POST /api/federation/delegate</code>. With
          <code>kowalski-cli --features postgres</code> and <code>memory.database_url</code>,
          NOTIFY on channel <code>kowalski_federation</code> is also forwarded to this broker. Connect
          the stream, then delegate — you should see one <code>AclEnvelope</code> per event.
        </p>
        <p>
          <button type="button" class="primary" @click="loadFederationRegistry">
            Refresh registry
          </button>
        </p>
        <div v-if="federationAgents.length" class="cards">
          <article v-for="agent in federationAgents" :key="agent.id" class="card">
            <header>
              <strong>{{ agent.id }}</strong>
              <span class="status-badge status-ok">ACTIVE</span>
            </header>
            <p class="muted">Capabilities: {{ agent.capabilities.join(", ") || "(none)" }}</p>
            <p v-if="agent.state" class="muted">State available</p>
          </article>
        </div>
        <p v-else class="muted">No registered agents.</p>
        <details>
          <summary>Raw federation registry JSON</summary>
          <pre v-if="fedRegistry" class="json json-scroll">{{ JSON.stringify(fedRegistry, null, 2) }}</pre>
        </details>
        <p v-if="fedRegistryErr" class="err">{{ fedRegistryErr }}</p>
        <p>
          <label class="lbl">Topic</label>
          <input v-model="fedTopic" class="inp" type="text" />
        </p>
        <p>
          <button type="button" class="primary" @click="fedConnect">Connect stream</button>
          <button type="button" @click="fedDisconnect">Disconnect</button>
        </p>
        <div v-if="fedLines.length" class="fed-events">
          <details
            v-for="(line, i) in fedLines"
            :key="i"
            class="fed-event"
            :open="i >= fedLines.length - 3"
          >
            <summary>Event {{ i + 1 }}</summary>
            <pre class="json fed-line">{{ line }}</pre>
          </details>
        </div>
        <p v-else class="muted">(no events yet)</p>
        <p class="row">
          <label class="lbl">Mark stale (seconds)</label>
          <input v-model.number="fedStaleSecs" class="inp" type="number" min="30" />
          <button
            type="button"
            :disabled="fedCleanupBusy"
            @click="fedCleanupStale"
          >
            {{ fedCleanupBusy ? "…" : "POST /api/federation/cleanup-stale" }}
          </button>
        </p>
        <p class="row">
          <label class="lbl">Register agent id</label>
          <input v-model="fedRegId" class="inp" type="text" placeholder="worker-1" />
        </p>
        <p class="row">
          <label class="lbl">Capabilities (comma-separated)</label>
          <input v-model="fedRegCaps" class="inp" type="text" />
          <button type="button" :disabled="fedBusy" @click="fedRegister">Register</button>
        </p>
        <p class="row">
          <label class="lbl">Deregister agent id</label>
          <input v-model="fedDeregisterId" class="inp" type="text" />
          <button type="button" :disabled="fedBusy" @click="fedDeregister">Deregister</button>
        </p>
        <h3>Delegate (ranked match)</h3>
        <p>
          <label class="lbl">task_id</label>
          <input v-model="fedTaskId" class="inp" type="text" />
        </p>
        <p>
          <label class="lbl">instruction</label>
          <input v-model="fedInstruction" class="inp" type="text" />
        </p>
        <p>
          <label class="lbl">capability</label>
          <input v-model="fedCap" class="inp" type="text" />
        </p>
        <p>
          <button type="button" class="primary" :disabled="fedBusy" @click="fedDelegate">
            {{ fedBusy ? "Delegating…" : "POST /api/federation/delegate" }}
          </button>
        </p>
        <details>
          <summary>Raw federation action JSON</summary>
          <pre v-if="fedResult" class="json json-scroll">{{ fedResult }}</pre>
        </details>
        <p v-if="fedErr" class="err">{{ fedErr }}</p>
      </section>

      <section v-else-if="tab === 'mcp'" class="panel">
        <h2>MCP</h2>
        <div class="guide">
          <h3>How to use</h3>
          <ol>
            <li>Add <code>[[mcp.servers]]</code> entries in <code>config.toml</code>.</li>
            <li>Reload server list to verify the config is loaded.</li>
            <li>Run ping and check <code>ok</code> plus <code>tool_count</code>.</li>
          </ol>
        </div>
        <p>
          <button type="button" class="primary" @click="loadServers">Reload server list</button>
          <button type="button" :disabled="pingBusy" @click="runMcpPing">
            {{ pingBusy ? "Pinging…" : "Ping all (initialize + tools/list)" }}
          </button>
        </p>
        <p v-if="serversErr" class="err">{{ serversErr }}</p>
        <div v-if="servers.length" class="cards">
          <article v-for="s in servers" :key="`${s.name}-${s.url}`" class="card">
            <header>
              <strong>{{ s.name }}</strong>
              <span class="status-badge status-neutral">{{ s.transport.toUpperCase() }}</span>
            </header>
            <p class="muted">{{ s.url }}</p>
          </article>
        </div>
        <p v-else-if="!serversErr" class="muted">No servers or empty config.</p>
        <details>
          <summary>Raw MCP servers JSON</summary>
          <pre v-if="servers.length" class="json json-scroll">{{ JSON.stringify(servers, null, 2) }}</pre>
        </details>
        <h3>Ping results</h3>
        <div v-if="pingResults?.length" class="cards">
          <article v-for="r in pingResults" :key="`${r.name}-${r.url}`" class="card">
            <header>
              <strong>{{ r.name }}</strong>
              <span class="status-badge" :class="statusClass(r.ok)">{{ statusLabel(r.ok) }}</span>
            </header>
            <p class="muted">{{ r.transport }} · {{ r.url }}</p>
            <p v-if="r.ok" class="muted">tools: {{ r.tool_count ?? 0 }}</p>
            <p v-else class="err">{{ r.error }}</p>
          </article>
        </div>
        <details>
          <summary>Raw MCP ping JSON</summary>
          <pre v-if="pingResults" class="json json-scroll">{{ JSON.stringify(pingResults, null, 2) }}</pre>
        </details>
        <p v-if="pingErr" class="err">{{ pingErr }}</p>
      </section>

      <section v-else-if="tab === 'chat'" class="panel">
        <h2>Chat</h2>
        <div class="guide">
          <h3>How to use</h3>
          <ol>
            <li>Ask a question with <strong>Send</strong> or <strong>Send (SSE)</strong>.</li>
            <li>Ask follow-up questions in the same session.</li>
            <li>Use <strong>New conversation</strong> to reset context.</li>
          </ol>
        </div>
        <p class="hint">
          One in-process agent + Ollama via <code>POST /api/chat</code> or SSE
          <code>POST /api/chat/stream</code> (one JSON event per line; assistant text arrives when
          the turn completes). With <strong>Tool-aware stream</strong>, the server runs the tool loop
          and emits <code>token</code> events only for the final LLM reply after tool result(s). Run
          <code>kowalski-cli serve -c config.toml</code> with Ollama up and the model from
          <code>[ollama]</code>.
        </p>
        <p class="row">
          <label class="chk">
            <input v-model="chatToolsStream" type="checkbox" />
            Tool-aware stream (<code>tools_stream</code>)
          </label>
        </p>
        <textarea v-model="chatIn" rows="4" class="ta" placeholder="Message…" />
        <p>
          <button type="button" class="primary" :disabled="chatBusy" @click="sendChat">
            {{ chatBusy ? "Sending…" : "Send" }}
          </button>
          <button type="button" :disabled="chatBusy" @click="sendChatStream">
            {{ chatBusy ? "Sending…" : "Send (SSE)" }}
          </button>
          <button type="button" :disabled="resetBusy" @click="resetChat">
            {{ resetBusy ? "Resetting…" : "New conversation" }}
          </button>
        </p>
        <p v-if="sessionId" class="muted">Session: {{ sessionId }}</p>
        <p v-if="chatMeta" class="muted">{{ chatMeta }}</p>
        <pre v-if="chatOut" class="json json-scroll chat-out">{{ chatOut }}</pre>
        <p v-if="chatErr" class="err">{{ chatErr }}</p>
        <h3>Transcript</h3>
        <div v-if="chatTurns.length" class="chat-history">
          <article
            v-for="(turn, idx) in chatTurns"
            :key="idx"
            class="chat-turn"
            :class="`turn-${turn.role}`"
          >
            <header>{{ turn.role === "user" ? "You" : "Assistant" }}</header>
            <pre class="chat-turn-content">{{ turn.content }}</pre>
          </article>
        </div>
        <p v-else class="muted">(no messages yet)</p>
      </section>
    </main>
  </div>
</template>

<style>
:root {
  font-family: system-ui, sans-serif;
  color: #e8e8ec;
  background: #12141a;
}
body {
  margin: 0;
}
.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}
.header {
  padding: 1rem 1.5rem;
  border-bottom: 1px solid #2a2e38;
  background: #1a1d26;
}
.header h1 {
  margin: 0;
  font-size: 1.35rem;
  font-weight: 600;
}
.tagline {
  margin: 0.35rem 0 0.75rem;
  color: #8b92a5;
  font-size: 0.9rem;
}
.nav {
  display: flex;
  gap: 0.5rem;
}
.nav button {
  background: #2a3142;
  border: 1px solid #3d4658;
  color: #c8cfdd;
  padding: 0.4rem 0.75rem;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
}
.nav button.active {
  background: #3d5a8c;
  border-color: #5a7ab8;
  color: #fff;
}
.main {
  flex: 1;
  padding: 1.25rem 1.5rem;
  max-width: 52rem;
}
.panel h2 {
  margin-top: 0;
  font-size: 1.1rem;
}
.panel h3 {
  font-size: 1rem;
  margin-top: 1.25rem;
}
.panel p {
  line-height: 1.55;
  color: #b8c0d0;
}
.hint {
  font-size: 0.9rem;
  color: #8b92a5;
}
.guide {
  border: 1px solid #2a2e38;
  border-radius: 8px;
  background: #171b22;
  padding: 0.65rem 0.8rem;
  margin-bottom: 0.8rem;
}
.guide h3 {
  margin: 0 0 0.35rem;
  font-size: 0.92rem;
}
.guide ol {
  margin: 0;
  padding-left: 1.1rem;
  color: #aeb8cc;
}
.guide li {
  margin: 0.2rem 0;
}
details {
  margin: 0.45rem 0;
}
details > summary {
  cursor: pointer;
  color: #9aa8c0;
  font-size: 0.86rem;
}
.cards {
  display: grid;
  gap: 0.45rem;
}
.card {
  border: 1px solid #2a2e38;
  border-radius: 8px;
  background: #171b22;
  padding: 0.55rem 0.65rem;
}
.card header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}
.status-badge {
  border-radius: 999px;
  font-size: 0.72rem;
  padding: 0.12rem 0.45rem;
  border: 1px solid transparent;
}
.status-ok {
  color: #8de3a8;
  border-color: #2f7c47;
  background: #153323;
}
.status-error {
  color: #ffb0b0;
  border-color: #8d3a3a;
  background: #381b1b;
}
.status-neutral {
  color: #b9c8ef;
  border-color: #41598e;
  background: #1c2844;
}
.row {
  margin: 0.35rem 0 0.5rem;
}
.chk {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.9rem;
  color: #b8c0d0;
  cursor: pointer;
}
.chk input {
  accent-color: #5a7ab8;
}
.muted {
  color: #6a7285;
  font-size: 0.9rem;
}
button.primary {
  background: #3d5a8c;
  border-color: #5a7ab8;
  color: #fff;
}
button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.ta {
  width: 100%;
  max-width: 40rem;
  box-sizing: border-box;
  background: #1a1d26;
  border: 1px solid #3d4658;
  color: #e8e8ec;
  border-radius: 6px;
  padding: 0.5rem 0.65rem;
  font: inherit;
}
.json {
  background: #1a1d26;
  border: 1px solid #2a2e38;
  border-radius: 6px;
  padding: 0.75rem;
  overflow-x: auto;
  font-size: 0.82rem;
  line-height: 1.45;
  color: #c8cfdd;
}
.err {
  color: #e88;
  font-size: 0.9rem;
}
code {
  background: #2a3142;
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
  font-size: 0.88em;
}
.lbl {
  display: block;
  font-size: 0.8rem;
  color: #8b92a5;
  margin-bottom: 0.25rem;
}
.inp {
  width: 100%;
  max-width: 28rem;
  box-sizing: border-box;
  background: #1a1d26;
  border: 1px solid #3d4658;
  color: #e8e8ec;
  border-radius: 6px;
  padding: 0.4rem 0.55rem;
  font: inherit;
}
.json-scroll {
  max-height: 18rem;
  overflow: auto;
}
.chat-out {
  max-height: 24rem;
  white-space: pre-wrap;
  word-break: break-word;
}
.chat-history {
  display: grid;
  gap: 0.5rem;
  max-height: 24rem;
  overflow: auto;
}
.chat-turn {
  border: 1px solid #2a2e38;
  border-radius: 8px;
  padding: 0.55rem 0.65rem;
  background: #171b22;
}
.chat-turn header {
  color: #9aa8c0;
  font-size: 0.8rem;
  margin-bottom: 0.2rem;
}
.chat-turn-content {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  color: #d2d9e8;
  font-size: 0.85rem;
  line-height: 1.4;
}
.turn-user {
  border-color: #3d5a8c;
}
.fed-events {
  max-height: min(50vh, 28rem);
  overflow-y: auto;
  border: 1px solid #2a2e38;
  border-radius: 6px;
  padding: 0.35rem 0.5rem;
}
.fed-event {
  margin: 0.35rem 0;
}
.fed-event summary {
  cursor: pointer;
  color: #9aa8c0;
  font-size: 0.85rem;
}
.fed-line {
  margin: 0.35rem 0 0;
  padding: 0.5rem;
  font-size: 0.78rem;
}
</style>
