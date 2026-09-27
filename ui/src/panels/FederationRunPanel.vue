<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import DOMPurify from "dompurify";
import { marked } from "marked";
import {
  api,
  openFederationEventSource,
  type FederationWorkerProfile,
  type HordeCatalogItem,
  type HordeRunRecord,
  type HordeTriggerStatus,
} from "../api";
import HordeRunForm from "../components/HordeRunForm.vue";
import PenguinAvatar from "../components/PenguinAvatar.vue";
import PipelineStepper, { type StepState, type StepperItem } from "../components/PipelineStepper.vue";
import { inferPenguinAvatarId } from "../penguins";
import { isDagHorde } from "../hordeGraph";
const props = defineProps<{ activeThreadId: string | null }>();
const emit = defineEmits<{
  (e: "new-chat-session"): void;
  (e: "open-build"): void;
  (e: "thread-upsert", item: { id: string; title: string; updatedAt: number }): void;
  (e: "new-thread-from-suggestion", payload: { prompt: string; hordeId: string }): void;
  (e: "thread-create-from-run", payload: {
    title: string;
    snapshot: {
      selectedHordeId: string;
      runId: string | null;
      runMessages: Array<{ role: "orchestrator" | "worker" | "system" | "user"; speaker: string; text: string }>;
      runResult: string | null;
      followupMsgs: Array<{ role: "user" | "assistant" | "orchestrator"; speaker: string; text: string }>;
      followupInput: string;
    };
  }): void;
}>();

const fedTopic = ref("federation");
const fedEs = ref<EventSource | null>(null);
const hordes = ref<HordeCatalogItem[]>([]);
const selectedHordeId = ref<string>("");
const runBusy = ref(false);
const runId = ref<string | null>(null);
const runMessages = ref<
  Array<{ role: "orchestrator" | "worker" | "system" | "user"; speaker: string; text: string; step?: string; at?: number }>
>([]);
/** Live per-step state from federation events for the current run (overlays run history). */
const stepLive = ref<Record<string, StepState>>({});
const runResult = ref<string | null>(null);
const runErr = ref<string | null>(null);
const runWatchdog = ref<number | null>(null);
const workerProfiles = ref<FederationWorkerProfile[]>([]);
const runHistory = ref<HordeRunRecord[]>([]);
const followupInput = ref("");
const followupBusy = ref(false);
const followupMsgs = ref<Array<{ role: "user" | "assistant" | "orchestrator"; speaker: string; text: string }>>([]);
const pathAction = ref<string | null>(null);
const cleanWorkdirBusy = ref(false);
const runPromotedToHistory = ref(false);
const triggerRows = ref<HordeTriggerStatus[]>([]);
const triggerBusy = ref<number | null>(null);
const triggerNote = ref<string | null>(null);
const highlightRunId = ref<string | null>(null);

const selectedHorde = computed(() => hordes.value.find((h) => h.id === selectedHordeId.value) ?? null);
const selectedHordeIsDag = computed(() => {
  const h = selectedHorde.value;
  if (!h) return false;
  return isDagHorde(h.pipeline, h.edges ?? []);
});
const selectedHordeWorkers = computed(() => workerProfiles.value.filter((w) => w.horde_id === selectedHordeId.value));
const resumableRuns = computed(() => runHistory.value.filter((r) => r.resumable));
const resumeBusyId = ref<string | null>(null);
/** A command step waiting for the operator (`approval_required`): what it would do. */
type PendingApproval = { runId: string; step: string; text: string; command: string | null };
const approval = ref<PendingApproval | null>(null);
const approvalBusy = ref(false);

/** The approval a parked run is waiting on, from its latest `approval_required` event. */
function approvalFromRun(r: HordeRunRecord): PendingApproval | null {
  if (r.status !== "awaiting_input") return null;
  const ev = [...(r.events ?? [])].reverse().find((e) => e.kind === "approval_required");
  if (!ev) return null;
  return {
    runId: r.run_id,
    step: String(ev.step ?? ""),
    text: String(ev.text ?? "A step is waiting for your approval."),
    command: ev.command != null ? String(ev.command) : null,
  };
}
/**
 * The approval shown in the big box: the live one, else the first parked run of this horde
 * that waits before a command step.
 */
const shownApproval = computed(
  (): PendingApproval | null =>
    approval.value ?? resumableRuns.value.map(approvalFromRun).find((a) => a !== null) ?? null,
);
/** Interrupted runs listed in the calm banner (the one in the approval box is not repeated). */
const bannerRuns = computed(() => resumableRuns.value.filter((r) => r.run_id !== shownApproval.value?.runId));
const activeRunFromHistory = computed(() =>
  runId.value ? runHistory.value.find((r) => r.run_id === runId.value) ?? null : null,
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
/** Tool ids served by tableski (SQL over the operator's spreadsheets). */
const TABLESKI_TOOLS = new Set(["list_tables", "get_schema", "column_statistics", "query_sql"]);
const TABLESKI_KINDS = new Set(["table_profile", "sql_batch"]);

/** Picker summary: what a horde needs and what it hands back (display only). */
function hordeBrief(h: HordeCatalogItem): { needs: string[]; delivers: string[]; triggers: string[] } {
  const needs = new Set<string>();
  const delivers: string[] = [];
  for (const a of h.sub_agents) {
    if (TABLESKI_KINDS.has(a.kind)) needs.add("tableski");
    for (const t of a.tool_ids ?? []) needs.add(TABLESKI_TOOLS.has(t) ? "tableski" : t);
    const out = (a.output ?? "").trim();
    if (out && !out.startsWith("debug/") && !out.endsWith("/")) delivers.push(out);
  }
  const triggers = (h.triggers ?? []).map((t) => t.kind);
  return { needs: [...needs], delivers, triggers };
}

function subAgentFor(step: string) {
  return selectedHorde.value?.sub_agents.find((a) => a.name === step) ?? null;
}

function historyStepState(status: string | undefined): StepState | null {
  switch (status) {
    case "success":
    case "succeeded":
      return "done";
    case "failed":
      return "failed";
    case "running":
    case "delegating":
      return "running";
    case "cancelled":
      return "cancelled";
    case "skipped":
      return "skipped";
    case "pending":
      return "pending";
    default:
      return null;
  }
}

/** Pipeline stepper: live federation events first, then the stored run record. */
const stepperItems = computed((): StepperItem[] => {
  const h = selectedHorde.value;
  if (!h) return [];
  const record = activeRunFromHistory.value;
  const waitingStep =
    shownApproval.value && shownApproval.value.runId === runId.value ? shownApproval.value.step : null;
  return h.pipeline.map((name) => {
    const agent = subAgentFor(name);
    let state: StepState =
      stepLive.value[name] ??
      historyStepState(record?.steps.find((s) => s.step === name)?.status) ??
      "pending";
    if (waitingStep === name) state = "waiting";
    return { name, label: agent?.display_name || titleCase(name), kind: agent?.kind, state };
  });
});
const stepperHeadline = computed(() => {
  const items = stepperItems.value;
  if (!items.length) return "";
  const cur = items.findIndex((s) => s.state === "running" || s.state === "waiting" || s.state === "failed");
  if (cur >= 0) return `Step ${cur + 1} of ${items.length}`;
  const done = items.filter((s) => s.state === "done").length;
  if (done === items.length) return `All ${items.length} steps done`;
  if (done > 0) return `${done} of ${items.length} steps done`;
  return `${items.length} steps`;
});
const hasRunActivity = computed(() => runMessages.value.length > 0 || !!runId.value);

/** The file the run delivered: the last step's artifact (e.g. HANDOFF.md, report.xlsx, BRIEF.md). */
const primaryArtifact = computed((): { step: string; path: string; name: string } | null => {
  const list = finalArtifacts.value;
  if (!list.length) return null;
  const [step, path] = list[list.length - 1];
  return { step, path, name: path.split(/[\\/]/).pop() || path };
});
const otherArtifacts = computed(() => finalArtifacts.value.slice(0, -1));

function isAbsolutePath(p: string): boolean {
  return p.startsWith("/") || /^[A-Za-z]:[\\/]/.test(p);
}

function clockTime(ts?: number): string {
  if (!ts) return "";
  return new Date(ts).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

/** One readable line from a run prompt (drops the markdown operator-input heading and emphasis). */
function promptLine(text?: string | null): string {
  const lines = (text ?? "")
    .split("\n")
    .map((l) => l.replace(/\*\*/g, "").trim())
    .filter((l) => l && !l.startsWith("#"));
  return lines.join(" · ");
}

function speakerLabel(speaker: string): string {
  return speaker.replace(/^Agent:\s*/, "");
}

function roleTag(role: string): string {
  if (role === "orchestrator") return "Boss";
  if (role === "worker") return "Penguin";
  if (role === "user") return "You";
  return "System";
}

function runStatusBadge(status: string): string {
  if (status === "completed") return "badge-ok";
  if (status === "running" || status === "pending") return "badge-red badge-running";
  if (status === "awaiting_input" || status === "failed") return "badge-red";
  return "badge-muted";
}

const progressText = ref("idle");
const copyPasteErr = ref<string | null>(null);
async function copyPasteToClipboard() {
  copyPasteErr.value = null;
  const t = handoffMarkdown.value;
  if (!t) {
    copyPasteErr.value = "Nothing to copy.";
    return;
  }
  try {
    await navigator.clipboard.writeText(t);
  } catch {
    copyPasteErr.value = "Clipboard failed — select the text in the box and copy manually (Cmd/Ctrl+C).";
  }
}
const finalShortSummary = computed(() => selectedHorde.value?.delivery_summary_note || "Run completed.");
const deliveryNoteHtml = computed(() =>
  DOMPurify.sanitize(marked.parseInline(selectedHorde.value?.delivery_note ?? "") as string),
);
/** The hand-off rendered for reading (sanitised; the raw Markdown stays one click away). */
const handoffHtml = computed(() =>
  handoffMarkdown.value ? DOMPurify.sanitize(marked.parse(handoffMarkdown.value, { gfm: true }) as string) : "",
);
const handoffMarkdown = computed(() => {
  if (!runResult.value) return "";
  try {
    const p = JSON.parse(runResult.value) as {
      handoff_markdown?: string;
      paste_for_obsidian?: string;
    };
    if (typeof p.handoff_markdown === "string") return p.handoff_markdown;
    if (typeof p.paste_for_obsidian === "string") return p.paste_for_obsidian;
    return "";
  } catch {
    return "";
  }
});
const hasCompletedRun = computed(() => runCompleted.value);
const isProcessing = computed(() => runBusy.value || followupBusy.value);
const processingLabel = computed(() =>
  followupBusy.value ? "Processing follow-up..." : `Horde is processing... ${progressText.value}`,
);

watch(
  () => selectedHordeId.value,
  async () => {
    await Promise.all([loadProfiles(), loadRunHistory(), loadTriggers()]);
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

function feed(
  role: "orchestrator" | "worker" | "system" | "user",
  text: string,
  speaker?: string,
  step?: string,
) {
  runMessages.value = [
    ...runMessages.value,
    {
      role,
      speaker: speaker || (role === "orchestrator" ? "Agent: Boss" : "System"),
      text,
      ...(step ? { step } : {}),
      at: Date.now(),
    },
  ];
}

function avatarForRunMessage(m: {
  role: string;
  step?: string;
}): { avatar: string; kind: string; name: string } | null {
  if (m.role === "orchestrator") {
    return { avatar: "director", kind: "orchestrator", name: "boss" };
  }
  if (m.role === "worker" && m.step) {
    const agent = selectedHorde.value?.sub_agents.find((a) => a.name === m.step);
    return {
      avatar: agent?.avatar ?? inferPenguinAvatarId(agent?.kind ?? "process", m.step),
      kind: agent?.kind ?? "process",
      name: m.step,
    };
  }
  return null;
}

function extractUrl(input: string): string | null {
  const m = input.match(/https?:\/\/\S+/);
  return m ? m[0] : null;
}

function extractUrls(input: string): string[] {
  const matches = input.match(/https?:\/\/\S+/g) ?? [];
  const cleaned = matches.map((u) => u.trim().replace(/[),.;]+$/, ""));
  return [...new Set(cleaned)];
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
    setStep(step, "running");
    feed("orchestrator", `${step} assigned to ${String(payload.to ?? "?")}`, "Agent: Boss");
  } else if (kind === "task_started") {
    const step = String(payload.step ?? "?");
    progressText.value = `${step} running`;
    setStep(step, "running");
    feed("worker", `${step} started by ${String(payload.agent ?? "?")}`, speakerNameFromStep(step), step);
  } else if (kind === "agent_message") {
    const step = String(payload.step ?? "");
    feed("worker", String(payload.text ?? "(message)"), speakerNameFromStep(step || "worker"), step || undefined);
  } else if (kind === "task_finished") {
    const step = String(payload.step ?? "?");
    const artifact = String(payload.artifact ?? "");
    const ok = Boolean(payload.success);
    const outcome = payload.outcome != null ? String(payload.outcome) : "";
    progressText.value = ok ? `${step} completed` : `${step} failed`;
    setStep(step, ok ? "done" : "failed");
    const outcomeNote = outcome ? ` (outcome: ${outcome})` : "";
    feed(
      "worker",
      ok
        ? `${step} completed${outcomeNote}${artifact ? ` -> ${artifact}` : ""}`
        : `${step} failed${outcomeNote}${payload.summary ? `: ${String(payload.summary)}` : ""}`,
      speakerNameFromStep(step),
      step,
    );
  } else if (kind === "step_routed") {
    const fromStep = String(payload.from_step ?? "?");
    const nextStep = String(payload.next_step ?? "?");
    const outcome = String(payload.outcome ?? "?");
    const isLoop = Boolean(payload.is_loop_back);
    const loopCount = payload.loop_count != null ? Number(payload.loop_count) : null;
    const verifyExcerpt =
      typeof payload.verify_excerpt === "string" ? payload.verify_excerpt.trim() : "";
    const branch = isLoop
      ? `retry loop → \`${nextStep}\`${loopCount != null ? ` (loop ${loopCount})` : ""}`
      : `branch → \`${nextStep}\``;
    progressText.value = `${fromStep} ${outcome} → ${nextStep}`;
    feed(
      "orchestrator",
      `${fromStep} outcome **${outcome}**: ${branch}`,
      "Agent: Boss",
      fromStep,
    );
    if (verifyExcerpt) {
      feed(
        "worker",
        verifyExcerpt,
        speakerNameFromStep(fromStep),
        fromStep,
      );
    }
  } else if (kind === "run_finished") {
    runResult.value = JSON.stringify(payload, null, 2);
    progressText.value = "finished";
    feed("system", "run finished", "System");
    runBusy.value = false;
    if (!props.activeThreadId && !runPromotedToHistory.value) {
      runPromotedToHistory.value = true;
      emit("thread-create-from-run", {
        title: titleFromCurrentRun(),
        snapshot: buildSnapshot(),
      });
    }
    clearRunWatchdog();
    void loadRunHistory();
  } else if (kind === "run_failed") {
    runResult.value = JSON.stringify(payload, null, 2);
    progressText.value = "failed";
    settleSteps("failed");
    feed("system", `run failed${payload.reason ? `: ${String(payload.reason)}` : ""}`, "System");
    runBusy.value = false;
    clearRunWatchdog();
    void loadRunHistory();
  } else if (kind === "approval_required") {
    const step = String(payload.step ?? "?");
    approval.value = {
      runId: evRunId || runId.value || "",
      step,
      text: String(payload.text ?? "A step is waiting for your approval."),
      command: payload.command != null ? String(payload.command) : null,
    };
    progressText.value = `${step} waiting for your approval`;
    setStep(step, "waiting");
    feed("orchestrator", String(payload.text ?? `${step} waits for approval`), "Agent: Boss", step);
    runBusy.value = false;
    clearRunWatchdog();
    void loadRunHistory();
  } else if (kind === "run_cancelled") {
    progressText.value = "cancelled";
    settleSteps("cancelled");
    feed("system", `run cancelled${payload.reason ? `: ${String(payload.reason)}` : ""}`, "System");
    runBusy.value = false;
    clearRunWatchdog();
    void loadRunHistory();
  }
}

function setStep(step: string, state: StepState) {
  if (!step || step === "?") return;
  stepLive.value = { ...stepLive.value, [step]: state };
}

/** Run ended early: whatever was still in flight takes the final state. */
function settleSteps(state: StepState) {
  const next = { ...stepLive.value };
  for (const [k, v] of Object.entries(next)) {
    if (v === "running" || v === "waiting") next[k] = state;
  }
  stepLive.value = next;
}

function connectStream() {
  fedEs.value?.close();
  fedEs.value = openFederationEventSource(fedTopic.value, processFederationEvent);
}

const hordesLoaded = ref(false);
async function loadHordes() {
  const res = await api.hordes();
  hordes.value = res.hordes ?? [];
  hordesLoaded.value = true;
  if (!selectedHordeId.value && hordes.value.length) {
    // `?horde=<id>` deep link, else the first horde.
    const linked = new URLSearchParams(window.location.search).get("horde");
    selectedHordeId.value = hordes.value.find((h) => h.id === linked)?.id ?? hordes.value[0].id;
  }
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
  await Promise.all([loadProfiles(), loadRunHistory(), loadTriggers()]);
}

async function loadTriggers() {
  if (!selectedHordeId.value) {
    triggerRows.value = [];
    return;
  }
  try {
    const r = await api.hordeTriggers(selectedHordeId.value);
    triggerRows.value = r.triggers ?? [];
  } catch {
    triggerRows.value = [];
  }
}

async function toggleTrigger(t: HordeTriggerStatus) {
  if (!selectedHordeId.value || triggerBusy.value !== null) return;
  triggerBusy.value = t.index;
  triggerNote.value = null;
  try {
    const r = await api.hordeTriggerSetEnabled(selectedHordeId.value, t.index, !t.effective_enabled);
    triggerRows.value = r.triggers ?? [];
    triggerNote.value = `Trigger "${t.detail}" ${t.effective_enabled ? "disabled" : "enabled"} (persisted server-side; horde.md is untouched).`;
  } catch (e) {
    triggerNote.value = e instanceof Error ? e.message : String(e);
  } finally {
    triggerBusy.value = null;
  }
}

async function fireTriggerNow(t: HordeTriggerStatus) {
  if (!selectedHordeId.value || triggerBusy.value !== null) return;
  triggerBusy.value = t.index;
  triggerNote.value = null;
  try {
    const r = await api.hordeTriggerFire(selectedHordeId.value, t.index);
    if (r.fired && r.run) {
      triggerNote.value = `Fired: run ${r.run.run_id} started.`;
      highlightRunId.value = r.run.run_id;
    } else if (r.skipped) {
      triggerNote.value = `Not fired: run ${r.active_run_id ?? "?"} from this trigger is still in flight (overlap=skip).`;
      highlightRunId.value = r.active_run_id ?? null;
    } else if (r.queued) {
      triggerNote.value = "Queued behind the in-flight run — fires when it ends (overlap=queue).";
    }
  } catch (e) {
    triggerNote.value = e instanceof Error ? e.message : String(e);
  } finally {
    triggerBusy.value = null;
    await Promise.all([loadTriggers(), loadRunHistory()]);
  }
}

/** Feed badge: `trigger:<kind>:<horde>` sources → the trigger kind; anything else is an operator run. */
function runSourceBadge(r: HordeRunRecord): { label: string; isTrigger: boolean } {
  const m = /^trigger:([a-z]+):/.exec(r.source ?? "");
  return m ? { label: m[1], isTrigger: true } : { label: "operator", isTrigger: false };
}

function shortTime(iso?: string | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
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

async function cleanSelectedWorkdir() {
  if (!selectedHordeId.value) return;
  cleanWorkdirBusy.value = true;
  pathAction.value = null;
  try {
    const r = await api.hordeCleanWorkdir(selectedHordeId.value);
    pathAction.value = `Workdir cleaned: ${r.workdir}`;
    emit("new-chat-session");
  } catch (e) {
    pathAction.value = e instanceof Error ? e.message : String(e);
  } finally {
    cleanWorkdirBusy.value = false;
  }
}

function isWorkerReady(w: FederationWorkerProfile): boolean {
  return Boolean(w.managed_running && w.registered_exact && !w.stale_registration);
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function ensureSelectedHordeReady() {
  if (!selectedHordeId.value) return false;
  const maxAttempts = 8;
  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    await loadProfiles();
    const workers = selectedHordeWorkers.value;
    if (workers.length > 0 && workers.every((w) => isWorkerReady(w))) {
      return true;
    }
    const notReady = workers.filter((w) => !isWorkerReady(w));
    for (const w of notReady) {
      await api.hordeWorkersStart(selectedHordeId.value, w.step);
    }
    await sleep(650);
  }
  await loadProfiles();
  return selectedHordeWorkers.value.length > 0 && selectedHordeWorkers.value.every((w) => isWorkerReady(w));
}

const RUN_HISTORY_KEY = "kowalski.ui.horde-runs.v1";
function threadStateKey(id: string) {
  return `kowalski.ui.horde.thread.${id}`;
}

function buildSnapshot() {
  return {
    selectedHordeId: selectedHordeId.value,
    runId: runId.value,
    runMessages: runMessages.value,
    runResult: runResult.value,
    followupMsgs: followupMsgs.value,
    followupInput: followupInput.value,
  };
}

function titleFromCurrentRun(): string {
  const firstRunPrompt = runMessages.value.find((m) => m.role === "user")?.text;
  const lastUser = [...followupMsgs.value].reverse().find((m) => m.role === "user")?.text;
  return (
    lastUser?.slice(0, 42) ||
    firstRunPrompt?.slice(0, 42) ||
    runMessages.value.find((m) => m.speaker === "Agent: Boss" && m.text.startsWith("source:"))?.text.replace("source: ", "") ||
    "Horde interaction"
  );
}

function resetDraftState() {
  runId.value = null;
  runMessages.value = [];
  runResult.value = null;
  runErr.value = null;
  followupMsgs.value = [];
  followupInput.value = "";
  progressText.value = "idle";
  runPromotedToHistory.value = false;
  stepLive.value = {};
}

function saveActiveThreadState() {
  if (!props.activeThreadId) return;
  localStorage.setItem(threadStateKey(props.activeThreadId), JSON.stringify(buildSnapshot()));
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
      runMessages?: Array<{ role: "orchestrator" | "worker" | "system" | "user"; speaker: string; text: string }>;
      runResult?: string | null;
      followupMsgs?: Array<{ role: "user" | "assistant" | "orchestrator"; speaker: string; text: string }>;
      followupInput?: string;
    };
    if (parsed.selectedHordeId) selectedHordeId.value = parsed.selectedHordeId;
    runId.value = parsed.runId ?? null;
    runMessages.value = parsed.runMessages ?? [];
    runResult.value = parsed.runResult ?? null;
    followupMsgs.value = parsed.followupMsgs ?? [];
    followupInput.value = parsed.followupInput ?? "";
  } catch {
    runId.value = null;
    runMessages.value = [];
    runResult.value = null;
    followupMsgs.value = [];
    followupInput.value = "";
  }
}

function upsertThreadMeta() {
  if (!props.activeThreadId) return;
  const firstRunPrompt = runMessages.value.find((m) => m.role === "user")?.text;
  const lastUser = [...followupMsgs.value].reverse().find((m) => m.role === "user")?.text;
  const title =
    lastUser?.slice(0, 42) ||
    firstRunPrompt?.slice(0, 42) ||
    runMessages.value.find((m) => m.speaker === "Agent: Boss" && m.text.startsWith("source:"))?.text.replace("source: ", "") ||
    "New horde interaction";
  emit("thread-upsert", {
    id: props.activeThreadId,
    title,
    updatedAt: Date.now(),
  });
}

function suggestedPromptFromConversation(): string {
  const source =
    runMessages.value.find((m) => m.speaker === "Agent: Boss" && m.text.startsWith("source:"))?.text.replace("source:", "").trim() ||
    activeRunFromHistory.value?.source?.trim() ||
    extractUrl(activeRunFromHistory.value?.prompt ?? "") ||
    "";
  const latestUserFocus =
    [...followupMsgs.value].reverse().find((m) => m.role === "user")?.text?.trim() ||
    activeRunFromHistory.value?.question?.trim() ||
    "summarize key findings and practical improvements";
  const base = source ? `Analyze ${source}.` : "Analyze the provided source URL.";
  return `${base} Focus on: ${latestUserFocus}. Produce a structured summary and clear action points.`;
}

function redefineAndStartAgain() {
  if (!selectedHordeId.value) return;
  emit("new-thread-from-suggestion", {
    prompt: suggestedPromptFromConversation(),
    hordeId: selectedHordeId.value,
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
  persistRunHistory();
}

watch(
  () => props.activeThreadId,
  (id) => {
    if (!id) {
      resetDraftState();
      return;
    }
    runPromotedToHistory.value = false;
    loadActiveThreadState(id);
  },
  { immediate: true },
);
watch(
  [selectedHordeId, runId, runResult, runMessages, followupMsgs],
  () => {
    saveActiveThreadState();
  },
  { deep: true },
);

async function runHordeWithPayload(payload: {
  prompt: string;
  source: string;
  question: string;
  formAnswers?: Record<string, string>;
}) {
  await Promise.all([loadHordes(), loadProfiles()]);
  const prompt = payload.prompt.trim();
  const hasFormAnswers = !!payload.formAnswers && Object.keys(payload.formAnswers).length > 0;
  if (!prompt && !hasFormAnswers) {
    runErr.value = "Source URL or text is required.";
    return;
  }
  const sources = extractUrls(prompt);
  runErr.value = null;
  // Steps with an in-process handler (verify/apply/ingest) need no worker; a horde
  // may legitimately list zero worker profiles.
  if (selectedHordeWorkers.value.length) {
    progressText.value = "ensuring workers are ready";
    const ready = await ensureSelectedHordeReady();
    if (!ready) {
      const missing = selectedHordeWorkers.value
        .filter((w) => !isWorkerReady(w))
        .map((w) => w.step || w.agent_id)
        .join(", ");
      runErr.value = `Some sub-agents are still unavailable: ${missing || "unknown"}.`;
      return;
    }
  }
  runResult.value = null;
  followupMsgs.value = [];
  runBusy.value = true;
  progressText.value = "starting";
  runPromotedToHistory.value = false;
  runId.value = null;
  runMessages.value = [];
  stepLive.value = {};
  clearRunWatchdog();
  connectStream();
  feed("user", prompt || "(operator form submitted)", "You");
  upsertThreadMeta();
  feed("orchestrator", "creating run", "Agent: Boss");
  if (sources.length) {
    feed("orchestrator", `source(s): ${sources.join(", ")}`, "Agent: Boss");
  }
  try {
    const out = await api.hordeRun(selectedHordeId.value, {
      prompt,
      source: payload.source,
      question: payload.question,
      form_answers: payload.formAnswers,
    });
    runId.value = out.run.run_id;
    feed("orchestrator", `run started: ${out.run.run_id}`, "Agent: Boss");
    runWatchdog.value = window.setTimeout(() => {
      if (!runBusy.value) return;
      runBusy.value = false;
      progressText.value = "timeout";
      feed("system", "timeout: no progress events within 60s", "System");
      runResult.value =
        "Run created, but no worker progress arrived in 60s. Check sub-agent worker status under Admin → Federation.";
    }, 60_000);
  } catch (e) {
    runBusy.value = false;
    progressText.value = "failed";
    runErr.value = e instanceof Error ? e.message : String(e);
  }
}

const cancelBusy = ref(false);
async function cancelActiveRun() {
  if (!selectedHordeId.value || !runId.value || cancelBusy.value) return;
  cancelBusy.value = true;
  try {
    await api.hordeRunCancel(selectedHordeId.value, runId.value);
    // The run_cancelled feed event flips runBusy and refreshes history.
  } catch (e) {
    runErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    cancelBusy.value = false;
  }
}

async function approvePending(pending: PendingApproval) {
  if (!selectedHordeId.value || approvalBusy.value) return;
  approvalBusy.value = true;
  runErr.value = null;
  try {
    if (runId.value !== pending.runId) {
      resetDraftState();
      runId.value = pending.runId;
    }
    connectStream();
    runBusy.value = true;
    progressText.value = `${pending.step} approved`;
    feed("user", `Approved \`${pending.step}\``, "You");
    await api.hordeRunApprove(selectedHordeId.value, pending.runId);
    approval.value = null;
  } catch (e) {
    runBusy.value = false;
    runErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    approvalBusy.value = false;
    void loadRunHistory();
  }
}

async function rejectPending(pending: PendingApproval) {
  if (!selectedHordeId.value || approvalBusy.value) return;
  approvalBusy.value = true;
  try {
    await api.hordeRunCancel(selectedHordeId.value, pending.runId);
    approval.value = null;
    feed("system", `run cancelled before \`${pending.step}\``, "System");
  } catch (e) {
    runErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    approvalBusy.value = false;
    void loadRunHistory();
  }
}

async function resumeInterruptedRun(run: { run_id: string; prompt: string }) {
  if (!selectedHordeId.value || resumeBusyId.value) return;
  resumeBusyId.value = run.run_id;
  runErr.value = null;
  progressText.value = "ensuring workers are ready";
  try {
    if (selectedHordeWorkers.value.length) {
      const ready = await ensureSelectedHordeReady();
      if (!ready) {
        runErr.value = "Sub-agent workers are not ready — start them under Admin → Federation, then resume again.";
        return;
      }
    }
    resetDraftState();
    runBusy.value = true;
    progressText.value = "resuming";
    connectStream();
    feed("user", run.prompt || "(interrupted run)", "You");
    feed("orchestrator", `resuming run ${run.run_id}`, "Agent: Boss");
    const out = await api.hordeRunResume(selectedHordeId.value, run.run_id);
    runId.value = out.run.run_id;
    runWatchdog.value = window.setTimeout(() => {
      if (!runBusy.value) return;
      runBusy.value = false;
      progressText.value = "timeout";
      feed("system", "timeout: no progress events within 60s after resume", "System");
    }, 60_000);
  } catch (e) {
    runBusy.value = false;
    progressText.value = "resume failed";
    runErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    resumeBusyId.value = null;
    void loadRunHistory();
  }
}

async function onHordeFormSubmit(payload: {
  prompt: string;
  source: string;
  question: string;
  formAnswers?: Record<string, string>;
}) {
  if (!hasCompletedRun.value) {
    await runHordeWithPayload(payload);
    return;
  }
  if (!selectedHordeId.value) return;
  if (!runId.value) {
    const fallbackRunId = runHistory.value.find((r) => r.status === "completed")?.run_id;
    if (fallbackRunId) runId.value = fallbackRunId;
  }
  if (!runId.value) return;
  const q = payload.prompt.trim();
  if (!q) return;
  followupMsgs.value = [...followupMsgs.value, { role: "user", speaker: "You", text: q }];
  upsertThreadMeta();
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

// The server catalog hot-reloads horde definitions (add/edit/remove without a
// restart); poll the listing so changes show up without a manual refresh.
let hordePollTimer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  restoreRunHistory();
  connectStream();
  void refreshAll();
  hordePollTimer = setInterval(() => {
    void loadHordes();
    void loadTriggers();
  }, 15000);
});

onUnmounted(() => {
  fedEs.value?.close();
  clearRunWatchdog();
  if (hordePollTimer) clearInterval(hordePollTimer);
});
</script>

<template>
  <section class="page hordes">
    <header class="page-head">
      <div>
        <p class="eyebrow">Hordes // {{ hordes.length }} on standby</p>
        <h1>Send in a horde</h1>
        <p class="lead">Pick a horde, tell it what you need, and watch the penguins work it step by step.</p>
      </div>
      <div class="page-head-actions">
        <button type="button" class="icon-btn" title="Refresh" aria-label="Refresh hordes and runs" @click="refreshAll">↻</button>
      </div>
    </header>

    <section v-if="shownApproval" class="approval-box" role="alert" aria-live="assertive">
      <p class="eyebrow plain approval-tag">Waiting for you // step {{ shownApproval.step }}</p>
      <h2>Approve before the horde continues</h2>
      <p class="approval-text">{{ shownApproval.text }}</p>
      <pre v-if="shownApproval.command" class="approval-cmd"><span class="prompt" aria-hidden="true">$ </span>{{ shownApproval.command }}</pre>
      <div class="btn-row">
        <button type="button" class="primary" :disabled="approvalBusy" @click="approvePending(shownApproval)">
          {{ approvalBusy ? "Working…" : "Approve and continue" }}
        </button>
        <button type="button" class="danger" :disabled="approvalBusy" @click="rejectPending(shownApproval)">Cancel run</button>
      </div>
      <p class="muted small approval-foot">Approval covers this step for the rest of the run.</p>
    </section>

    <section v-if="bannerRuns.length" class="resume-banner note note-warn">
      <h3>Interrupted runs</h3>
      <p class="small">
        These runs did not finish (server restart or awaiting input). Completed steps are kept —
        Resume continues from the next ready step.
      </p>
      <article v-for="r in bannerRuns" :key="r.run_id" class="resume-item">
        <div class="resume-meta">
          <span class="resume-prompt">{{ promptLine(r.prompt) || r.question || "(no prompt)" }}</span>
          <span class="mono muted tiny">
            {{ r.run_id }} · {{ r.status }}{{ (r.resume_count ?? 0) > 0 ? ` · ${r.resume_count} resume attempt(s)` : "" }}
          </span>
        </div>
        <template v-if="approvalFromRun(r)">
          <span class="resume-approval">{{ approvalFromRun(r)?.text }}</span>
          <div class="btn-row">
            <button
              type="button"
              class="primary sm"
              :disabled="approvalBusy || runBusy"
              @click="approvePending(approvalFromRun(r)!)"
            >
              Approve
            </button>
            <button type="button" class="sm" :disabled="approvalBusy" @click="rejectPending(approvalFromRun(r)!)">Cancel run</button>
          </div>
        </template>
        <button
          v-else
          type="button"
          class="sm"
          :disabled="resumeBusyId !== null || runBusy"
          @click="resumeInterruptedRun(r)"
        >
          {{ resumeBusyId === r.run_id ? "Resuming…" : "Resume" }}
        </button>
      </article>
    </section>

    <div class="hordes-layout">
      <aside class="picker" aria-label="Choose a horde">
        <p class="eyebrow">Pick a horde</p>
        <div v-if="hordes.length" class="picker-list" role="radiogroup" aria-label="Hordes">
          <button
            v-for="h in hordes"
            :key="h.id"
            type="button"
            role="radio"
            class="horde-card"
            :class="{ selected: h.id === selectedHordeId }"
            :aria-checked="h.id === selectedHordeId"
            @click="selectedHordeId = h.id"
          >
            <span class="hc-head">
              <span class="hc-name">{{ h.display_name }}</span>
              <span v-if="h.load_error" class="badge badge-warn no-dot" title="Latest edit failed to load">⚠ edit failed</span>
            </span>
            <span class="hc-desc">{{ h.description }}</span>
            <span class="hc-meta">
              <span class="chip" :title="`${h.pipeline.length} steps`">{{ h.pipeline.length }} penguins</span>
              <span v-for="n in hordeBrief(h).needs" :key="n" class="chip chip-steel" :title="`Needs ${n}`">needs {{ n }}</span>
              <span v-for="t in hordeBrief(h).triggers" :key="t" class="chip" :title="`Has a ${t} trigger`">{{ t }}</span>
            </span>
            <span v-if="hordeBrief(h).delivers.length" class="hc-delivers">
              → {{ hordeBrief(h).delivers.join(", ") }}
            </span>
          </button>
        </div>
        <p v-else-if="!hordesLoaded" class="muted">Loading hordes…</p>
        <div v-else class="empty-state">
          <h3>No hordes yet</h3>
          <p>Build your first horde by describing the job in plain words.</p>
          <button type="button" class="primary" @click="emit('open-build')">Build a horde</button>
        </div>
      </aside>

      <div class="mission">
        <article v-if="selectedHorde" class="card accent briefing">
          <p class="eyebrow plain">Squad // {{ selectedHorde.id }}</p>
          <h2>{{ selectedHorde.display_name }}</h2>
          <p class="briefing-desc">{{ selectedHorde.description }}</p>
          <p v-if="selectedHorde.load_error" class="note note-warn small">
            ⚠ Definition edit failed to load — running the last good version. {{ selectedHorde.load_error }}
          </p>
          <p v-if="selectedHordeIsDag" class="note note-info small">
            <strong>Branching horde.</strong> Steps that share a layer run when all earlier steps finish (they may run in
            parallel).
          </p>
          <ul class="squad" aria-label="Penguins in this horde">
            <li v-for="a in selectedHorde.sub_agents" :key="a.name" class="squad-member" :title="a.description">
              <PenguinAvatar
                :avatar="a.avatar ?? inferPenguinAvatarId(a.kind, a.name)"
                :kind="a.kind"
                :name="a.name"
                variant="inline"
                :alt="a.display_name || a.name"
              />
              <span class="sm-text">
                <span class="sm-name">{{ a.display_name || a.name }}</span>
                <span class="sm-kind">{{ a.kind }}</span>
              </span>
            </li>
          </ul>
          <details class="briefing-details">
            <summary>Output folder and housekeeping</summary>
            <div class="workdir-row">
              <span class="lbl">Output folder</span>
              <code>{{ selectedHorde.workdir || selectedHorde.root_path }}</code>
              <button type="button" class="sm" @click="openOutputFolder(selectedHorde.workdir || selectedHorde.root_path)">
                Open output folder
              </button>
            </div>
            <div class="workdir-row">
              <span class="lbl">Clean on startup</span>
              <strong>{{ (selectedHorde.config_on_startup_effective ?? selectedHorde.config_on_startup) ? "true" : "false" }}</strong>
              <button
                type="button"
                class="sm danger"
                :disabled="cleanWorkdirBusy"
                title="Delete workdir debug tree, legacy raw/wiki/scratch, agents_log, and PASTE_ME.md (same paths as server clean-on-startup)"
                @click="cleanSelectedWorkdir"
              >
                {{ cleanWorkdirBusy ? "…" : "FORCE Clean" }}
              </button>
            </div>
          </details>
        </article>

        <section v-if="selectedHorde && stepperItems.length && hasRunActivity" class="card flat progress-card">
          <div class="progress-head">
            <p class="eyebrow">Pipeline // {{ stepperHeadline }}</p>
            <span v-if="runId" class="mono muted tiny">run {{ runId }}</span>
          </div>
          <PipelineStepper :steps="stepperItems" />
          <div v-if="isProcessing" class="processing" aria-live="polite" aria-busy="true">
            <span class="dot-running" aria-hidden="true"></span>
            <span class="processing-text">{{ processingLabel }}</span>
            <button
              v-if="runBusy && runId"
              type="button"
              class="sm danger"
              :disabled="cancelBusy"
              title="Cancel this run: the in-flight step stops, remaining steps are skipped"
              @click="cancelActiveRun"
            >
              {{ cancelBusy ? "Cancelling…" : "Cancel run" }}
            </button>
          </div>
        </section>

        <section v-if="selectedHorde && !hasCompletedRun" class="card orders">
          <p class="eyebrow">Your request</p>
          <HordeRunForm
            :horde="selectedHorde"
            :disabled="!selectedHordeId"
            :busy="runBusy || followupBusy"
            :follow-up-mode="false"
            @submit="onHordeFormSubmit"
          />
        </section>

        <p v-if="runErr" class="note note-err">{{ runErr }}</p>

        <section v-if="runMessages.length" class="feed-wrap">
          <p class="eyebrow">Live feed</p>
          <ol class="feed">
            <li v-for="(m, i) in runMessages" :key="i" class="msg" :class="`msg-${m.role}`">
              <span class="msg-avatar" aria-hidden="true">
                <PenguinAvatar
                  v-if="avatarForRunMessage(m)"
                  :avatar="avatarForRunMessage(m)!.avatar"
                  :kind="avatarForRunMessage(m)!.kind"
                  :name="avatarForRunMessage(m)!.name"
                  variant="inline"
                  :alt="m.speaker"
                />
                <span v-else class="avatar-fallback">{{ m.role === "user" ? "You" : "•" }}</span>
              </span>
              <div class="msg-body">
                <header class="msg-head">
                  <span class="msg-speaker">{{ speakerLabel(m.speaker) }}</span>
                  <span class="msg-role">{{ roleTag(m.role) }}</span>
                  <time v-if="m.at" class="msg-time">{{ clockTime(m.at) }}</time>
                </header>
                <pre class="msg-text">{{ m.text }}</pre>
              </div>
            </li>
          </ol>
        </section>

        <section v-if="runCompleted && runResult" class="card delivery">
          <p class="eyebrow">Delivered</p>
          <div v-if="primaryArtifact" class="deliverable">
            <div class="deliverable-file">
              <span class="file-glyph" aria-hidden="true">▤</span>
              <div class="file-text">
                <span class="file-name">{{ primaryArtifact.name }}</span>
                <code class="file-path">{{ primaryArtifact.path }}</code>
              </div>
            </div>
            <div class="btn-row">
              <button
                v-if="selectedHorde"
                type="button"
                class="primary"
                @click="openOutputFolder(selectedHorde.workdir || selectedHorde.root_path)"
              >
                Open output folder
              </button>
              <button
                v-if="isAbsolutePath(primaryArtifact.path)"
                type="button"
                @click="openOutputFolder(primaryArtifact.path)"
              >
                Open file
              </button>
            </div>
          </div>
          <div v-else class="btn-row">
            <button
              v-if="selectedHorde"
              type="button"
              class="primary"
              @click="openOutputFolder(selectedHorde.workdir || selectedHorde.root_path)"
            >
              Open output folder
            </button>
          </div>
          <p v-if="pathAction" class="muted small">{{ pathAction }}</p>
          <h3 class="delivery-title">{{ selectedHorde?.delivery_title || "Final delivery" }}</h3>
          <p>{{ finalShortSummary }}</p>
          <p v-if="selectedHorde?.delivery_note" class="muted small" v-html="deliveryNoteHtml" />

          <template v-if="handoffMarkdown">
            <div class="handoff-rendered" v-html="handoffHtml" />
            <details class="handoff-raw">
              <summary>Markdown source</summary>
              <textarea
                readonly
                class="paste-handoff-markdown"
                rows="14"
                spellcheck="false"
                :value="handoffMarkdown"
              />
            </details>
            <div class="btn-row">
              <button type="button" @click="copyPasteToClipboard">Copy as Markdown</button>
            </div>
            <p v-if="copyPasteErr" class="err">{{ copyPasteErr }}</p>
          </template>
          <p v-else class="muted small">
            This run finished without a hand-off file. Its working files are in the output folder, under
            <code>debug/</code>.
          </p>
          <details v-if="otherArtifacts.length">
            <summary>Intermediate files ({{ otherArtifacts.length }})</summary>
            <ul class="artifact-list">
              <li v-for="a in otherArtifacts" :key="`${a[0]}-${a[1]}`">
                <span class="chip">{{ a[0] }}</span>
                <code>{{ a[1] }}</code>
              </li>
            </ul>
          </details>
          <details>
            <summary>Raw run_finished payload</summary>
            <pre class="json">{{ runResult }}</pre>
          </details>
        </section>

        <section v-if="hasCompletedRun" class="card followups">
          <p class="eyebrow">Follow-up</p>
          <ol v-if="followupMsgs.length" class="feed followup-feed">
            <li
              v-for="(m, i) in followupMsgs"
              :key="`f-${i}`"
              class="msg"
              :class="m.role === 'user' ? 'msg-user' : m.role === 'orchestrator' ? 'msg-system' : 'msg-worker'"
            >
              <div class="msg-body">
                <header class="msg-head"><span class="msg-speaker">{{ m.speaker }}</span></header>
                <pre class="msg-text">{{ m.text }}</pre>
              </div>
            </li>
          </ol>
          <HordeRunForm
            :horde="selectedHorde"
            :disabled="!selectedHordeId"
            :busy="runBusy || followupBusy"
            :follow-up-mode="true"
            @submit="onHordeFormSubmit"
          />
          <div class="btn-row redo">
            <button type="button" class="ghost" :disabled="runBusy || followupBusy" @click="redefineAndStartAgain">
              Redefine and start again
            </button>
          </div>
        </section>

        <p v-if="pathAction && !(runCompleted && runResult)" class="muted small">{{ pathAction }}</p>

        <section v-if="triggerRows.length" class="card flat">
          <p class="eyebrow">Triggers</p>
          <p class="muted small">
            Declared in this horde's <code>horde.md</code>. The toggle is a server-side operator
            override (survives restarts, never edits the file); Fire now starts the trigger's run
            immediately — the overlap policy still applies.
          </p>
          <article v-for="t in triggerRows" :key="t.index" class="row-item">
            <div class="row-meta">
              <div class="row-line">
                <span class="badge" :class="t.effective_enabled ? 'badge-ok' : 'badge-muted'">{{ t.kind }}</span>
                <code>{{ t.detail }}</code>
                <span class="muted small">
                  {{ t.effective_enabled ? "armed" : "disabled" }}{{ t.overridden ? " · operator override" : "" }} · overlap={{ t.overlap }}
                </span>
              </div>
              <div class="muted small row-line">
                <span v-if="t.next_fire">next fire {{ shortTime(t.next_fire) }}</span>
                <span v-if="t.last_fired">
                  last fired {{ shortTime(t.last_fired.time) }} ·
                  <a
                    href="#"
                    :title="`Highlight run ${t.last_fired.run_id} in Recent runs`"
                    @click.prevent="highlightRunId = t.last_fired?.run_id ?? null"
                  >{{ t.last_fired.run_id }}</a>
                  ({{ t.last_fired.status }})
                </span>
                <span v-else>never fired</span>
              </div>
            </div>
            <div class="btn-row">
              <button type="button" class="sm" :disabled="triggerBusy !== null" @click="toggleTrigger(t)">
                {{ triggerBusy === t.index ? "…" : t.effective_enabled ? "Disable" : "Enable" }}
              </button>
              <button
                type="button"
                class="sm"
                :disabled="triggerBusy !== null"
                title="Start this trigger's run immediately (works while disabled; overlap policy still applies)"
                @click="fireTriggerNow(t)"
              >
                Fire now
              </button>
            </div>
          </article>
          <p v-if="triggerNote" class="muted small">{{ triggerNote }}</p>
        </section>

        <section v-if="runHistory.length" class="card flat">
          <p class="eyebrow">Recent runs</p>
          <ol class="runs">
            <li
              v-for="r in runHistory.slice(0, 15)"
              :key="r.run_id"
              class="run-row"
              :class="{ 'run-highlight': r.run_id === highlightRunId }"
            >
              <span class="badge" :class="runStatusBadge(r.status)">{{ r.status }}</span>
              <span class="run-prompt" :title="r.prompt">{{ promptLine(r.prompt) || r.question || r.run_id }}</span>
              <span
                class="chip"
                :title="runSourceBadge(r).isTrigger ? `Fired by a ${runSourceBadge(r).label} trigger` : 'Started by an operator'"
              >{{ runSourceBadge(r).label }}</span>
              <span
                v-if="(r.resume_count ?? 0) > 0"
                class="chip"
                title="This run was interrupted and resumed"
              >resumed ×{{ r.resume_count }}</span>
              <span class="run-time mono">{{ shortTime(r.started_at) }}</span>
              <code class="run-id">{{ r.run_id }}</code>
            </li>
          </ol>
        </section>
      </div>
    </div>
  </section>
</template>

<style scoped>
.tiny { font-size: 0.72rem; }

/* ---------- approval: unmissable ---------- */
.approval-box {
  background: var(--red-soft);
  border: 1px solid var(--red);
  border-left: 8px solid var(--red);
  border-radius: var(--radius);
  padding: 1.1rem 1.25rem 1rem;
  margin: 0 0 1.25rem;
}
.approval-box h2 { font-size: 1.3rem; margin: 0 0 0.4rem; }
.approval-tag { color: var(--red-ink); }
.approval-text { color: var(--ink); }
.approval-cmd {
  background: var(--ink);
  color: var(--paper);
  border-radius: var(--radius-sm);
  padding: 0.7rem 0.9rem;
  margin: 0 0 0.9rem;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-word;
}
.approval-cmd .prompt { color: var(--red); font-weight: 600; }
.approval-foot { margin: 0.6rem 0 0; color: var(--body); }

/* ---------- interrupted runs: calm ---------- */
.resume-banner h3 { margin: 0 0 0.25rem; font-size: 1rem; }
.resume-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  flex-wrap: wrap;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  padding: 0.5rem 0.7rem;
  margin-top: 0.45rem;
}
.resume-meta { display: grid; gap: 0.1rem; min-width: 0; flex: 1 1 16rem; }
.resume-prompt { color: var(--ink); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.resume-approval { flex: 1 1 12rem; font-size: 0.9rem; color: var(--ink); }

/* ---------- hand-off ---------- */
.handoff-rendered {
  background: var(--surface);
  border: 1px solid var(--line);
  border-left: 4px solid var(--ink);
  border-radius: 6px;
  padding: 0.4rem 1.2rem 0.8rem;
  line-height: 1.6;
  overflow-x: auto;
}
.handoff-rendered :deep(h1) { font-size: 1.35rem; margin: 0.7rem 0 0.4rem; }
.handoff-rendered :deep(h2) { font-size: 1.12rem; margin: 1.1rem 0 0.35rem; }
.handoff-rendered :deep(h3) { font-size: 1rem; margin: 1rem 0 0.3rem; }
.handoff-rendered :deep(table) { border-collapse: collapse; margin: 0.5rem 0; font-size: 0.92rem; }
.handoff-rendered :deep(th), .handoff-rendered :deep(td) { border: 1px solid var(--hair); padding: 0.3rem 0.6rem; text-align: left; }
.handoff-rendered :deep(th) { background: var(--sunk); }
.handoff-rendered :deep(code) { background: var(--sunk); padding: 0.05rem 0.3rem; border-radius: 3px; }
.handoff-raw { margin-top: 0.6rem; }

/* ---------- layout ---------- */
.hordes-layout {
  display: grid;
  grid-template-columns: minmax(15rem, 19rem) minmax(0, 1fr);
  gap: 1.5rem;
  align-items: start;
}
.mission { display: grid; gap: 1rem; min-width: 0; }
.mission > * { margin: 0; min-width: 0; }

/* ---------- picker ---------- */
.picker { position: sticky; top: 0; }
.picker-list { display: grid; gap: 0.5rem; }
.horde-card {
  display: grid;
  gap: 0.35rem;
  justify-items: start;
  text-align: left;
  white-space: normal;
  width: 100%;
  padding: 0.75rem 0.85rem 0.75rem 0.95rem;
  background: var(--surface);
  border: 1px solid var(--line);
  border-left: 4px solid transparent;
  border-radius: var(--radius);
  color: var(--body);
  font-weight: 400;
}
.horde-card:hover:not(:disabled) { background: var(--surface); border-color: var(--muted); border-left-color: var(--line); }
.horde-card.selected { border-color: var(--ink); border-left-color: var(--red); }
.hc-head { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.hc-name { font-family: var(--font-display); font-weight: 700; font-size: 1rem; color: var(--ink); }
.hc-desc {
  font-size: 0.86rem;
  color: var(--muted);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.hc-meta { display: flex; flex-wrap: wrap; gap: 0.3rem; }
.hc-meta .chip { font-size: 0.68rem; }
.hc-delivers { font-family: var(--font-mono); font-size: 0.72rem; color: var(--body); overflow-wrap: anywhere; }

/* ---------- briefing ---------- */
.briefing h2 { margin: 0 0 0.35rem; }
.briefing-desc { max-width: 48rem; }
.squad {
  list-style: none;
  padding: 0;
  margin: 0.75rem 0 0.25rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}
.squad-member {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.3rem 0.65rem 0.3rem 0.3rem;
  background: var(--sunk);
  border: 1px solid var(--hair);
  border-radius: var(--radius);
}
.sm-text { display: grid; line-height: 1.15; }
.sm-name { font-weight: 600; font-size: 0.86rem; color: var(--ink); }
.sm-kind { font-family: var(--font-mono); font-size: 0.66rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.06em; }
.briefing-details { margin: 0.75rem 0 0; }
.workdir-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  flex-wrap: wrap;
  margin-top: 0.5rem;
}
.workdir-row .lbl { min-width: 8.5rem; }

/* ---------- progress ---------- */
.progress-head { display: flex; justify-content: space-between; align-items: baseline; gap: 0.75rem; flex-wrap: wrap; }
.processing {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  flex-wrap: wrap;
  margin-top: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--hair);
}
.processing-text { color: var(--ink); font-weight: 500; }
.idle-hint { margin: 0.6rem 0 0; }

/* ---------- live feed ---------- */
.feed {
  list-style: none;
  margin: 0;
  padding: 0;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  max-height: 60vh;
  overflow-y: auto;
}
.msg {
  display: grid;
  grid-template-columns: 2.4rem minmax(0, 1fr);
  gap: 0.65rem;
  padding: 0.65rem 0.9rem;
  border-bottom: 1px solid var(--hair);
}
.msg:last-child { border-bottom: 0; }
.msg-avatar { display: flex; justify-content: center; padding-top: 0.1rem; }
.avatar-fallback {
  width: 1.9rem;
  height: 1.9rem;
  border-radius: 50%;
  display: inline-grid;
  place-items: center;
  font-family: var(--font-mono);
  font-size: 0.62rem;
  font-weight: 600;
  background: var(--sunk);
  color: var(--muted);
}
.msg-user .avatar-fallback { background: var(--ink); color: var(--paper); }
.msg-body { min-width: 0; }
.msg-head { display: flex; align-items: baseline; gap: 0.5rem; flex-wrap: wrap; margin-bottom: 0.15rem; }
.msg-speaker { font-weight: 600; color: var(--ink); font-size: 0.92rem; }
.msg-role {
  font-family: var(--font-mono);
  font-size: 0.64rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted);
}
.msg-time { margin-left: auto; font-family: var(--font-mono); font-size: 0.7rem; color: var(--muted); }
.msg-text {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--font-body);
  font-size: 0.92rem;
  line-height: 1.5;
  color: var(--body);
}
.msg-user { background: var(--sunk); }
.msg-user .msg-text { color: var(--ink); }
.msg-system .msg-text { font-family: var(--font-mono); font-size: 0.8rem; color: var(--muted); }
.msg-orchestrator .msg-speaker::before {
  content: "";
  display: inline-block;
  width: 0.6rem;
  height: 3px;
  background: var(--red);
  margin-right: 0.4rem;
  vertical-align: middle;
}
.followup-feed { max-height: none; margin-bottom: 1rem; }
.followup-feed .msg { grid-template-columns: minmax(0, 1fr); }

/* ---------- delivery ---------- */
.delivery { border-top-color: var(--ok); }
.deliverable {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
  padding: 0.9rem 1rem;
  margin: 0.25rem 0 0.9rem;
  background: var(--ok-soft);
  border: 1px solid var(--ok);
  border-radius: var(--radius);
}
.deliverable-file { display: flex; align-items: center; gap: 0.8rem; min-width: 0; }
.file-glyph {
  width: 2.6rem;
  height: 2.6rem;
  display: inline-grid;
  place-items: center;
  font-size: 1.3rem;
  border-radius: var(--radius-sm);
  background: var(--ink);
  color: var(--paper);
  flex: 0 0 auto;
}
.file-text { display: grid; gap: 0.2rem; min-width: 0; }
.file-name { font-family: var(--font-display); font-weight: 800; font-size: 1.25rem; color: var(--ink); letter-spacing: -0.01em; }
.file-path { background: transparent; border: 0; padding: 0; font-size: 0.76rem; color: var(--body); }
.delivery-title { margin-top: 0.5rem; }
.handoff-title { margin: 1rem 0 0.35rem; }
.paste-handoff-markdown {
  width: 100%;
  min-height: 12rem;
  font-family: var(--font-mono);
  font-size: 0.8rem;
  line-height: 1.5;
  background: var(--sunk);
  white-space: pre;
  overflow-x: auto;
  margin-bottom: 0.6rem;
}
.artifact-list { list-style: none; padding: 0; margin: 0.4rem 0 0; display: grid; gap: 0.35rem; }
.artifact-list li { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.redo { margin-top: 0.6rem; }

/* ---------- triggers / runs ---------- */
.row-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  flex-wrap: wrap;
  padding: 0.6rem 0;
  border-top: 1px solid var(--hair);
}
.row-meta { display: grid; gap: 0.25rem; min-width: 0; }
.row-line { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.runs { list-style: none; padding: 0; margin: 0; }
.run-row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto auto auto;
  align-items: center;
  gap: 0.6rem;
  padding: 0.5rem 0.4rem;
  border-top: 1px solid var(--hair);
}
.run-prompt { color: var(--ink); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 0.9rem; }
.run-time { font-size: 0.72rem; color: var(--muted); }
.run-id { grid-column: 2 / -1; font-size: 0.7rem; background: transparent; border: 0; padding: 0; color: var(--muted); }
.run-highlight { background: var(--red-soft); box-shadow: inset 3px 0 0 var(--red); }

@media (max-width: 1180px) {
  .hordes-layout { grid-template-columns: minmax(0, 1fr); }
  .picker { position: static; }
  .picker-list { grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr)); }
}
@media (max-width: 700px) {
  .run-row { grid-template-columns: auto minmax(0, 1fr); }
  .run-time { grid-column: 2; }
}
</style>
