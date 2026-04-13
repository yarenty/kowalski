<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { api, openFederationEventSource, type FederationRegistryResponse } from "../api";

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
const federationAgents = computed(() => fedRegistry.value?.agents ?? []);

function fedDisconnect() {
  fedEs.value?.close();
  fedEs.value = null;
}

function fedConnect() {
  fedDisconnect();
  fedLines.value = [];
  fedEs.value = openFederationEventSource(fedTopic.value, (data) => {
    try {
      const j = JSON.parse(data) as unknown;
      fedLines.value = [...fedLines.value.slice(-99), JSON.stringify(j, null, 2)];
    } catch {
      fedLines.value = [...fedLines.value.slice(-99), data];
    }
  });
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

async function fedDelegate() {
  fedBusy.value = true;
  fedErr.value = null;
  fedResult.value = null;
  try {
    const r = await api.federationDelegate({
      task_id: fedTaskId.value.trim() || "task",
      instruction: fedInstruction.value.trim() || "...",
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
    const r = await api.federationCleanupStale(Math.max(30, Math.floor(fedStaleSecs.value)));
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
  const caps = fedRegCaps.value.split(",").map((s) => s.trim()).filter(Boolean);
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

onMounted(() => {
  void loadFederationRegistry();
});

onUnmounted(() => {
  fedDisconnect();
});
</script>

<template>
  <section class="panel">
    <h2>Federation</h2>
    <p><button type="button" class="primary" @click="loadFederationRegistry">Refresh registry</button></p>
    <div v-if="federationAgents.length" class="cards">
      <article v-for="agent in federationAgents" :key="agent.id" class="card">
        <header><strong>{{ agent.id }}</strong><span class="status-badge status-ok">ACTIVE</span></header>
        <p class="muted">Capabilities: {{ agent.capabilities.join(", ") || "(none)" }}</p>
      </article>
    </div>
    <p v-else class="muted">No registered agents.</p>
    <details>
      <summary>Raw federation registry JSON</summary>
      <pre v-if="fedRegistry" class="json json-scroll">{{ JSON.stringify(fedRegistry, null, 2) }}</pre>
    </details>
    <p v-if="fedRegistryErr" class="err">{{ fedRegistryErr }}</p>

    <p><label class="lbl">Topic</label><input v-model="fedTopic" class="inp" type="text" /></p>
    <p>
      <button type="button" class="primary" @click="fedConnect">Connect stream</button>
      <button type="button" @click="fedDisconnect">Disconnect</button>
    </p>
    <div v-if="fedLines.length" class="fed-events">
      <details v-for="(line, i) in fedLines" :key="i" class="fed-event" :open="i >= fedLines.length - 3">
        <summary>Event {{ i + 1 }}</summary>
        <pre class="json fed-line">{{ line }}</pre>
      </details>
    </div>
    <p v-else class="muted">(no events yet)</p>

    <p class="row">
      <label class="lbl">Mark stale (seconds)</label>
      <input v-model.number="fedStaleSecs" class="inp" type="number" min="30" />
      <button type="button" :disabled="fedCleanupBusy" @click="fedCleanupStale">
        {{ fedCleanupBusy ? "..." : "POST /api/federation/cleanup-stale" }}
      </button>
    </p>
    <p class="row"><label class="lbl">Register agent id</label><input v-model="fedRegId" class="inp" type="text" placeholder="worker-1" /></p>
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
    <p><label class="lbl">task_id</label><input v-model="fedTaskId" class="inp" type="text" /></p>
    <p><label class="lbl">instruction</label><input v-model="fedInstruction" class="inp" type="text" /></p>
    <p><label class="lbl">capability</label><input v-model="fedCap" class="inp" type="text" /></p>
    <p>
      <button type="button" class="primary" :disabled="fedBusy" @click="fedDelegate">
        {{ fedBusy ? "Delegating..." : "POST /api/federation/delegate" }}
      </button>
    </p>
    <details>
      <summary>Raw federation action JSON</summary>
      <pre v-if="fedResult" class="json json-scroll">{{ fedResult }}</pre>
    </details>
    <p v-if="fedErr" class="err">{{ fedErr }}</p>
  </section>
</template>

<style scoped>
.panel h2 { margin-top: 0; font-size: 1.1rem; }
.panel h3 { font-size: 1rem; margin-top: 1.25rem; }
.cards { display: grid; gap: 0.45rem; }
.card { border: 1px solid #2a2e38; border-radius: 8px; background: #171b22; padding: 0.55rem 0.65rem; }
.card header { display: flex; justify-content: space-between; align-items: center; }
.status-badge { border-radius: 999px; font-size: 0.72rem; padding: 0.12rem 0.45rem; border: 1px solid #2f7c47; color: #8de3a8; background: #153323; }
.json { background: #1a1d26; border: 1px solid #2a2e38; border-radius: 6px; padding: 0.75rem; overflow-x: auto; font-size: 0.82rem; line-height: 1.45; color: #c8cfdd; }
.json-scroll { max-height: 18rem; overflow: auto; }
.row { margin: 0.35rem 0 0.5rem; }
.lbl { display: block; font-size: 0.8rem; color: #8b92a5; margin-bottom: 0.25rem; }
.inp { width: 100%; max-width: 28rem; box-sizing: border-box; background: #1a1d26; border: 1px solid #3d4658; color: #e8e8ec; border-radius: 6px; padding: 0.4rem 0.55rem; font: inherit; }
.muted { color: #6a7285; font-size: 0.9rem; }
.err { color: #e88; font-size: 0.9rem; }
.fed-events { max-height: min(50vh, 28rem); overflow-y: auto; border: 1px solid #2a2e38; border-radius: 6px; padding: 0.35rem 0.5rem; }
.fed-event { margin: 0.35rem 0; }
.fed-event summary { cursor: pointer; color: #9aa8c0; font-size: 0.85rem; }
.fed-line { margin: 0.35rem 0 0; padding: 0.5rem; font-size: 0.78rem; }
details { margin: 0.45rem 0; }
details > summary { cursor: pointer; color: #9aa8c0; font-size: 0.86rem; }
button { background: #2a3142; border: 1px solid #3d4658; color: #c8cfdd; padding: 0.4rem 0.75rem; border-radius: 6px; cursor: pointer; margin-right: 0.5rem; }
button.primary { background: #3d5a8c; border-color: #5a7ab8; color: #fff; }
</style>
