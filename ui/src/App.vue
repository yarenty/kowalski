<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
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
  type SessionsResponse,
} from "./api";

const tab = ref<"home" | "mcp" | "chat" | "federation">("home");

const health = ref<Health | null>(null);
const healthErr = ref<string | null>(null);

const agents = ref<AgentsResponse | null>(null);
const agentsErr = ref<string | null>(null);

const sessions = ref<SessionsResponse | null>(null);
const sessionsErr = ref<string | null>(null);

const doctor = ref<Doctor | null>(null);
const doctorErr = ref<string | null>(null);

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

const fedTopic = ref("federation");
const fedTaskId = ref("demo-1");
const fedInstruction = ref("Smoke test");
const fedCap = ref("chat");
const fedResult = ref<string | null>(null);
const fedErr = ref<string | null>(null);
const fedBusy = ref(false);
const fedLines = ref<string[]>([]);
const fedEs = ref<EventSource | null>(null);
const fedRegistry = ref<FederationRegistryResponse | null>(null);
const fedRegistryErr = ref<string | null>(null);

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
  chatBusy.value = true;
  chatErr.value = null;
  chatOut.value = null;
  chatMeta.value = null;
  try {
    const r = await api.chat(msg);
    chatOut.value = r.reply;
    chatMeta.value = `${r.mode} · ${r.model}`;
  } catch (e) {
    chatErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    chatBusy.value = false;
  }
}

async function sendChatStream() {
  const msg = chatIn.value.trim();
  if (!msg) return;
  chatBusy.value = true;
  chatErr.value = null;
  chatOut.value = "";
  chatMeta.value = null;
  try {
    await chatStream(msg, (ev) => {
      if (ev.type === "start") {
        sessionId.value = ev.conversation_id;
        chatMeta.value = `SSE · ${ev.model}`;
      } else if (ev.type === "assistant") {
        chatOut.value = ev.content;
      } else if (ev.type === "error") {
        chatErr.value = ev.message;
      }
    });
  } catch (e) {
    chatErr.value = e instanceof Error ? e.message : String(e);
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
    void loadSessions();
  } catch (e) {
    chatErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    resetBusy.value = false;
  }
}

onMounted(() => {
  void loadHealth();
  void loadAgents();
  void loadSessions();
});

onUnmounted(() => {
  fedDisconnect();
});
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
        </p>
        <pre v-if="health" class="json">{{ JSON.stringify(health, null, 2) }}</pre>
        <p v-if="healthErr" class="err">{{ healthErr }}</p>
        <h3>Agents</h3>
        <pre v-if="agents" class="json">{{ JSON.stringify(agents, null, 2) }}</pre>
        <p v-if="agentsErr" class="err">{{ agentsErr }}</p>
        <h3>Sessions</h3>
        <pre v-if="sessions" class="json">{{ JSON.stringify(sessions, null, 2) }}</pre>
        <p v-if="sessionsErr" class="err">{{ sessionsErr }}</p>
        <h3>Ollama probe</h3>
        <pre v-if="doctor" class="json">{{ JSON.stringify(doctor, null, 2) }}</pre>
        <p v-if="doctorErr" class="err">{{ doctorErr }}</p>
      </section>

      <section v-else-if="tab === 'federation'" class="panel">
        <h2>Federation</h2>
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
        <pre v-if="fedRegistry" class="json">{{ JSON.stringify(fedRegistry, null, 2) }}</pre>
        <p v-if="fedRegistryErr" class="err">{{ fedRegistryErr }}</p>
        <p>
          <label class="lbl">Topic</label>
          <input v-model="fedTopic" class="inp" type="text" />
        </p>
        <p>
          <button type="button" class="primary" @click="fedConnect">Connect stream</button>
          <button type="button" @click="fedDisconnect">Disconnect</button>
        </p>
        <pre class="json fed-log">{{ fedLines.length ? fedLines.join("\n\n") : "(no events yet)" }}</pre>
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
        <pre v-if="fedResult" class="json">{{ fedResult }}</pre>
        <p v-if="fedErr" class="err">{{ fedErr }}</p>
      </section>

      <section v-else-if="tab === 'mcp'" class="panel">
        <h2>MCP</h2>
        <p>
          <button type="button" class="primary" @click="loadServers">Reload server list</button>
          <button type="button" :disabled="pingBusy" @click="runMcpPing">
            {{ pingBusy ? "Pinging…" : "Ping all (initialize + tools/list)" }}
          </button>
        </p>
        <p v-if="serversErr" class="err">{{ serversErr }}</p>
        <pre v-if="servers.length" class="json">{{ JSON.stringify(servers, null, 2) }}</pre>
        <p v-else-if="!serversErr" class="muted">No servers or empty config.</p>
        <h3>Ping results</h3>
        <pre v-if="pingResults" class="json">{{ JSON.stringify(pingResults, null, 2) }}</pre>
        <p v-if="pingErr" class="err">{{ pingErr }}</p>
      </section>

      <section v-else-if="tab === 'chat'" class="panel">
        <h2>Chat</h2>
        <p class="hint">
          One in-process agent + Ollama via <code>POST /api/chat</code> or SSE
          <code>POST /api/chat/stream</code> (one JSON event per line; assistant text arrives when
          the turn completes). Run <code>kowalski-cli serve -c config.toml</code> with Ollama up and
          the model from <code>[ollama]</code>.
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
        <pre v-if="chatOut" class="json">{{ chatOut }}</pre>
        <p v-if="chatErr" class="err">{{ chatErr }}</p>
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
.fed-log {
  max-height: 14rem;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
