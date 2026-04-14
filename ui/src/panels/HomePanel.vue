<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import { api, type AgentsResponse, type Doctor, type Health, type MemoryStatus, type SessionsResponse } from "../api";

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
  <section class="panel">
    <h2>API status</h2>
    <p class="hint">
      Run <code>kowalski</code> (default <code>127.0.0.1:3456</code>), then
      <code>bun run dev</code> in <code>ui/</code>.
    </p>
    <p class="row">
      <button type="button" class="primary" @click="refreshAll">Refresh all</button>
      <label class="chk">
        <input v-model="autoRefresh" type="checkbox" />
        Auto-refresh
      </label>
      <input v-model.number="autoRefreshSecs" class="inp tiny" type="number" min="3" />
      <span class="muted">seconds</span>
    </p>

    <h3>Memory</h3>
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
</template>

<style scoped>
.panel h2 { margin-top: 0; font-size: 1.1rem; }
.panel h3 { font-size: 1rem; margin-top: 1.25rem; }
.hint { font-size: 0.9rem; color: #8b92a5; }
.row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.chk { display: inline-flex; align-items: center; gap: 0.4rem; color: #b8c0d0; }
.inp { background: #1a1d26; border: 1px solid #3d4658; color: #e8e8ec; border-radius: 6px; padding: 0.35rem 0.5rem; }
.inp.tiny { width: 4.5rem; }
.muted { color: #6a7285; font-size: 0.9rem; }
.json { background: #1a1d26; border: 1px solid #2a2e38; border-radius: 6px; padding: 0.75rem; overflow-x: auto; font-size: 0.82rem; line-height: 1.45; color: #c8cfdd; }
.json-scroll { max-height: 18rem; overflow: auto; }
.err { color: #e88; font-size: 0.9rem; }
.card { border: 1px solid #2a2e38; border-radius: 8px; background: #171b22; padding: 0.55rem 0.65rem; }
.card header { display: flex; justify-content: space-between; align-items: center; }
.status-badge { border-radius: 999px; font-size: 0.72rem; padding: 0.12rem 0.45rem; border: 1px solid transparent; }
.status-ok { color: #8de3a8; border-color: #2f7c47; background: #153323; }
.status-error { color: #ffb0b0; border-color: #8d3a3a; background: #381b1b; }
details { margin: 0.45rem 0; }
details > summary { cursor: pointer; color: #9aa8c0; font-size: 0.86rem; }
code { background: #2a3142; padding: 0.15rem 0.4rem; border-radius: 4px; font-size: 0.88em; }
button { background: #2a3142; border: 1px solid #3d4658; color: #c8cfdd; padding: 0.35rem 0.7rem; border-radius: 6px; cursor: pointer; }
button.primary { background: #3d5a8c; border-color: #5a7ab8; color: #fff; }
</style>
