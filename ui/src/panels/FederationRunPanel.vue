<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  api,
  openFederationEventSource,
  type FederationWorkerProfile,
  type HordeCatalogItem,
  type HordeRunRecord,
} from "../api";

const fedTopic = ref("federation");
const fedEs = ref<EventSource | null>(null);
const hordes = ref<HordeCatalogItem[]>([]);
const selectedHordeId = ref<string>("");
const runPrompt = ref("can you check https://yarenty.com and get summary into obsidian?");
const runBusy = ref(false);
const runId = ref<string | null>(null);
const runMessages = ref<Array<{ role: "orchestrator" | "worker" | "system"; text: string }>>([]);
const runResult = ref<string | null>(null);
const runErr = ref<string | null>(null);
const runWatchdog = ref<number | null>(null);
const workerProfiles = ref<FederationWorkerProfile[]>([]);
const selectedProfileId = ref<string>("");
const runHistory = ref<HordeRunRecord[]>([]);

const selectedHorde = computed(() => hordes.value.find((h) => h.id === selectedHordeId.value) ?? null);
const selectedProfile = computed(() =>
  workerProfiles.value.find((p) => p.id === selectedProfileId.value) ?? null,
);

watch(
  () => selectedHordeId.value,
  async () => {
    await Promise.all([loadProfiles(), loadRunHistory()]);
  },
  { immediate: true },
);

function clearRunWatchdog() {
  if (runWatchdog.value !== null) {
    window.clearTimeout(runWatchdog.value);
    runWatchdog.value = null;
  }
}

function feed(role: "orchestrator" | "worker" | "system", text: string) {
  runMessages.value = [...runMessages.value, { role, text }];
}

function extractUrl(input: string): string | null {
  const m = input.match(/https?:\/\/\S+/);
  return m ? m[0] : null;
}

function processFederationEvent(data: string) {
  let parsed: unknown = data;
  try {
    parsed = JSON.parse(data) as unknown;
  } catch {
    return;
  }
  const envelope = parsed as { payload?: Record<string, unknown> };
  const payload = envelope.payload;
  if (!payload || typeof payload !== "object") return;
  const kind = String(payload.kind ?? "");
  const evRunId = String(payload.run_id ?? "");
  if (runId.value && evRunId && evRunId !== runId.value) return;

  if (kind === "task_assigned") {
    feed("orchestrator", `${String(payload.step ?? "?")} assigned to ${String(payload.to ?? "?")}`);
  } else if (kind === "task_started") {
    feed("worker", `${String(payload.step ?? "?")} started by ${String(payload.agent ?? "?")}`);
  } else if (kind === "agent_message") {
    feed("worker", String(payload.text ?? "(message)"));
  } else if (kind === "task_finished") {
    const step = String(payload.step ?? "?");
    const artifact = String(payload.artifact ?? "");
    const ok = Boolean(payload.success);
    feed("worker", `${step} ${ok ? "completed" : "failed"}${artifact ? ` -> ${artifact}` : ""}`);
  } else if (kind === "run_finished") {
    runResult.value = JSON.stringify(payload, null, 2);
    feed("system", "run finished");
    runBusy.value = false;
    clearRunWatchdog();
    void loadRunHistory();
  } else if (kind === "run_failed") {
    runResult.value = JSON.stringify(payload, null, 2);
    feed("system", "run failed");
    runBusy.value = false;
    clearRunWatchdog();
    void loadRunHistory();
  }
}

function connectStream() {
  fedEs.value?.close();
  fedEs.value = openFederationEventSource(fedTopic.value, processFederationEvent);
}

async function loadHordes() {
  const res = await api.hordes();
  hordes.value = res.hordes ?? [];
  if (!selectedHordeId.value && hordes.value.length) selectedHordeId.value = hordes.value[0].id;
}

async function loadProfiles() {
  if (!selectedHordeId.value) return;
  try {
    const r = await api.hordeWorkers(selectedHordeId.value);
    workerProfiles.value = r.workers ?? [];
    if (!workerProfiles.value.some((p) => p.id === selectedProfileId.value) && workerProfiles.value.length) {
      selectedProfileId.value = workerProfiles.value[0].id;
    }
  } catch {
    workerProfiles.value = [];
  }
}

const RUN_HISTORY_KEY = "kowalski.ui.horde-runs.v1";
function persistRunHistory() {
  localStorage.setItem(RUN_HISTORY_KEY, JSON.stringify(runHistory.value.slice(0, 30)));
}
function restoreRunHistory() {
  const raw = localStorage.getItem(RUN_HISTORY_KEY);
  if (!raw) return;
  try {
    runHistory.value = JSON.parse(raw) as HordeRunRecord[];
  } catch {
    runHistory.value = [];
  }
}
async function loadRunHistory() {
  if (!selectedHordeId.value) return;
  const res = await api.hordeRuns(selectedHordeId.value);
  runHistory.value = res.runs ?? [];
  persistRunHistory();
}

async function runKnowledgeCompiler() {
  await Promise.all([loadHordes(), loadProfiles()]);
  const source = extractUrl(runPrompt.value);
  if (!source) {
    runErr.value = "Prompt must include URL, e.g. https://yarenty.com.";
    return;
  }
  runErr.value = null;
  if (!selectedProfile.value) {
    runErr.value = "Select worker profile first.";
    return;
  }
  if (!selectedProfile.value.managed_running) {
    const lastExit = selectedProfile.value.last_exit ? ` Last exit: ${selectedProfile.value.last_exit}.` : "";
    runErr.value = `Selected worker is not running: ${selectedProfile.value.name}.${lastExit} Start it in Federation Management first.`;
    return;
  }
  if (!selectedProfile.value.registered_exact) {
    runErr.value =
      `Selected worker is starting but not registered yet: ${selectedProfile.value.name}. ` +
      "Wait a few seconds, click Refresh worker profiles, and retry.";
    return;
  }
  if (selectedProfile.value.stale_registration) {
    runErr.value =
      `Selected worker has stale registration: ${selectedProfile.value.name}. ` +
      "Stop and start worker in Federation Management, then retry.";
    return;
  }
  runResult.value = null;
  runBusy.value = true;
  runId.value = null;
  runMessages.value = [];
  clearRunWatchdog();
  connectStream();
  feed("orchestrator", "creating run");
  feed("orchestrator", `source: ${source}`);
  try {
    const out = await api.hordeRun(selectedHordeId.value, {
      prompt: runPrompt.value.trim(),
      source,
    });
    runId.value = out.run.run_id;
    feed("orchestrator", `run started: ${out.run.run_id}`);
    runWatchdog.value = window.setTimeout(() => {
      if (!runBusy.value) return;
      runBusy.value = false;
      feed("system", "timeout: no progress events within 60s");
      runResult.value =
        "Run created, but no worker progress arrived in 60s. Check sub-agent worker status in Federation Management.";
    }, 60_000);
  } catch (e) {
    runBusy.value = false;
    runErr.value = e instanceof Error ? e.message : String(e);
  }
}

onMounted(() => {
  restoreRunHistory();
  connectStream();
  void loadHordes();
  void loadProfiles();
  void loadRunHistory();
});

onUnmounted(() => {
  fedEs.value?.close();
  clearRunWatchdog();
});
</script>

<template>
  <section class="panel">
    <h2>Horde Run</h2>
    <p class="muted">
      Orchestrator delegates by capability. Worker executes internal sub-agents and streams step progress.
    </p>
    <p><button type="button" class="primary" @click="loadHordes">Refresh hordes</button></p>
    <p><button type="button" @click="loadProfiles">Refresh worker profiles</button></p>
    <p>
      <label class="lbl">Horde</label>
      <select v-model="selectedHordeId" class="inp">
        <option v-for="h in hordes" :key="h.id" :value="h.id">{{ h.display_name }}</option>
      </select>
    </p>
    <p v-if="selectedHorde" class="muted">{{ selectedHorde.description }}</p>
    <p>
      <label class="lbl">Worker profile</label>
      <select v-model="selectedProfileId" class="inp">
        <option v-for="p in workerProfiles" :key="p.id" :value="p.id">
          {{ p.name }} ({{ p.managed_running ? "RUNNING" : "STOPPED" }})
        </option>
      </select>
    </p>
    <p class="muted">Capability: {{ selectedProfile?.capability || "(none)" }}</p>

    <p><label class="lbl">Prompt</label><input v-model="runPrompt" class="inp" type="text" /></p>
    <p>
      <button type="button" class="primary" :disabled="runBusy" @click="runKnowledgeCompiler">
        {{ runBusy ? "Running..." : "Run via orchestrator" }}
      </button>
    </p>
    <p v-if="runId" class="muted">Run ID: {{ runId }}</p>

    <div class="chat-feed">
      <article v-for="(m, i) in runMessages" :key="i" class="msg" :class="`msg-${m.role}`">
        <header>{{ m.role }}</header>
        <pre>{{ m.text }}</pre>
      </article>
      <p v-if="!runMessages.length" class="muted">(no run messages yet)</p>
    </div>

    <details v-if="runResult">
      <summary>Final delivery</summary>
      <pre class="json">{{ runResult }}</pre>
    </details>
    <details v-if="runHistory.length">
      <summary>Previous runs</summary>
      <pre class="json">{{ JSON.stringify(runHistory.slice(0, 8), null, 2) }}</pre>
    </details>
    <p v-if="runErr" class="err">{{ runErr }}</p>
  </section>
</template>

<style scoped>
.panel h2 { margin-top: 0; font-size: 1.1rem; }
.muted { color: #6a7285; font-size: 0.9rem; }
.err { color: #e88; font-size: 0.9rem; }
.lbl { display: block; font-size: 0.8rem; color: #8b92a5; margin-bottom: 0.25rem; }
.inp { width: 100%; max-width: 48rem; box-sizing: border-box; background: #1a1d26; border: 1px solid #3d4658; color: #e8e8ec; border-radius: 6px; padding: 0.4rem 0.55rem; font: inherit; }
.chat-feed { border: 1px solid #2a2e38; border-radius: 8px; background: #141820; padding: 0.6rem; display: grid; gap: 0.45rem; max-height: 55vh; overflow: auto; }
.msg { border: 1px solid #2a2e38; border-radius: 8px; background: #171b22; padding: 0.5rem 0.65rem; }
.msg header { color: #9aa8c0; font-size: 0.8rem; margin-bottom: 0.2rem; text-transform: capitalize; }
.msg pre { margin: 0; white-space: pre-wrap; word-break: break-word; color: #d2d9e8; font-size: 0.85rem; }
.msg-orchestrator { border-color: #5a7ab8; }
.msg-worker { border-color: #2f7c47; }
.msg-system { border-color: #555f74; }
.json { background: #1a1d26; border: 1px solid #2a2e38; border-radius: 6px; padding: 0.75rem; overflow-x: auto; font-size: 0.82rem; line-height: 1.45; color: #c8cfdd; }
button { background: #2a3142; border: 1px solid #3d4658; color: #c8cfdd; padding: 0.4rem 0.75rem; border-radius: 6px; cursor: pointer; margin-right: 0.5rem; }
button.primary { background: #3d5a8c; border-color: #5a7ab8; color: #fff; }
</style>
