<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import { api, getApiToken, setApiToken, type AgentsResponse, type Doctor, type Health, type MemoryStatus, type SessionsResponse } from "../api";

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

const autoRefresh = ref(false);
const autoRefreshSecs = ref(10);
let timer: ReturnType<typeof setInterval> | null = null;

const apiToken = ref(getApiToken());
const tokenSaved = ref(false);

function saveApiToken() {
  setApiToken(apiToken.value.trim());
  apiToken.value = getApiToken();
  tokenSaved.value = true;
  setTimeout(() => {
    tokenSaved.value = false;
  }, 2000);
  void refreshAll();
}

function statusLabel(ok: boolean): string {
  return ok ? "OK" : "ERROR";
}

function statusClass(ok: boolean): string {
  return ok ? "status-ok" : "status-error";
}

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

async function refreshAll() {
  await Promise.allSettled([
    loadHealth(),
    loadAgents(),
    loadSessions(),
    loadDoctor(),
    loadMemoryStatus(),
  ]);
}

function stopTimer() {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
}

function startTimer() {
  stopTimer();
  if (!autoRefresh.value) return;
  const ms = Math.max(3, Math.floor(autoRefreshSecs.value)) * 1000;
  timer = setInterval(() => {
    void refreshAll();
  }, ms);
}

watch([autoRefresh, autoRefreshSecs], () => {
  startTimer();
});

onMounted(() => {
  void refreshAll();
  startTimer();
});

onUnmounted(() => {
  stopTimer();
});
</script>

<template>
  <section class="page">
    <header class="page-head">
      <div>
        <p class="eyebrow">Admin // diagnostics</p>
        <h1>Diagnostics</h1>
        <p class="lead">Server health, memory, agents and sessions — for troubleshooting.</p>
      </div>
      <div class="page-head-actions">
        <label class="chk">
          <input v-model="autoRefresh" type="checkbox" />
          Auto-refresh every
        </label>
        <input v-model.number="autoRefreshSecs" class="inp tiny" type="number" min="3" aria-label="Auto-refresh seconds" />
        <span class="muted small">s</span>
        <button type="button" class="primary" @click="refreshAll">Refresh all</button>
      </div>
    </header>

    <h2 class="section-title">API token</h2>
    <p class="row">
      <label class="lbl" for="api-token">API token</label>
      <input
        id="api-token"
        v-model="apiToken"
        class="inp token"
        type="password"
        placeholder="from the server's db/api_token file"
        autocomplete="off"
        @keyup.enter="saveApiToken"
      />
      <button type="button" @click="saveApiToken">Save</button>
      <span v-if="tokenSaved" class="ok small">✓ saved</span>
    </p>
    <p class="hint">
      Only needed when the server runs with <code>--auth</code> (off by default): it then requires this
      bearer token on <code>/api/*</code> (generated at first start; the token file path is in the server
      log). Stored in this browser only.
    </p>

    <h2 class="section-title">Memory</h2>
    <article v-if="memoryStatus" class="card">
      <header>
        <strong>Embeddings</strong>
        <span class="status-badge" :class="statusClass(memoryStatus.embeddings_ok)">
          {{ statusLabel(memoryStatus.embeddings_ok) }}
        </span>
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

    <h2 class="section-title">Agents</h2>
    <details>
      <summary>Raw agents JSON</summary>
      <pre v-if="agents" class="json json-scroll">{{ JSON.stringify(agents, null, 2) }}</pre>
    </details>
    <p v-if="agentsErr" class="err">{{ agentsErr }}</p>

    <h2 class="section-title">Sessions</h2>
    <details>
      <summary>Raw sessions JSON</summary>
      <pre v-if="sessions" class="json json-scroll">{{ JSON.stringify(sessions, null, 2) }}</pre>
    </details>
    <p v-if="sessionsErr" class="err">{{ sessionsErr }}</p>

    <h2 class="section-title">Ollama probe</h2>
    <details>
      <summary>Raw doctor JSON</summary>
      <pre v-if="doctor" class="json json-scroll">{{ JSON.stringify(doctor, null, 2) }}</pre>
    </details>
    <p v-if="doctorErr" class="err">{{ doctorErr }}</p>
  </section>
</template>

<style scoped>
.row { display: flex; align-items: center; gap: 0.6rem; flex-wrap: wrap; }
.inp.tiny { width: 4.5rem; }
.inp.token { width: 22rem; max-width: 100%; }
.card { margin-bottom: 0.5rem; }
.card p { margin: 0.15rem 0; }
</style>
