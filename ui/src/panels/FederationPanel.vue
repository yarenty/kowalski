<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import {
  api,
  openFederationEventSource,
  type FederationRegistryResponse,
  type FederationWorkerProfile,
} from "../api";

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
const workerProfiles = ref<FederationWorkerProfile[]>([]);
const workerProfilesErr = ref<string | null>(null);
const workerBusy = ref<string | null>(null);
const knowledgeCompilerAgents = computed(() =>
  federationAgents.value.filter((a) =>
    a.capabilities.some((c) => c === "knowledge-compiler" || c === "kc.run"),
  ),
);

const runPrompt = ref("can you check https://yarenty.com and get summary into obsidian?");
const runBusy = ref(false);
const runTaskId = ref<string | null>(null);
const runTimeline = ref<string[]>([]);
const runResult = ref<string | null>(null);
const runWatchdog = ref<number | null>(null);

function extractUrl(input: string): string | null {
  const m = input.match(/https?:\/\/\S+/);
  return m ? m[0] : null;
}

function buildInstructionFromPrompt(prompt: string): { instruction: string; source: string; question: string } | null {
  const source = extractUrl(prompt);
  if (!source) return null;
  const question = prompt.trim() || "What changed?";
  return { instruction: `kc.run:${source}|${question}`, source, question };
}

function fedDisconnect() {
  fedEs.value?.close();
  fedEs.value = null;
}

function clearRunWatchdog() {
  if (runWatchdog.value !== null) {
    window.clearTimeout(runWatchdog.value);
    runWatchdog.value = null;
  }
}

function processFederationEvent(data: string) {
  let parsed: unknown = data;
  try {
    parsed = JSON.parse(data) as unknown;
  } catch {
    fedLines.value = [...fedLines.value.slice(-99), data];
    return;
  }
  fedLines.value = [...fedLines.value.slice(-99), JSON.stringify(parsed, null, 2)];

  const envelope = parsed as { payload?: Record<string, unknown> };
  const payload = envelope.payload;
  if (!payload || typeof payload !== "object") return;
  const kind = String(payload.kind ?? "");
  const taskId = String(payload.task_id ?? "");
  if (!runTaskId.value || taskId !== runTaskId.value) return;

  if (kind === "task_progress") {
    const phase = String(payload.phase ?? "");
    const step = String(payload.step ?? "");
    const stepKind = String(payload.step_kind ?? "");
    const output = String(payload.output ?? "");
    if (phase === "step_complete" && step) {
      runTimeline.value = [
        ...runTimeline.value,
        `${step} (${stepKind || "agent"}) -> ${output || "(no output path)"}`,
      ];
    } else {
      runTimeline.value = [...runTimeline.value, `phase: ${phase}`];
    }
  } else if (kind === "task_result") {
    runResult.value = String(payload.outcome ?? "(no outcome)");
    const ok = Boolean(payload.success);
    runTimeline.value = [...runTimeline.value, ok ? "done: success" : "done: failed"];
    runBusy.value = false;
    clearRunWatchdog();
  }
}

function fedConnect() {
  fedDisconnect();
  fedLines.value = [];
  fedEs.value = openFederationEventSource(fedTopic.value, processFederationEvent);
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

async function loadWorkerProfiles() {
  workerProfilesErr.value = null;
  try {
    const r = await api.federationWorkers();
    workerProfiles.value = r.profiles ?? [];
  } catch (e) {
    workerProfilesErr.value = e instanceof Error ? e.message : String(e);
    workerProfiles.value = [];
  }
}

async function startWorker(profileId: string) {
  workerBusy.value = profileId;
  fedErr.value = null;
  try {
    await api.federationWorkerStart(profileId);
    await Promise.all([loadWorkerProfiles(), loadFederationRegistry()]);
  } catch (e) {
    fedErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    workerBusy.value = null;
  }
}

async function stopWorker(profileId: string) {
  workerBusy.value = profileId;
  fedErr.value = null;
  try {
    await api.federationWorkerStop(profileId);
    await Promise.all([loadWorkerProfiles(), loadFederationRegistry()]);
  } catch (e) {
    fedErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    workerBusy.value = null;
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

async function runKnowledgeCompiler() {
  const built = buildInstructionFromPrompt(runPrompt.value);
  if (!built) {
    fedErr.value = "Prompt must include a URL (e.g. https://yarenty.com).";
    return;
  }
  fedErr.value = null;
  runBusy.value = true;
  runResult.value = null;
  runTimeline.value = [];
  clearRunWatchdog();
  runTaskId.value = `ui-kc-${Date.now()}`;
  if (!fedEs.value) fedConnect();

  try {
    const out = await api.federationDelegate({
      task_id: runTaskId.value,
      instruction: built.instruction,
      capability: "kc.run",
    });
    runTimeline.value = [
      ...runTimeline.value,
      `delegated to: ${out.delegated_to ?? "(no target)"}`,
      `source: ${built.source}`,
    ];
    if (!out.delegated_to) {
      runBusy.value = false;
      runResult.value =
        "No federation worker matched capability `kc.run`.\n\nHow to fix:\n- Start worker: cargo run -p kowalski-cli -- extension run knowledge-compiler worker kc-worker-1\n- Click Refresh registry and verify an agent with `kc.run` capability is active.\n- Retry this run.";
      runTimeline.value = [
        ...runTimeline.value,
        "blocked: no target worker available for capability kc.run",
      ];
      return;
    }
    runWatchdog.value = window.setTimeout(() => {
      if (!runBusy.value) return;
      runBusy.value = false;
      runTimeline.value = [
        ...runTimeline.value,
        "timeout: no progress events received within 30s",
      ];
      runResult.value =
        "Run was delegated but no worker progress arrived.\n\nPotential causes:\n- Worker process is down or subscribed to another topic.\n- Worker cannot publish progress/result events.\n\nHow to fix:\n- Ensure worker is running with default topic `federation`.\n- Check server logs and worker terminal output.\n- Retry run.";
    }, 30_000);
    void loadFederationRegistry();
  } catch (e) {
    runBusy.value = false;
    clearRunWatchdog();
    fedErr.value = e instanceof Error ? e.message : String(e);
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
  void loadWorkerProfiles();
});

onUnmounted(() => {
  fedDisconnect();
  clearRunWatchdog();
});
</script>

<template>
  <section class="panel">
    <h2>Federation</h2>
    <p><button type="button" class="primary" @click="loadFederationRegistry">Refresh registry</button></p>
    <h3>Worker Profiles (UI-managed)</h3>
    <p class="muted">
      Worker = runtime process registered in federation by capability. Sub-agents are internal pipeline steps and are not required to be separate federation workers.
    </p>
    <div v-if="workerProfiles.length" class="cards">
      <article v-for="p in workerProfiles" :key="p.id" class="card">
        <header>
          <strong>{{ p.name }}</strong>
          <span class="status-badge" :class="p.managed_running ? 'status-ok' : 'status-off'">
            {{ p.managed_running ? "RUNNING" : "STOPPED" }}
          </span>
        </header>
        <p class="muted">{{ p.description }}</p>
        <p class="muted">Capability: {{ p.capability }}</p>
        <p class="muted">Registry agents: {{ p.registry_agents.join(", ") || "(none)" }}</p>
        <p class="muted">Command: {{ p.command }} {{ p.args.join(" ") }}</p>
        <p class="muted">Cwd: {{ p.cwd }}</p>
        <p>
          <button
            type="button"
            class="primary"
            :disabled="workerBusy === p.id || p.managed_running"
            @click="startWorker(p.id)"
          >
            {{ workerBusy === p.id ? "Starting..." : "Start" }}
          </button>
          <button
            type="button"
            :disabled="workerBusy === p.id || !p.managed_running"
            @click="stopWorker(p.id)"
          >
            {{ workerBusy === p.id ? "Stopping..." : "Stop" }}
          </button>
        </p>
      </article>
    </div>
    <p v-else class="muted">No worker profiles found.</p>
    <p v-if="workerProfilesErr" class="err">{{ workerProfilesErr }}</p>
    <p class="muted" v-if="knowledgeCompilerAgents.length">
      Knowledge Compiler agents: {{ knowledgeCompilerAgents.map((a) => a.id).join(", ") }}
    </p>
    <p class="muted" v-else>No active `knowledge-compiler`/`kc.run` workers in registry.</p>
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

    <h3>Knowledge Compiler (chat-style run)</h3>
    <p class="muted">
      Send one prompt (with URL), then follow step-by-step sub-agent outputs and final artifacts below.
    </p>
    <p>
      <label class="lbl">Prompt</label>
      <input
        v-model="runPrompt"
        class="inp"
        type="text"
        placeholder="can you check https://yarenty.com and get summary into obsidian?"
      />
    </p>
    <p>
      <button type="button" class="primary" :disabled="runBusy" @click="runKnowledgeCompiler">
        {{ runBusy ? "Running..." : "Run knowledge-compiler" }}
      </button>
    </p>
    <p v-if="runTaskId" class="muted">Task ID: {{ runTaskId }}</p>
    <div v-if="runTimeline.length" class="fed-events">
      <details v-for="(line, i) in runTimeline" :key="`run-${i}`" class="fed-event" :open="i >= runTimeline.length - 6">
        <summary>Step {{ i + 1 }}</summary>
        <pre class="json fed-line">{{ line }}</pre>
      </details>
    </div>
    <p v-else class="muted">(no run steps yet)</p>
    <details v-if="runResult">
      <summary>Final delivery</summary>
      <pre class="json fed-line">{{ runResult }}</pre>
    </details>

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
.status-off { border-color: #555f74; color: #b0b7c7; background: #2a3142; }
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
