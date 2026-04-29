<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, type FederationRegistryResponse, type FederationWorkerProfile, type HordeCatalogItem } from "../api";

const hordes = ref<HordeCatalogItem[]>([]);
const selectedHordeId = ref<string>("");
const workers = ref<FederationWorkerProfile[]>([]);
const workerErr = ref<string | null>(null);
const workerAction = ref<string | null>(null);
const workerBusy = ref<string | null>(null);
const fedRegistry = ref<FederationRegistryResponse | null>(null);
const fedRegistryErr = ref<string | null>(null);

const federationAgents = computed(() => fedRegistry.value?.agents ?? []);
const selectedHorde = computed(() => hordes.value.find((h) => h.id === selectedHordeId.value) ?? null);

async function loadHordes() {
  const res = await api.hordes();
  hordes.value = res.hordes ?? [];
  if (!selectedHordeId.value && hordes.value.length) selectedHordeId.value = hordes.value[0].id;
}

async function loadRegistry() {
  fedRegistryErr.value = null;
  try {
    fedRegistry.value = await api.federationRegistry();
  } catch (e) {
    fedRegistry.value = null;
    fedRegistryErr.value = e instanceof Error ? e.message : String(e);
  }
}

async function loadWorkers() {
  if (!selectedHordeId.value) return;
  workerErr.value = null;
  try {
    const res = await api.hordeWorkers(selectedHordeId.value);
    workers.value = res.workers ?? [];
  } catch (e) {
    workers.value = [];
    workerErr.value = e instanceof Error ? e.message : String(e);
  }
}

async function refreshAll() {
  await loadHordes();
  await Promise.all([loadWorkers(), loadRegistry()]);
}

async function startWorker(step: string) {
  if (!selectedHordeId.value) return;
  workerBusy.value = step;
  workerAction.value = null;
  try {
    await api.hordeWorkersStart(selectedHordeId.value, step);
    workerAction.value = `Started worker for step: ${step}`;
    await Promise.all([loadWorkers(), loadRegistry()]);
  } catch (e) {
    workerErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    workerBusy.value = null;
  }
}

async function stopWorker(step: string) {
  if (!selectedHordeId.value) return;
  workerBusy.value = step;
  workerAction.value = null;
  try {
    await api.hordeWorkersStop(selectedHordeId.value, step);
    workerAction.value = `Stopped worker for step: ${step}`;
    await Promise.all([loadWorkers(), loadRegistry()]);
  } catch (e) {
    workerErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    workerBusy.value = null;
  }
}

onMounted(() => void refreshAll());
</script>

<template>
  <section class="panel">
    <h2>Federation Management</h2>
    <p class="muted">
      Manage hordes and sub-agent workers. In this MVP, each sub-agent maps to one worker process.
    </p>
    <p><button type="button" class="primary" @click="refreshAll">Refresh</button></p>
    <p>
      <label class="muted">Horde</label>
      <select v-model="selectedHordeId" class="inp" @change="loadWorkers">
        <option v-for="h in hordes" :key="h.id" :value="h.id">{{ h.display_name }}</option>
      </select>
    </p>
    <p v-if="selectedHorde" class="muted">{{ selectedHorde.description }}</p>

    <h3>Sub-agent Workers</h3>
    <div v-if="workers.length" class="cards">
      <article v-for="p in workers" :key="p.id" class="card">
        <header>
          <strong>{{ p.step }} · {{ p.name }}</strong>
          <span class="status-badge" :class="p.managed_running ? 'status-ok' : 'status-off'">
            {{ p.managed_running ? "RUNNING" : "STOPPED" }}
          </span>
        </header>
        <p class="muted">{{ p.description }}</p>
        <p class="muted">Capability: {{ p.capability }}</p>
        <p class="muted">Agent ID: {{ p.agent_id }}</p>
        <p class="muted">Registry agents: {{ p.registry_agents.join(", ") || "(none)" }}</p>
        <p class="muted">Registered exact: {{ p.registered_exact ? "yes" : "no" }}</p>
        <p v-if="p.stale_registration" class="err">Stale registration detected (registered but worker not running).</p>
        <p v-if="p.last_exit" class="muted">Last exit: {{ p.last_exit }}</p>
        <p class="muted">Command: {{ p.command }} {{ p.args.join(" ") }}</p>
        <p>
          <button
            type="button"
            class="primary"
            :disabled="workerBusy === p.step || p.managed_running"
            @click="startWorker(p.step || '')"
          >
            {{ workerBusy === p.step ? "Starting..." : "Start" }}
          </button>
          <button
            type="button"
            :disabled="workerBusy === p.step || !p.managed_running"
            @click="stopWorker(p.step || '')"
          >
            {{ workerBusy === p.step ? "Stopping..." : "Stop" }}
          </button>
        </p>
      </article>
    </div>
    <p v-else class="muted">No workers found for selected horde.</p>
    <p v-if="workerAction" class="muted">{{ workerAction }}</p>
    <p v-if="workerErr" class="err">{{ workerErr }}</p>

    <h3>Registry (Active Agents)</h3>
    <div v-if="federationAgents.length" class="cards">
      <article v-for="agent in federationAgents" :key="agent.id" class="card">
        <header><strong>{{ agent.id }}</strong><span class="status-badge status-ok">ACTIVE</span></header>
        <p class="muted">Capabilities: {{ agent.capabilities.join(", ") || "(none)" }}</p>
      </article>
    </div>
    <p v-else class="muted">No registered agents.</p>
    <p v-if="fedRegistryErr" class="err">{{ fedRegistryErr }}</p>
  </section>
</template>

<style scoped>
.panel h2 { margin-top: 0; font-size: 1.1rem; }
.panel h3 { font-size: 1rem; margin-top: 1.1rem; }
.cards { display: grid; gap: 0.45rem; }
.card { border: 1px solid #2a2e38; border-radius: 8px; background: #171b22; padding: 0.55rem 0.65rem; }
.card header { display: flex; justify-content: space-between; align-items: center; }
.status-badge { border-radius: 999px; font-size: 0.72rem; padding: 0.12rem 0.45rem; border: 1px solid #2f7c47; color: #8de3a8; background: #153323; }
.status-off { border-color: #555f74; color: #b0b7c7; background: #2a3142; }
.muted { color: #6a7285; font-size: 0.9rem; }
.err { color: #e88; font-size: 0.9rem; }
.inp { width: 100%; max-width: 28rem; box-sizing: border-box; background: #1a1d26; border: 1px solid #3d4658; color: #e8e8ec; border-radius: 6px; padding: 0.4rem 0.55rem; font: inherit; }
button { background: #2a3142; border: 1px solid #3d4658; color: #c8cfdd; padding: 0.4rem 0.75rem; border-radius: 6px; cursor: pointer; margin-right: 0.5rem; }
button.primary { background: #3d5a8c; border-color: #5a7ab8; color: #fff; }
</style>
