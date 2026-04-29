<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, type FederationRegistryResponse, type FederationWorkerProfile, type HordeCatalogItem } from "../api";

const hordes = ref<HordeCatalogItem[]>([]);
const workersByHorde = ref<Record<string, FederationWorkerProfile[]>>({});
const workerErr = ref<string | null>(null);
const workerAction = ref<string | null>(null);
const workerBusy = ref<string | null>(null);
const fedRegistry = ref<FederationRegistryResponse | null>(null);
const fedRegistryErr = ref<string | null>(null);

const federationAgents = computed(() => fedRegistry.value?.agents ?? []);
const hordeCards = computed(() =>
  hordes.value.map((h) => ({
    horde: h,
    workers: workersByHorde.value[h.id] ?? [],
    total: (workersByHorde.value[h.id] ?? []).length,
    healthy: (workersByHorde.value[h.id] ?? []).filter(
      (w) => w.managed_running && w.registered_exact && !w.stale_registration,
    ).length,
  })),
);

async function loadHordes() {
  const res = await api.hordes();
  hordes.value = res.hordes ?? [];
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
  workerErr.value = null;
  const next: Record<string, FederationWorkerProfile[]> = {};
  for (const h of hordes.value) {
    try {
      const res = await api.hordeWorkers(h.id);
      next[h.id] = res.workers ?? [];
    } catch (e) {
      next[h.id] = [];
      workerErr.value = e instanceof Error ? e.message : String(e);
    }
  }
  workersByHorde.value = next;
}

async function refreshAll() {
  await loadHordes();
  await Promise.all([loadWorkers(), loadRegistry()]);
}

async function startWorker(hordeId: string, step?: string) {
  workerBusy.value = `${hordeId}:${step ?? "all"}`;
  workerAction.value = null;
  try {
    await api.hordeWorkersStart(hordeId, step);
    workerAction.value = step
      ? `Started worker for ${hordeId}/${step}`
      : `Started all workers for ${hordeId}`;
    await Promise.all([loadWorkers(), loadRegistry()]);
  } catch (e) {
    workerErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    workerBusy.value = null;
  }
}

async function stopWorker(hordeId: string, step?: string) {
  workerBusy.value = `${hordeId}:${step ?? "all"}`;
  workerAction.value = null;
  try {
    await api.hordeWorkersStop(hordeId, step);
    workerAction.value = step
      ? `Stopped worker for ${hordeId}/${step}`
      : `Stopped all workers for ${hordeId}`;
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
      One card per horde. Each horde contains its own internal sub-agent workers.
    </p>
    <p><button type="button" class="primary" @click="refreshAll">Refresh</button></p>

    <h3>Hordes</h3>
    <div v-if="hordeCards.length" class="cards">
      <article v-for="card in hordeCards" :key="card.horde.id" class="card">
        <header>
          <strong>{{ card.horde.display_name }}</strong>
          <span class="status-badge" :class="card.healthy === card.total && card.total > 0 ? 'status-ok' : 'status-off'">
            {{ card.healthy }}/{{ card.total }} READY
          </span>
        </header>
        <p class="muted">{{ card.horde.description }}</p>
        <p class="muted">Sub-agents: {{ card.horde.pipeline.join(" → ") }}</p>
        <p>
          <button
            type="button"
            class="primary"
            :disabled="workerBusy === `${card.horde.id}:all`"
            @click="startWorker(card.horde.id)"
          >
            {{ workerBusy === `${card.horde.id}:all` ? "Starting..." : "Start All" }}
          </button>
          <button
            type="button"
            :disabled="workerBusy === `${card.horde.id}:all`"
            @click="stopWorker(card.horde.id)"
          >
            {{ workerBusy === `${card.horde.id}:all` ? "Stopping..." : "Stop All" }}
          </button>
        </p>
        <details open>
          <summary>View internal agents</summary>
          <div class="sub-list">
            <article v-for="p in card.workers" :key="p.id" class="sub-card">
              <header>
                <strong>{{ p.step }}</strong>
                <span class="status-badge" :class="p.managed_running && p.registered_exact && !p.stale_registration ? 'status-ok' : 'status-off'">
                  {{ p.managed_running && p.registered_exact && !p.stale_registration ? "READY" : "NOT READY" }}
                </span>
              </header>
              <p class="muted">{{ p.agent_id }} · {{ p.capability }}</p>
              <p v-if="p.last_exit" class="muted">Last exit: {{ p.last_exit }}</p>
              <p v-if="p.stale_registration" class="err">Stale registration detected.</p>
            </article>
          </div>
        </details>
      </article>
    </div>
    <p v-else class="muted">No horde cards found.</p>
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
.sub-list { display: grid; gap: 0.35rem; margin-top: 0.45rem; }
.sub-card { border: 1px solid #2a2e38; border-radius: 6px; background: #13171e; padding: 0.45rem 0.55rem; }
.card details { margin-top: 0.35rem; }
.card summary { color: #9aa8c0; cursor: pointer; font-size: 0.86rem; }
.card header { display: flex; justify-content: space-between; align-items: center; }
.status-badge { border-radius: 999px; font-size: 0.72rem; padding: 0.12rem 0.45rem; border: 1px solid #2f7c47; color: #8de3a8; background: #153323; }
.status-off { border-color: #555f74; color: #b0b7c7; background: #2a3142; }
.muted { color: #6a7285; font-size: 0.9rem; }
.err { color: #e88; font-size: 0.9rem; }
button { background: #2a3142; border: 1px solid #3d4658; color: #c8cfdd; padding: 0.4rem 0.75rem; border-radius: 6px; cursor: pointer; margin-right: 0.5rem; }
button.primary { background: #3d5a8c; border-color: #5a7ab8; color: #fff; }
</style>
