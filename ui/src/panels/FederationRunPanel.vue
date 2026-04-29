<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  api,
  openFederationEventSource,
  type FederationWorkerProfile,
  type HordeCatalogItem,
  type HordeRunRecord,
} from "../api";
const props = defineProps<{ activeThreadId: string | null }>();
const emit = defineEmits<{
  (e: "thread-upsert", item: { id: string; title: string; updatedAt: number }): void;
}>();

const fedTopic = ref("federation");
const fedEs = ref<EventSource | null>(null);
const hordes = ref<HordeCatalogItem[]>([]);
const selectedHordeId = ref<string>("");
const runBusy = ref(false);
const runId = ref<string | null>(null);
const runMessages = ref<Array<{ role: "orchestrator" | "worker" | "system"; speaker: string; text: string }>>([]);
const runResult = ref<string | null>(null);
const runErr = ref<string | null>(null);
const runWatchdog = ref<number | null>(null);
const workerProfiles = ref<FederationWorkerProfile[]>([]);
const runHistory = ref<HordeRunRecord[]>([]);
const followupInput = ref("");
const followupBusy = ref(false);
const followupMsgs = ref<Array<{ role: "user" | "assistant" | "orchestrator"; speaker: string; text: string }>>([]);

const selectedHorde = computed(() => hordes.value.find((h) => h.id === selectedHordeId.value) ?? null);
const selectedHordeWorkers = computed(() => workerProfiles.value.filter((w) => w.horde_id === selectedHordeId.value));
const activeRunFromHistory = computed(() =>
  runId.value ? runHistory.value.find((r) => r.run_id === runId.value) ?? null : null,
);
const latestCompletedRun = computed(() =>
  runHistory.value.find((r) => r.status === "completed") ?? null,
);
const finalDelivery = computed(() => {
  if (!runResult.value) return null;
  try {
    return JSON.parse(runResult.value) as {
      kind?: string;
      text?: string;
      artifacts?: Array<[string, string]>;
    };
  } catch {
    return null;
  }
});
const finalArtifacts = computed(() => finalDelivery.value?.artifacts ?? []);
const runCompleted = computed(
  () =>
    finalDelivery.value?.kind === "run_finished" ||
    activeRunFromHistory.value?.status === "completed",
);
const progressText = ref("idle");
const obsidianRoot = computed(() =>
  selectedHorde.value?.root_path
    ? `${selectedHorde.value.root_path}/${selectedHorde.value.delivery_root_rel || "wiki"}`
    : "(unknown)",
);
const finalShortSummary = computed(() => selectedHorde.value?.delivery_summary_note || "Run completed.");
const hasCompletedRun = computed(() => runCompleted.value || !!latestCompletedRun.value);

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

function titleCase(input: string): string {
  if (!input) return input;
  return input.slice(0, 1).toUpperCase() + input.slice(1);
}

function speakerNameFromStep(step?: string): string {
  if (!step) return "Agent: Worker";
  return `Agent: ${titleCase(step)}`;
}

function feed(role: "orchestrator" | "worker" | "system", text: string, speaker?: string) {
  runMessages.value = [...runMessages.value, { role, speaker: speaker || (role === "orchestrator" ? "Agent: Boss" : "System"), text }];
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
    const step = String(payload.step ?? "?");
    progressText.value = `assigned ${step}`;
    feed("orchestrator", `${step} assigned to ${String(payload.to ?? "?")}`, "Agent: Boss");
  } else if (kind === "task_started") {
    const step = String(payload.step ?? "?");
    progressText.value = `${step} running`;
    feed("worker", `${step} started by ${String(payload.agent ?? "?")}`, speakerNameFromStep(step));
  } else if (kind === "agent_message") {
    const step = String(payload.step ?? "");
    feed("worker", String(payload.text ?? "(message)"), speakerNameFromStep(step || "worker"));
  } else if (kind === "task_finished") {
    const step = String(payload.step ?? "?");
    const artifact = String(payload.artifact ?? "");
    const ok = Boolean(payload.success);
    progressText.value = ok ? `${step} completed` : `${step} failed`;
    feed("worker", `${step} ${ok ? "completed" : "failed"}${artifact ? ` -> ${artifact}` : ""}`, speakerNameFromStep(step));
  } else if (kind === "run_finished") {
    runResult.value = JSON.stringify(payload, null, 2);
    progressText.value = "finished";
    feed("system", "run finished", "System");
    runBusy.value = false;
    clearRunWatchdog();
    void loadRunHistory();
  } else if (kind === "run_failed") {
    runResult.value = JSON.stringify(payload, null, 2);
    progressText.value = "failed";
    feed("system", "run failed", "System");
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
  } catch {
    workerProfiles.value = [];
  }
}

async function refreshAll() {
  await loadHordes();
  await Promise.all([loadProfiles(), loadRunHistory()]);
}

const RUN_HISTORY_KEY = "kowalski.ui.horde-runs.v1";
function threadStateKey(id: string) {
  return `kowalski.ui.horde.thread.${id}`;
}

function saveActiveThreadState() {
  if (!props.activeThreadId) return;
  const snapshot = {
    selectedHordeId: selectedHordeId.value,
    runId: runId.value,
    runMessages: runMessages.value,
    runResult: runResult.value,
    followupMsgs: followupMsgs.value,
  };
  localStorage.setItem(threadStateKey(props.activeThreadId), JSON.stringify(snapshot));
}

function loadActiveThreadState(id: string) {
  const raw = localStorage.getItem(threadStateKey(id));
  if (!raw) {
    runId.value = null;
    runMessages.value = [];
    runResult.value = null;
    followupMsgs.value = [];
    return;
  }
  try {
    const parsed = JSON.parse(raw) as {
      selectedHordeId?: string;
      runId?: string | null;
      runMessages?: Array<{ role: "orchestrator" | "worker" | "system"; speaker: string; text: string }>;
      runResult?: string | null;
      followupMsgs?: Array<{ role: "user" | "assistant" | "orchestrator"; speaker: string; text: string }>;
    };
    if (parsed.selectedHordeId) selectedHordeId.value = parsed.selectedHordeId;
    runId.value = parsed.runId ?? null;
    runMessages.value = parsed.runMessages ?? [];
    runResult.value = parsed.runResult ?? null;
    followupMsgs.value = parsed.followupMsgs ?? [];
  } catch {
    runId.value = null;
    runMessages.value = [];
    runResult.value = null;
    followupMsgs.value = [];
  }
}

function upsertThreadMeta() {
  if (!props.activeThreadId) return;
  const lastUser = [...followupMsgs.value].reverse().find((m) => m.role === "user")?.text;
  const title =
    lastUser?.slice(0, 42) ||
    runMessages.value.find((m) => m.speaker === "Agent: Boss" && m.text.startsWith("source:"))?.text.replace("source: ", "") ||
    "New horde interaction";
  emit("thread-upsert", {
    id: props.activeThreadId,
    title,
    updatedAt: Date.now(),
  });
}

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
  if (!runId.value && runHistory.value.length) {
    runId.value = runHistory.value[0].run_id;
  }
  if (!runResult.value) {
    const completed = runHistory.value.find((r) => r.status === "completed");
    if (completed) {
      const artifacts = completed.steps
        .filter((s) => !!s.artifact)
        .map((s) => [s.step, s.artifact] as [string, string]);
      runResult.value = JSON.stringify(
        {
          kind: "run_finished",
          run_id: completed.run_id,
          horde: completed.horde_id,
          artifacts,
          text: `${selectedHorde.value?.display_name || "Horde"} run completed; ${artifacts.length} artifact(s).`,
        },
        null,
        2,
      );
    }
  }
  persistRunHistory();
}

watch(
  () => props.activeThreadId,
  (id) => {
    if (!id) return;
    loadActiveThreadState(id);
  },
  { immediate: true },
);
watch(
  [selectedHordeId, runId, runResult, runMessages, followupMsgs],
  () => {
    saveActiveThreadState();
    upsertThreadMeta();
  },
  { deep: true },
);

async function runKnowledgeCompiler() {
  await Promise.all([loadHordes(), loadProfiles()]);
  const prompt = followupInput.value.trim();
  const source = extractUrl(prompt);
  if (!source) {
    runErr.value = "Prompt must include URL, e.g. https://yarenty.com.";
    return;
  }
  runErr.value = null;
  if (!selectedHordeWorkers.value.length) {
    runErr.value = "No workers loaded for selected horde.";
    return;
  }
  const notRunning = selectedHordeWorkers.value.filter((w) => !w.managed_running);
  if (notRunning.length) {
    runErr.value =
      `Horde ${selectedHorde.value?.display_name ?? selectedHordeId.value} is not fully ready. ` +
      `Start all sub-agents in Federation Management. Missing: ${notRunning.map((w) => w.step).join(", ")}.`;
    return;
  }
  const notRegistered = selectedHordeWorkers.value.filter((w) => !w.registered_exact || w.stale_registration);
  if (notRegistered.length) {
    runErr.value =
      `Horde has stale/unregistered agents: ${notRegistered.map((w) => w.step).join(", ")}. ` +
      "Refresh profiles or restart all workers in Federation Management.";
    return;
  }
  runResult.value = null;
  followupMsgs.value = [];
  runBusy.value = true;
  progressText.value = "starting";
  runId.value = null;
  runMessages.value = [];
  clearRunWatchdog();
  connectStream();
  feed("orchestrator", "creating run", "Agent: Boss");
  feed("orchestrator", `source: ${source}`, "Agent: Boss");
  try {
    const out = await api.hordeRun(selectedHordeId.value, {
      prompt,
      source,
    });
    followupInput.value = "";
    runId.value = out.run.run_id;
    feed("orchestrator", `run started: ${out.run.run_id}`, "Agent: Boss");
    runWatchdog.value = window.setTimeout(() => {
      if (!runBusy.value) return;
      runBusy.value = false;
      progressText.value = "timeout";
      feed("system", "timeout: no progress events within 60s", "System");
      runResult.value =
        "Run created, but no worker progress arrived in 60s. Check sub-agent worker status in Federation Management.";
    }, 60_000);
  } catch (e) {
    runBusy.value = false;
    progressText.value = "failed";
    runErr.value = e instanceof Error ? e.message : String(e);
  }
}

async function askFollowup() {
  if (!hasCompletedRun.value) {
    await runKnowledgeCompiler();
    return;
  }
  if (!selectedHordeId.value) return;
  if (!runId.value) {
    const fallbackRunId = latestCompletedRun.value?.run_id;
    if (fallbackRunId) runId.value = fallbackRunId;
  }
  if (!runId.value) return;
  const q = followupInput.value.trim();
  if (!q) return;
  followupMsgs.value = [...followupMsgs.value, { role: "user", speaker: "You", text: q }];
  followupInput.value = "";
  followupBusy.value = true;
  try {
    const out = await api.hordeFollowup(selectedHordeId.value, {
      run_id: runId.value,
      message: q,
    });
    followupMsgs.value = [
      ...followupMsgs.value,
      {
        role: "assistant",
        speaker: selectedHorde.value?.display_name || "Horde",
        text: out.output_path ? `${out.reply}\n\nSaved output: ${out.output_path}` : out.reply,
      },
    ];
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    followupMsgs.value = [
      ...followupMsgs.value,
      { role: "assistant", speaker: "System", text: `[error] ${msg}` },
    ];
  } finally {
    followupBusy.value = false;
  }
}

onMounted(() => {
  restoreRunHistory();
  connectStream();
  void refreshAll();
});

onUnmounted(() => {
  fedEs.value?.close();
  clearRunWatchdog();
});
</script>

<template>
  <section class="panel">
    <div class="panel-top">
      <h2>Horde Run</h2>
      <button type="button" class="icon-btn" title="Refresh all" aria-label="Refresh all" @click="refreshAll">
        ↻
      </button>
    </div>
    <p class="muted">
      Talk to a whole horde. The orchestrator coordinates all internal sub-agents and streams their collaboration.
    </p>
    <p>
      <label class="lbl">Horde</label>
      <select v-model="selectedHordeId" class="inp">
        <option v-for="h in hordes" :key="h.id" :value="h.id">{{ h.display_name }}</option>
      </select>
    </p>
    <div v-if="selectedHorde" class="horde-box">
      <p class="muted">{{ selectedHorde.description }}</p>
      <p class="muted"><strong>Pipeline:</strong> {{ selectedHorde.pipeline.join(" → ") }}</p>
      <p class="muted">
        <strong>Readiness:</strong>
        {{ selectedHordeWorkers.filter((w) => w.managed_running && w.registered_exact && !w.stale_registration).length }}/{{ selectedHordeWorkers.length }}
        agents ready
      </p>
    </div>

    <p v-if="runBusy" class="muted thinking">Thinking... {{ progressText }}</p>
    <p v-if="followupBusy" class="muted thinking">Thinking... follow-up in progress</p>
    <p v-if="runId" class="muted">Run ID: {{ runId }}</p>

    <div class="chat-feed">
      <article v-for="(m, i) in runMessages" :key="i" class="msg" :class="`msg-${m.role}`">
        <header>{{ m.speaker }}</header>
        <pre>{{ m.text }}</pre>
      </article>
    </div>

    <details v-if="runHistory.length">
      <summary>Previous runs</summary>
      <pre class="json">{{ JSON.stringify(runHistory.slice(0, 8), null, 2) }}</pre>
    </details>
    <section v-if="runResult" class="delivery">
      <h3 style="margin:0 0 0.35rem;">Output</h3>
      <div>
        <p class="muted">
          {{ finalDelivery?.text || "Run completed." }}
        </p>
        <p class="muted"><strong>Summary:</strong> {{ finalShortSummary }}</p>
        <p class="muted"><strong>{{ selectedHorde?.delivery_title || "Final delivery" }}</strong></p>
        <p class="muted">{{ selectedHorde?.delivery_note || "" }}</p>
        <p class="muted"><strong>Obsidian-ready folder:</strong> <code>{{ obsidianRoot }}</code></p>
        <p class="muted">
          Copy/sync this folder into your Obsidian vault (or set your vault root there).
        </p>
        <div v-if="finalArtifacts.length" class="artifact-list">
          <article v-for="a in finalArtifacts" :key="`${a[0]}-${a[1]}`" class="artifact-item">
            <strong>{{ a[0] }}</strong>
            <code>{{ a[1] }}</code>
          </article>
        </div>
        <details>
          <summary>Raw run_finished payload</summary>
          <pre class="json">{{ runResult }}</pre>
        </details>
      </div>
    </section>
    <p v-if="runErr" class="err">{{ runErr }}</p>
    <section v-if="runCompleted || latestCompletedRun" class="delivery">
      <div class="chat-feed followup-feed">
        <article
          v-for="(m, i) in followupMsgs"
          :key="`f-${i}`"
          class="msg"
          :class="m.role === 'user' ? 'msg-user' : m.role === 'orchestrator' ? 'msg-system' : 'msg-worker'"
        >
          <header>{{ m.speaker }}</header>
          <pre>{{ m.text }}</pre>
        </article>
        <p v-if="!followupMsgs.length" class="muted">(no follow-up messages yet)</p>
      </div>
    </section>
    <section class="followup-composer">
      <h3 v-if="hasCompletedRun" style="margin:0 0 0.35rem;">Follow-up chat on this run</h3>
      <p class="muted">
        {{
          hasCompletedRun
            ? 'Ask refining questions about this run, e.g. "emphasize AI findings" or "only technology part in simple language".'
            : (selectedHorde?.prompt_tip || "Enter your prompt with source URL to start a new horde interaction.")
        }}
      </p>
      <p>
        <label class="lbl">{{ hasCompletedRun ? "Follow-up question" : "Prompt" }}</label>
        <input
          v-model="followupInput"
          class="inp"
          type="text"
          :disabled="runBusy || followupBusy"
          @keydown.enter.prevent="askFollowup"
        />
      </p>
      <p>
        <button
          type="button"
          class="primary"
          :disabled="runBusy || followupBusy || !followupInput.trim()"
          @click="askFollowup"
        >
          {{
            followupBusy
              ? "Asking..."
              : runBusy
                ? "Running Horde..."
                : hasCompletedRun
                  ? "Ask follow-up"
                  : "Run Horde"
          }}
        </button>
      </p>
    </section>
  </section>
</template>

<style scoped>
.panel h2 { margin-top: 0; font-size: 1.1rem; }
.panel-top { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; }
.muted { color: #6a7285; font-size: 0.9rem; }
.err { color: #e88; font-size: 0.9rem; }
.lbl { display: block; font-size: 0.8rem; color: #8b92a5; margin-bottom: 0.25rem; }
.inp { width: 100%; max-width: 48rem; box-sizing: border-box; background: #1a1d26; border: 1px solid #3d4658; color: #e8e8ec; border-radius: 6px; padding: 0.4rem 0.55rem; font: inherit; }
.chat-feed { border: 1px solid #2a2e38; border-radius: 8px; background: #141820; padding: 0.6rem; display: grid; gap: 0.45rem; max-height: 55vh; overflow: auto; }
.followup-feed { max-height: none; overflow: visible; }
.horde-box { border: 1px solid #2a2e38; border-radius: 8px; background: #161b22; padding: 0.55rem 0.65rem; margin-bottom: 0.55rem; }
.delivery { border: 1px solid #2a2e38; border-radius: 8px; background: #151922; padding: 0.55rem 0.65rem; margin-top: 0.45rem; }
.followup-composer {
  position: sticky;
  bottom: 0;
  z-index: 5;
  border: 1px solid #2a2e38;
  border-radius: 10px;
  background: #171b22;
  padding: 0.65rem 0.75rem;
  margin-top: 0.55rem;
  box-shadow: 0 -6px 18px rgba(0, 0, 0, 0.35);
}
.artifact-list { display: grid; gap: 0.35rem; margin-top: 0.45rem; }
.artifact-item { border: 1px solid #2a2e38; border-radius: 6px; background: #12161d; padding: 0.45rem 0.55rem; display: grid; gap: 0.25rem; }
.msg { border: 1px solid #2a2e38; border-radius: 8px; background: #171b22; padding: 0.5rem 0.65rem; }
.msg header { color: #9aa8c0; font-size: 0.8rem; margin-bottom: 0.2rem; text-transform: capitalize; }
.msg pre { margin: 0; white-space: pre-wrap; word-break: break-word; color: #d2d9e8; font-size: 0.85rem; }
.msg-orchestrator { border-color: #5a7ab8; }
.msg-user { border-color: #5a7ab8; margin-left: auto; max-width: 80%; background: #1d2a42; }
.msg-worker { border-color: #2f7c47; }
.msg-system { border-color: #555f74; }
.json { background: #1a1d26; border: 1px solid #2a2e38; border-radius: 6px; padding: 0.75rem; overflow-x: auto; font-size: 0.82rem; line-height: 1.45; color: #c8cfdd; }
.thinking { color: #9cc2ff; }
button { background: #2a3142; border: 1px solid #3d4658; color: #c8cfdd; padding: 0.4rem 0.75rem; border-radius: 6px; cursor: pointer; margin-right: 0.5rem; }
button.primary { background: #3d5a8c; border-color: #5a7ab8; color: #fff; }
.icon-btn { width: 34px; height: 34px; padding: 0; font-size: 1rem; line-height: 1; margin-right: 0; }
</style>
