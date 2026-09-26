<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  api,
  type FederationRegistryResponse,
  type FederationWorkerProfile,
  type HordeCatalogItem,
  type PortabilityReport,
} from "../api";
import { isDagHorde } from "../hordeGraph";

const emit = defineEmits<{
  (e: "new-chat-session"): void;
}>();

const hordes = ref<HordeCatalogItem[]>([]);
const workersByHorde = ref<Record<string, FederationWorkerProfile[]>>({});
const workerErr = ref<string | null>(null);
const workerAction = ref<string | null>(null);
const workerBusy = ref<string | null>(null);
const fedRegistry = ref<FederationRegistryResponse | null>(null);
const fedRegistryErr = ref<string | null>(null);
const pathAction = ref<string | null>(null);
const cleanBusyHordeId = ref<string | null>(null);
const exportBusyHordeId = ref<string | null>(null);
const importFileInput = ref<HTMLInputElement | null>(null);
const importPending = ref<File | null>(null);
const importReport = ref<PortabilityReport | null>(null);
const importHordeId = ref<string | null>(null);
const importBusy = ref(false);
const importAction = ref<string | null>(null);
const importErr = ref<string | null>(null);

const importGaps = computed(() => {
  const r = importReport.value;
  if (!r) return [];
  const gaps: string[] = [];
  if (r.unknown_step_kinds.length) gaps.push(`unknown step kinds: ${r.unknown_step_kinds.join(", ")}`);
  if (r.unknown_tool_providers.length) gaps.push(`unknown tool providers: ${r.unknown_tool_providers.join(", ")}`);
  if (r.unknown_tool_ids.length) gaps.push(`missing builtin tools: ${r.unknown_tool_ids.join(", ")}`);
  if (r.unresolved_models.length) gaps.push(`unresolved pinned models: ${r.unresolved_models.join(", ")}`);
  gaps.push(...r.warnings);
  return gaps;
});

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

async function openOutputFolder(path?: string) {
  if (!path) return;
  pathAction.value = null;
  try {
    await api.openPath(path);
    await navigator.clipboard.writeText(path);
    pathAction.value = `Opened and copied path: ${path}`;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    try {
      await navigator.clipboard.writeText(path);
      pathAction.value = `Open failed (${msg}). Copied path: ${path}`;
    } catch {
      pathAction.value = `Open failed (${msg}). Path: ${path}`;
    }
  }
}

async function cleanHordeWorkdir(hordeId: string) {
  cleanBusyHordeId.value = hordeId;
  pathAction.value = null;
  try {
    const r = await api.hordeCleanWorkdir(hordeId);
    pathAction.value = `Workdir cleaned (${hordeId}): ${r.workdir}`;
    emit("new-chat-session");
  } catch (e) {
    pathAction.value = e instanceof Error ? e.message : String(e);
  } finally {
    cleanBusyHordeId.value = null;
  }
}

async function exportHorde(hordeId: string) {
  exportBusyHordeId.value = hordeId;
  pathAction.value = null;
  try {
    const { fileName, blob } = await api.hordeExportDownload(hordeId);
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = fileName;
    link.click();
    URL.revokeObjectURL(url);
    pathAction.value = `Exported ${fileName}`;
  } catch (e) {
    pathAction.value = `Export failed: ${e instanceof Error ? e.message : String(e)}`;
  } finally {
    exportBusyHordeId.value = null;
  }
}

function pickImportFile() {
  importErr.value = null;
  importAction.value = null;
  importFileInput.value?.click();
}

async function onImportFileChosen(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  importErr.value = null;
  importAction.value = null;
  importReport.value = null;
  importPending.value = file;
  importBusy.value = true;
  try {
    // Dry run first: every import gate runs, nothing lands until the operator confirms.
    const res = await api.hordeImport(file, true);
    importReport.value = res.report;
    importHordeId.value = res.horde_id;
  } catch (e) {
    importPending.value = null;
    importErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    importBusy.value = false;
  }
}

async function confirmImport() {
  const file = importPending.value;
  if (!file) return;
  importBusy.value = true;
  importErr.value = null;
  try {
    const res = await api.hordeImport(file, false);
    importAction.value = `Imported \`${res.horde_id}\` — it appears below once the catalog picks it up (no restart needed).`;
    importPending.value = null;
    importReport.value = null;
    importHordeId.value = null;
    await refreshAll();
  } catch (e) {
    importErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    importBusy.value = false;
  }
}

function cancelImport() {
  importPending.value = null;
  importReport.value = null;
  importHordeId.value = null;
}

function isWorkerReady(w: FederationWorkerProfile): boolean {
  return Boolean(w.managed_running && w.registered_exact && !w.stale_registration);
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function ensureHordeReady(hordeId: string) {
  const maxAttempts = 8;
  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    await Promise.all([loadWorkers(), loadRegistry()]);
    const workers = workersByHorde.value[hordeId] ?? [];
    if (workers.length > 0 && workers.every((w) => isWorkerReady(w))) {
      return true;
    }
    const notReady = workers.filter((w) => !isWorkerReady(w));
    for (const w of notReady) {
      // Retry only the specific step that is not yet fully registered.
      await api.hordeWorkersStart(hordeId, w.step);
    }
    await sleep(650);
  }
  return false;
}

async function startWorker(hordeId: string, step?: string) {
  workerBusy.value = `${hordeId}:${step ?? "all"}`;
  workerAction.value = null;
  try {
    await api.hordeWorkersStart(hordeId, step);
    if (step) {
      workerAction.value = `Started worker for ${hordeId}/${step}`;
      await Promise.all([loadWorkers(), loadRegistry()]);
    } else {
      workerAction.value = `Starting all workers for ${hordeId} and waiting for readiness...`;
      const ready = await ensureHordeReady(hordeId);
      workerAction.value = ready
        ? `Started all workers for ${hordeId} (ready).`
        : `Started all workers for ${hordeId}, but some are still warming up.`;
    }
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
    <p class="import-row">
      <button type="button" :disabled="importBusy" @click="pickImportFile">
        {{ importBusy && !importReport ? "Checking bundle..." : "Import bundle…" }}
      </button>
      <input
        ref="importFileInput"
        type="file"
        accept=".zip"
        class="hidden-input"
        @change="onImportFileChosen"
      />
      <span class="muted">.kwf.zip or .bbwf.zip — lands as a draft with triggers disabled</span>
    </p>
    <div v-if="importReport && importPending" class="card import-report">
      <header>
        <strong>Import `{{ importHordeId }}`?</strong>
        <span class="status-badge" :class="importGaps.length ? 'status-off' : 'status-ok'">
          {{ importGaps.length ? `${importGaps.length} GAP(S)` : "PORTABLE" }}
        </span>
      </header>
      <p v-if="!importGaps.length" class="muted">No portability gaps — fully runnable on this deployment.</p>
      <ul v-else class="muted">
        <li v-for="gap in importGaps" :key="gap">{{ gap }}</li>
      </ul>
      <p v-for="note in importReport.migrations" :key="note" class="muted">Migrated: {{ note }}</p>
      <p v-if="importReport.triggers_disabled" class="muted">
        {{ importReport.triggers_disabled }} trigger(s) will be imported disabled — re-enable them on the Horde tab.
      </p>
      <p v-if="importReport.steps_isolated" class="muted">
        {{ importReport.steps_isolated }} step(s) will run in a separate process, so a faulty step cannot affect the server.
      </p>
      <p>
        <button type="button" class="primary" :disabled="importBusy" @click="confirmImport">
          {{ importBusy ? "Importing..." : "Confirm import" }}
        </button>
        <button type="button" :disabled="importBusy" @click="cancelImport">Cancel</button>
      </p>
    </div>
    <p v-if="importAction" class="muted">{{ importAction }}</p>
    <p v-if="importErr" class="err">{{ importErr }}</p>
    <div v-if="hordeCards.length" class="cards">
      <article v-for="card in hordeCards" :key="card.horde.id" class="card">
        <header>
          <strong>{{ card.horde.display_name }}</strong>
          <span class="status-badge" :class="card.healthy === card.total && card.total > 0 ? 'status-ok' : 'status-off'">
            {{ card.healthy }}/{{ card.total }} READY
          </span>
        </header>
        <p class="muted">{{ card.horde.description }}</p>
        <p v-if="isDagHorde(card.horde.pipeline, card.horde.edges ?? [])" class="muted">
          DAG horde · {{ (card.horde.edges ?? []).length }} scheduling edge(s)
        </p>
        <p v-else class="muted">Sub-agents: {{ card.horde.pipeline.join(" → ") }}</p>
        <p v-if="card.horde.triggers?.length" class="muted trigger-row">
          <span
            v-for="t in card.horde.triggers"
            :key="t.index"
            class="trigger-badge"
            :class="t.effective_enabled ? 'trigger-on' : 'trigger-off'"
            :title="`${t.detail} — ${t.effective_enabled ? 'armed' : 'disabled'}${t.overridden ? ' (operator override)' : ''}; manage on the Horde tab`"
          >
            {{ t.detail }}{{ t.effective_enabled ? "" : " · off" }}
          </span>
        </p>
        <p class="muted workdir-row">
          Workdir: <code>{{ card.horde.workdir || card.horde.root_path }}</code>
          <button type="button" class="inline-btn" @click="openOutputFolder(card.horde.workdir || card.horde.root_path)">
            Open output folder
          </button>
        </p>
        <p class="muted workdir-row">
          <span>
            Clean on startup:
            <strong>{{ (card.horde.config_on_startup_effective ?? card.horde.config_on_startup) ? "true" : "false" }}</strong>
          </span>
          <button
            type="button"
            class="inline-btn"
            :disabled="cleanBusyHordeId === card.horde.id"
            title="Delete workdir debug tree, legacy paths, agents_log, and PASTE_ME.md for this horde"
            @click="cleanHordeWorkdir(card.horde.id)"
          >
            {{ cleanBusyHordeId === card.horde.id ? "…" : "FORCE Clean" }}
          </button>
        </p>
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
          <button
            type="button"
            :disabled="exportBusyHordeId === card.horde.id"
            title="Download this horde as a portable .kwf.zip bundle"
            @click="exportHorde(card.horde.id)"
          >
            {{ exportBusyHordeId === card.horde.id ? "Exporting..." : "Export" }}
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
    <p v-if="pathAction" class="muted">{{ pathAction }}</p>
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
.trigger-row { display: flex; gap: 0.35rem; flex-wrap: wrap; }
.trigger-badge { border-radius: 999px; font-size: 0.72rem; padding: 0.12rem 0.45rem; border: 1px solid #555f74; color: #b0b7c7; background: #2a3142; }
.trigger-on { border-color: #5a7ab8; color: #9cc2ff; background: #1d2a42; }
.trigger-off { border-color: #8a4b3b; color: #e0a184; background: #2b1c15; }
.muted { color: #6a7285; font-size: 0.9rem; }
.workdir-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.clean-on-row { align-items: center; justify-content: space-between; gap: 0.75rem; }
.err { color: #e88; font-size: 0.9rem; }
button { background: #2a3142; border: 1px solid #3d4658; color: #c8cfdd; padding: 0.4rem 0.75rem; border-radius: 6px; cursor: pointer; margin-right: 0.5rem; }
button.primary { background: #3d5a8c; border-color: #5a7ab8; color: #fff; }
.inline-btn { padding: 0.2rem 0.5rem; font-size: 0.78rem; margin-right: 0; }
.import-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.hidden-input { display: none; }
.import-report { margin-bottom: 0.55rem; }
.import-report ul { margin: 0.35rem 0; padding-left: 1.2rem; }
</style>
