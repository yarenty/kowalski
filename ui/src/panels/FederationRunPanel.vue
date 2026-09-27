<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import DOMPurify from "dompurify";
import { marked } from "marked";
import {
  api,
  openFederationEventSource,
  type FederationWorkerProfile,
  type HordeRunRecord,
  type HordeTriggerStatus,
  type RunSummary,
} from "../api";
import HordeIcon from "../components/HordeIcon.vue";
import HordeRunForm from "../components/HordeRunForm.vue";
import PenguinAvatar from "../components/PenguinAvatar.vue";
import PipelineStepper, { type StepState, type StepperItem } from "../components/PipelineStepper.vue";
import { hordeById, hordesLoaded, refreshHordes, useHordePolling, firstSentence } from "../hordesStore";
import { inferPenguinAvatarId } from "../penguins";
import { isDagHorde } from "../hordeGraph";
import { elapsed, relativeTime, runStatusLabel, upcomingTime } from "../runs";

/**
 * One horde's page: the request form, its past runs, and — when a run is open — that run's
 * progress, approval and delivered answers. Entered from a Hordes tile, the ⌘K picker, the Runs
 * page or a `?horde=<id>[&run=<id>]` deep link.
 */
const props = defineProps<{ hordeId: string; runId: string | null }>();
const emit = defineEmits<{
  (e: "go-home"): void;
  /** Open a run of this horde (`null` = back to the request form). */
  (e: "open-run", runId: string | null): void;
  /** A run started from this page: reflect it in the URL without reloading it. */
  (e: "run-started", runId: string): void;
  (e: "open-runs", hordeId: string): void;
  (e: "open-build"): void;
  (e: "new-chat-session"): void;
  /** A run started, ended or was approved: counts elsewhere (top bar badge) are stale. */
  (e: "runs-changed"): void;
}>();

type FeedRole = "orchestrator" | "worker" | "system" | "user";
type FeedMsg = { role: FeedRole; speaker: string; text: string; step?: string; at?: number };
type FollowupMsg = { role: "user" | "assistant" | "orchestrator"; speaker: string; text: string };

const fedTopic = ref("federation");
const fedEs = ref<EventSource | null>(null);
const runBusy = ref(false);
const runId = ref<string | null>(null);
const runMessages = ref<FeedMsg[]>([]);
/** Live per-step state from federation events for the current run (overlays run history). */
const stepLive = ref<Record<string, StepState>>({});
/** When each step started working (ms), for the "Now" card's elapsed time. */
const stepStartedAt = ref<Record<string, number>>({});
const runResult = ref<string | null>(null);
const runFailure = ref<{ kind: "failed" | "cancelled"; reason: string } | null>(null);
const runErr = ref<string | null>(null);
const runWatchdog = ref<number | null>(null);
const runLoading = ref(false);
const workerProfiles = ref<FederationWorkerProfile[]>([]);
const runHistory = ref<HordeRunRecord[]>([]);
const pastRuns = ref<RunSummary[]>([]);
const followupBusy = ref(false);
const followupMsgs = ref<FollowupMsg[]>([]);
const pathAction = ref<string | null>(null);
const cleanWorkdirBusy = ref(false);
const triggerRows = ref<HordeTriggerStatus[]>([]);
const triggerBusy = ref<number | null>(null);
const triggerNote = ref<string | null>(null);
const now = ref(Date.now());

const selectedHorde = computed(() => hordeById(props.hordeId));
const selectedHordeIsDag = computed(() => {
  const h = selectedHorde.value;
  return h ? isDagHorde(h.pipeline, h.edges ?? []) : false;
});
const selectedHordeWorkers = computed(() => workerProfiles.value.filter((w) => w.horde_id === props.hordeId));
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
 * The approval shown in the big box: the live one, else the open run's, else the first parked
 * run of this horde that waits before a command step.
 */
const shownApproval = computed((): PendingApproval | null => {
  if (approval.value) return approval.value;
  const parked = resumableRuns.value.map(approvalFromRun).filter((a): a is PendingApproval => a !== null);
  return parked.find((a) => a.runId === runId.value) ?? parked[0] ?? null;
});
/** Interrupted runs listed in the calm banner (the one in the approval box is not repeated). */
const bannerRuns = computed(() => resumableRuns.value.filter((r) => r.run_id !== shownApproval.value?.runId));
const activeRunFromHistory = computed(() =>
  runId.value ? runHistory.value.find((r) => r.run_id === runId.value) ?? null : null,
);
const finalDelivery = computed(() => {
  if (!runResult.value) return null;
  try {
    return JSON.parse(runResult.value) as { kind?: string; text?: string; artifacts?: Array<[string, string]> };
  } catch {
    return null;
  }
});
const finalArtifacts = computed(() => finalDelivery.value?.artifacts ?? []);
const runCompleted = computed(
  () => finalDelivery.value?.kind === "run_finished" || activeRunFromHistory.value?.status === "completed",
);

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
      stepLive.value[name] ?? historyStepState(record?.steps.find((s) => s.step === name)?.status) ?? "pending";
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

/** What the page shows: the request form, a run in progress, a finished run, or an ended one. */
const view = computed((): "form" | "loading" | "running" | "done" | "ended" => {
  if (!runId.value && !runBusy.value) return runLoading.value ? "loading" : "form";
  if (runCompleted.value) return "done";
  if (runFailure.value) return "ended";
  const st = activeRunFromHistory.value?.status;
  if (!runBusy.value && (st === "failed" || st === "cancelled")) return "ended";
  return "running";
});

/** The open run's readable title (server-built), falling back to the operator's words. */
const runTitle = computed(() => {
  const id = runId.value;
  const fromList = id ? pastRuns.value.find((r) => r.run_id === id)?.title : undefined;
  return (
    activeRunFromHistory.value?.title ||
    fromList ||
    promptLine(runMessages.value.find((m) => m.role === "user")?.text).slice(0, 80) ||
    "This run"
  );
});
const runStartedAt = computed(() => activeRunFromHistory.value?.started_at ?? null);

/** The step working right now, for the single "Now" card. */
const nowStep = computed(() => {
  const item = stepperItems.value.find((s) => s.state === "running" || s.state === "waiting");
  if (!item) return null;
  const agent = subAgentFor(item.name);
  const recordStart = activeRunFromHistory.value?.steps.find((s) => s.step === item.name) as
    | { started_at?: string }
    | undefined;
  const liveStart = stepStartedAt.value[item.name];
  const stored = recordStart?.started_at ? Date.parse(recordStart.started_at) : NaN;
  const since = liveStart ?? (Number.isNaN(stored) ? null : stored);
  return {
    label: item.label,
    waiting: item.state === "waiting",
    what: firstSentence(agent?.description) || (agent?.kind ? `A ${agent.kind} step.` : ""),
    since,
  };
});

/** The file the run delivered: the last step's artifact (e.g. HANDOFF.md, report.xlsx, BRIEF.md). */
const primaryArtifact = computed((): { step: string; path: string; name: string } | null => {
  const list = finalArtifacts.value;
  if (!list.length) return null;
  const [step, path] = list[list.length - 1];
  return { step, path, name: path.split(/[\\/]/).pop() || path };
});
const otherArtifacts = computed(() => finalArtifacts.value.slice(0, -1));
/** Per-step summaries of the open run ("How it was computed"). */
const stepSummaries = computed(() =>
  (activeRunFromHistory.value?.steps ?? [])
    .filter((s) => s.summary || s.artifact)
    .map((s) => ({ ...s, label: subAgentFor(s.step)?.display_name || titleCase(s.step) })),
);

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
const finalShortSummary = computed(() => selectedHorde.value?.delivery_summary_note || "");
const deliveryNoteHtml = computed(() =>
  DOMPurify.sanitize(marked.parseInline(selectedHorde.value?.delivery_note ?? "") as string),
);
/** "cancelled by operator" → "Cancelled by operator." */
function sentence(text: string): string {
  const t = text.trim();
  if (!t) return t;
  return t[0].toUpperCase() + t.slice(1) + (/[.!?]$/.test(t) ? "" : ".");
}
/** Hand-offs may start with a `---` metadata block; the reader wants the document. */
function stripFrontMatter(md: string): string {
  return md.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, "");
}
/** Server text with `code` spans (approval prompts), rendered inline and sanitised. */
function inlineMd(text: string): string {
  return DOMPurify.sanitize(marked.parseInline(text) as string);
}
const handoffMarkdown = computed(() => {
  if (!runResult.value) return "";
  try {
    const p = JSON.parse(runResult.value) as { handoff_markdown?: string; paste_for_obsidian?: string };
    if (typeof p.handoff_markdown === "string") return p.handoff_markdown;
    if (typeof p.paste_for_obsidian === "string") return p.paste_for_obsidian;
    return "";
  } catch {
    return "";
  }
});
/** The hand-off rendered for reading (sanitised; the raw Markdown stays one click away). */
const handoffHtml = computed(() =>
  handoffMarkdown.value
    ? DOMPurify.sanitize(marked.parse(stripFrontMatter(handoffMarkdown.value), { gfm: true }) as string)
    : "",
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

function feed(role: FeedRole, text: string, speaker?: string, step?: string, at?: number) {
  runMessages.value = [
    ...runMessages.value,
    {
      role,
      speaker: speaker || (role === "orchestrator" ? "Agent: Boss" : "System"),
      text,
      ...(step ? { step } : {}),
      at: at ?? Date.now(),
    },
  ];
}

function avatarForRunMessage(m: { role: string; step?: string }): { avatar: string; kind: string; name: string } | null {
  if (m.role === "orchestrator") return { avatar: "director", kind: "orchestrator", name: "boss" };
  if (m.role === "worker" && m.step) {
    const agent = subAgentFor(m.step);
    return {
      avatar: agent?.avatar ?? inferPenguinAvatarId(agent?.kind ?? "process", m.step),
      kind: agent?.kind ?? "process",
      name: m.step,
    };
  }
  return null;
}

function extractUrls(input: string): string[] {
  const matches = input.match(/https?:\/\/\S+/g) ?? [];
  const cleaned = matches.map((u) => u.trim().replace(/[),.;]+$/, ""));
  return [...new Set(cleaned)];
}

/** Stored event time (`<secs>.<ms>Z` or RFC 3339) in ms, when present. */
function eventTime(ev: Record<string, unknown>): number | undefined {
  const ts = ev.ts;
  if (typeof ts !== "string" && typeof ts !== "number") return undefined;
  const s = String(ts);
  const secs = /^(\d+(?:\.\d+)?)Z?$/.exec(s);
  if (secs) return Math.round(Number(secs[1]) * 1000);
  const d = Date.parse(s);
  return Number.isNaN(d) ? undefined : d;
}

function processFederationEvent(data: string) {
  let parsed: unknown;
  try {
    parsed = JSON.parse(data) as unknown;
  } catch {
    return;
  }
  const payload = (parsed as { payload?: Record<string, unknown> }).payload;
  if (!payload || typeof payload !== "object") return;
  const evRunId = String(payload.run_id ?? "");
  if (evRunId && String(payload.horde ?? props.hordeId) !== props.hordeId) return;
  if (runId.value && evRunId && evRunId !== runId.value) {
    // Another run of this horde changed: keep the side list and banners current.
    if (["run_finished", "run_failed", "run_cancelled", "approval_required"].includes(String(payload.kind))) {
      void refreshRunLists();
    }
    return;
  }
  if (!runId.value && !runBusy.value) return;
  applyEvent(payload, false);
}

/**
 * Fold one run event into the page. `replay` = rebuilding an opened run from its stored events:
 * no network side effects, times come from the event.
 */
function applyEvent(payload: Record<string, unknown>, replay: boolean) {
  const kind = String(payload.kind ?? "");
  const at = replay ? eventTime(payload) : Date.now();
  const evRunId = String(payload.run_id ?? "");

  if (kind === "task_assigned") {
    const step = String(payload.step ?? "?");
    setStep(step, "running", at);
    feed("orchestrator", `${step} assigned to ${String(payload.to ?? "?")}`, "Agent: Boss", undefined, at);
  } else if (kind === "task_started") {
    const step = String(payload.step ?? "?");
    setStep(step, "running", at);
    feed("worker", `${step} started by ${String(payload.agent ?? "?")}`, speakerNameFromStep(step), step, at);
  } else if (kind === "agent_message") {
    const step = String(payload.step ?? "");
    feed("worker", String(payload.text ?? "(message)"), speakerNameFromStep(step || "worker"), step || undefined, at);
  } else if (kind === "task_finished") {
    const step = String(payload.step ?? "?");
    const artifact = String(payload.artifact ?? "");
    const ok = Boolean(payload.success);
    const outcome = payload.outcome != null ? String(payload.outcome) : "";
    setStep(step, ok ? "done" : "failed", at);
    const outcomeNote = outcome ? ` (outcome: ${outcome})` : "";
    feed(
      "worker",
      ok
        ? `${step} completed${outcomeNote}${artifact ? ` -> ${artifact}` : ""}`
        : `${step} failed${outcomeNote}${payload.summary ? `: ${String(payload.summary)}` : ""}`,
      speakerNameFromStep(step),
      step,
      at,
    );
  } else if (kind === "step_routed") {
    const fromStep = String(payload.from_step ?? "?");
    const nextStep = String(payload.next_step ?? "?");
    const outcome = String(payload.outcome ?? "?");
    const isLoop = Boolean(payload.is_loop_back);
    const loopCount = payload.loop_count != null ? Number(payload.loop_count) : null;
    const verifyExcerpt = typeof payload.verify_excerpt === "string" ? payload.verify_excerpt.trim() : "";
    const branch = isLoop
      ? `retry loop → \`${nextStep}\`${loopCount != null ? ` (loop ${loopCount})` : ""}`
      : `branch → \`${nextStep}\``;
    feed("orchestrator", `${fromStep} outcome **${outcome}**: ${branch}`, "Agent: Boss", fromStep, at);
    if (verifyExcerpt) feed("worker", verifyExcerpt, speakerNameFromStep(fromStep), fromStep, at);
  } else if (kind === "run_finished") {
    runResult.value = JSON.stringify(payload, null, 2);
    feed("system", "run finished", "System", undefined, at);
    runBusy.value = false;
    if (!replay) onRunEnded();
  } else if (kind === "run_failed") {
    const reason = payload.reason ? String(payload.reason) : "";
    runFailure.value = { kind: "failed", reason };
    settleSteps("failed");
    feed("system", `run failed${reason ? `: ${reason}` : ""}`, "System", undefined, at);
    runBusy.value = false;
    if (!replay) onRunEnded();
  } else if (kind === "approval_required") {
    const step = String(payload.step ?? "?");
    if (!replay) {
      approval.value = {
        runId: evRunId || runId.value || "",
        step,
        text: String(payload.text ?? "A step is waiting for your approval."),
        command: payload.command != null ? String(payload.command) : null,
      };
    }
    setStep(step, "waiting", at);
    feed("orchestrator", String(payload.text ?? `${step} waits for approval`), "Agent: Boss", step, at);
    runBusy.value = false;
    if (!replay) onRunEnded();
  } else if (kind === "run_cancelled") {
    const reason = payload.reason ? String(payload.reason) : "";
    runFailure.value = { kind: "cancelled", reason };
    settleSteps("cancelled");
    feed("system", `run cancelled${reason ? `: ${reason}` : ""}`, "System", undefined, at);
    runBusy.value = false;
    if (!replay) onRunEnded();
  }
}

function onRunEnded() {
  clearRunWatchdog();
  void refreshRunLists();
  emit("runs-changed");
}

function setStep(step: string, state: StepState, at?: number) {
  if (!step || step === "?") return;
  if (state === "running" && stepLive.value[step] !== "running") {
    stepStartedAt.value = { ...stepStartedAt.value, [step]: at ?? Date.now() };
  }
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

async function loadProfiles() {
  try {
    const r = await api.hordeWorkers(props.hordeId);
    workerProfiles.value = r.workers ?? [];
  } catch {
    workerProfiles.value = [];
  }
}

async function loadTriggers() {
  try {
    const r = await api.hordeTriggers(props.hordeId);
    triggerRows.value = r.triggers ?? [];
  } catch {
    triggerRows.value = [];
  }
}

/** Full run records (resumable runs, approvals, the open run) and the short side list. */
async function refreshRunLists() {
  const id = props.hordeId;
  const [full, list] = await Promise.allSettled([api.hordeRuns(id), api.runs({ horde: id, limit: 5 })]);
  if (id !== props.hordeId) return;
  if (full.status === "fulfilled") runHistory.value = full.value.runs ?? [];
  if (list.status === "fulfilled") pastRuns.value = list.value.runs;
}

async function toggleTrigger(t: HordeTriggerStatus) {
  if (triggerBusy.value !== null) return;
  triggerBusy.value = t.index;
  triggerNote.value = null;
  try {
    const r = await api.hordeTriggerSetEnabled(props.hordeId, t.index, !t.effective_enabled);
    triggerRows.value = r.triggers ?? [];
    triggerNote.value = `${t.effective_enabled ? "Switched off" : "Switched on"} (kept on the server; horde.md is untouched).`;
  } catch (e) {
    triggerNote.value = e instanceof Error ? e.message : String(e);
  } finally {
    triggerBusy.value = null;
  }
}

async function fireTriggerNow(t: HordeTriggerStatus) {
  if (triggerBusy.value !== null) return;
  triggerBusy.value = t.index;
  triggerNote.value = null;
  try {
    const r = await api.hordeTriggerFire(props.hordeId, t.index);
    if (r.fired && r.run) {
      triggerNote.value = "Started a run.";
      emit("open-run", r.run.run_id);
    } else if (r.skipped) {
      triggerNote.value = "Not started: the run this trigger fired last is still working.";
    } else if (r.queued) {
      triggerNote.value = "Queued behind the run in flight; it starts when that one ends.";
    }
  } catch (e) {
    triggerNote.value = e instanceof Error ? e.message : String(e);
  } finally {
    triggerBusy.value = null;
    await Promise.all([loadTriggers(), refreshRunLists()]);
  }
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
  cleanWorkdirBusy.value = true;
  pathAction.value = null;
  try {
    const r = await api.hordeCleanWorkdir(props.hordeId);
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
  const maxAttempts = 8;
  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    await loadProfiles();
    const workers = selectedHordeWorkers.value;
    if (workers.length > 0 && workers.every((w) => isWorkerReady(w))) return true;
    for (const w of workers.filter((w) => !isWorkerReady(w))) {
      await api.hordeWorkersStart(props.hordeId, w.step);
    }
    await sleep(650);
  }
  await loadProfiles();
  return selectedHordeWorkers.value.length > 0 && selectedHordeWorkers.value.every((w) => isWorkerReady(w));
}

// ---------- follow-ups (kept per run in this browser) ----------
function followupKey(id: string) {
  return `kowalski.ui.horde.followups.${id}`;
}
function loadFollowups(id: string) {
  try {
    const raw = localStorage.getItem(followupKey(id));
    const parsed = raw ? (JSON.parse(raw) as FollowupMsg[]) : [];
    followupMsgs.value = Array.isArray(parsed) ? parsed : [];
  } catch {
    followupMsgs.value = [];
  }
}
function saveFollowups() {
  if (!runId.value) return;
  try {
    if (followupMsgs.value.length) localStorage.setItem(followupKey(runId.value), JSON.stringify(followupMsgs.value));
  } catch {
    /* storage unavailable */
  }
}

function resetRunState() {
  clearRunWatchdog();
  runId.value = null;
  runBusy.value = false;
  runMessages.value = [];
  runResult.value = null;
  runFailure.value = null;
  runErr.value = null;
  followupMsgs.value = [];
  stepLive.value = {};
  stepStartedAt.value = {};
  approval.value = null;
}

/** Open a stored run: rebuild its feed, stepper and delivery from the run record. */
let openSeq = 0;
async function openRun(id: string) {
  const seq = ++openSeq;
  resetRunState();
  runId.value = id;
  runLoading.value = true;
  try {
    const { run } = await api.hordeRunDetail(props.hordeId, id);
    if (seq !== openSeq || runId.value !== id) return;
    runHistory.value = [run, ...runHistory.value.filter((r) => r.run_id !== id)];
    feed("user", promptLine(run.prompt) || run.question || "(operator form submitted)", "You", undefined, Date.parse(run.started_at ?? "") || undefined);
    for (const ev of run.events ?? []) applyEvent(ev, true);
    runBusy.value = run.status === "running" || run.status === "pending";
    if (run.status === "awaiting_input") approval.value = approvalFromRun(run);
    loadFollowups(id);
  } catch (e) {
    runErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    runLoading.value = false;
  }
}

async function runHordeWithPayload(payload: {
  prompt: string;
  source: string;
  question: string;
  formAnswers?: Record<string, string>;
}) {
  await Promise.all([refreshHordes(), loadProfiles()]);
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
  resetRunState();
  runBusy.value = true;
  connectStream();
  feed("user", promptLine(prompt) || "(operator form submitted)", "You");
  feed("orchestrator", "creating run", "Agent: Boss");
  if (sources.length) feed("orchestrator", `source(s): ${sources.join(", ")}`, "Agent: Boss");
  try {
    const out = await api.hordeRun(props.hordeId, {
      prompt,
      source: payload.source,
      question: payload.question,
      form_answers: payload.formAnswers,
    });
    runId.value = out.run.run_id;
    runHistory.value = [out.run, ...runHistory.value.filter((r) => r.run_id !== out.run.run_id)];
    emit("run-started", out.run.run_id);
    feed("orchestrator", `run started: ${out.run.run_id}`, "Agent: Boss");
    void refreshRunLists();
    runWatchdog.value = window.setTimeout(() => {
      if (!runBusy.value) return;
      feed("system", "no progress events within 60s — the run may still be working", "System");
      runErr.value =
        "No progress arrived in 60 s. The run may still be working; check sub-agent workers under Admin → Federation.";
    }, 60_000);
  } catch (e) {
    runBusy.value = false;
    runErr.value = e instanceof Error ? e.message : String(e);
  }
}

const cancelBusy = ref(false);
async function cancelActiveRun() {
  if (!runId.value || cancelBusy.value) return;
  cancelBusy.value = true;
  try {
    await api.hordeRunCancel(props.hordeId, runId.value);
    // The run_cancelled feed event flips runBusy and refreshes history.
  } catch (e) {
    runErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    cancelBusy.value = false;
  }
}

async function approvePending(pending: PendingApproval) {
  if (approvalBusy.value) return;
  approvalBusy.value = true;
  runErr.value = null;
  try {
    if (runId.value !== pending.runId) {
      // Approving another parked run: open it here so its progress shows.
      emit("open-run", pending.runId);
      await openRun(pending.runId);
    }
    connectStream();
    await api.hordeRunApprove(props.hordeId, pending.runId);
    approval.value = null;
    emit("runs-changed");
    runBusy.value = true;
    setStep(pending.step, "running");
    feed("user", `Approved \`${pending.step}\``, "You");
  } catch (e) {
    runErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    approvalBusy.value = false;
    void refreshRunLists();
  }
}

async function rejectPending(pending: PendingApproval) {
  if (approvalBusy.value) return;
  approvalBusy.value = true;
  try {
    await api.hordeRunCancel(props.hordeId, pending.runId);
    approval.value = null;
    if (pending.runId === runId.value) {
      runFailure.value = { kind: "cancelled", reason: `cancelled before ${pending.step}` };
      settleSteps("cancelled");
      feed("system", `run cancelled before \`${pending.step}\``, "System");
    }
  } catch (e) {
    runErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    approvalBusy.value = false;
    void refreshRunLists();
  }
}

async function resumeInterruptedRun(run: { run_id: string; prompt: string }) {
  if (resumeBusyId.value) return;
  resumeBusyId.value = run.run_id;
  runErr.value = null;
  try {
    if (selectedHordeWorkers.value.length) {
      const ready = await ensureSelectedHordeReady();
      if (!ready) {
        runErr.value = "Sub-agent workers are not ready — start them under Admin → Federation, then resume again.";
        return;
      }
    }
    resetRunState();
    runBusy.value = true;
    connectStream();
    feed("user", promptLine(run.prompt) || "(interrupted run)", "You");
    feed("orchestrator", `resuming run ${run.run_id}`, "Agent: Boss");
    const out = await api.hordeRunResume(props.hordeId, run.run_id);
    runId.value = out.run.run_id;
    emit("run-started", out.run.run_id);
  } catch (e) {
    runBusy.value = false;
    runErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    resumeBusyId.value = null;
    void refreshRunLists();
  }
}

async function sendFollowup(payload: { prompt: string }) {
  if (!runId.value) return;
  const q = payload.prompt.trim();
  if (!q) return;
  followupMsgs.value = [...followupMsgs.value, { role: "user", speaker: "You", text: q }];
  followupBusy.value = true;
  try {
    const out = await api.hordeFollowup(props.hordeId, { run_id: runId.value, message: q });
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
    followupMsgs.value = [...followupMsgs.value, { role: "assistant", speaker: "System", text: `[error] ${msg}` }];
  } finally {
    followupBusy.value = false;
    saveFollowups();
  }
}

watch(
  () => props.hordeId,
  () => {
    resetRunState();
    runHistory.value = [];
    pastRuns.value = [];
    triggerRows.value = [];
    void Promise.all([loadProfiles(), refreshRunLists(), loadTriggers()]);
    if (props.runId) void openRun(props.runId);
  },
);
watch(
  () => props.runId,
  (id) => {
    if (id === runId.value) return;
    if (id) void openRun(id);
    else resetRunState();
  },
);

let stopPolling: (() => void) | null = null;
let triggerTimer: ReturnType<typeof setInterval> | null = null;
let tick: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  stopPolling = useHordePolling();
  connectStream();
  void Promise.all([loadProfiles(), refreshRunLists(), loadTriggers()]);
  if (props.runId) void openRun(props.runId);
  triggerTimer = setInterval(() => void loadTriggers(), 15000);
  tick = setInterval(() => {
    if (view.value === "running") now.value = Date.now();
  }, 1000);
});

onUnmounted(() => {
  fedEs.value?.close();
  clearRunWatchdog();
  stopPolling?.();
  if (triggerTimer) clearInterval(triggerTimer);
  if (tick) clearInterval(tick);
});
</script>

<template>
  <section class="page horde-page">
    <nav class="crumbs" aria-label="Breadcrumb">
      <a href="?" @click.prevent="emit('go-home')">Hordes</a>
      <span aria-hidden="true">/</span>
      <a v-if="runId && selectedHorde" href="#" @click.prevent="emit('open-run', null)">{{ selectedHorde.display_name }}</a>
      <span v-else aria-current="page">{{ selectedHorde?.display_name ?? hordeId }}</span>
    </nav>

    <template v-if="!selectedHorde">
      <p v-if="!hordesLoaded" class="muted">Loading…</p>
      <div v-else class="empty-state">
        <h3>No horde called “{{ hordeId }}”</h3>
        <p>It may have been renamed or removed.</p>
        <button type="button" class="primary" @click="emit('go-home')">Back to hordes</button>
      </div>
    </template>

    <template v-else>
      <header class="horde-head">
        <HordeIcon :category="selectedHorde.category" :icon="selectedHorde.icon" :size="52" />
        <div class="head-text">
          <template v-if="runId">
            <h1>{{ runTitle }}</h1>
            <p class="head-sub">
              {{ selectedHorde.display_name }}<template v-if="runStartedAt"> · started {{ relativeTime(runStartedAt) }}</template>
              <template v-if="activeRunFromHistory">
                ·
                <span class="run-status" :class="`tone-${runStatusLabel(activeRunFromHistory.status).tone}`">{{
                  runStatusLabel(activeRunFromHistory.status).label
                }}</span>
              </template>
            </p>
          </template>
          <template v-else>
            <h1>{{ selectedHorde.display_name }}</h1>
            <p class="head-sub desc">{{ selectedHorde.description }}</p>
          </template>
        </div>
      </header>

      <p v-if="selectedHorde.load_error" class="note note-warn small">
        The latest edit of this horde failed to load; it runs the last good version. {{ selectedHorde.load_error }}
      </p>

      <section v-if="shownApproval" class="approval-box" role="alert" aria-live="assertive">
        <p class="eyebrow plain approval-tag">Waiting for you // step {{ shownApproval.step }}</p>
        <h2>Approve before the horde continues</h2>
        <p class="approval-text" v-html="inlineMd(shownApproval.text)" />
        <pre v-if="shownApproval.command" class="approval-cmd"><span class="prompt" aria-hidden="true">$ </span>{{ shownApproval.command }}</pre>
        <div class="btn-row">
          <button type="button" class="primary" :disabled="approvalBusy" @click="approvePending(shownApproval)">
            {{ approvalBusy ? "Working…" : "Approve and continue" }}
          </button>
          <button type="button" class="danger" :disabled="approvalBusy" @click="rejectPending(shownApproval)">Cancel run</button>
          <button
            v-if="shownApproval.runId !== runId"
            type="button"
            class="ghost"
            @click="emit('open-run', shownApproval.runId)"
          >
            Show this run
          </button>
        </div>
        <p class="muted small approval-foot">Approval covers this step for the rest of the run.</p>
      </section>

      <div class="horde-layout">
        <div class="main-col">
          <section v-if="bannerRuns.length && view === 'form'" class="resume-banner note note-warn">
            <h3>Interrupted runs</h3>
            <p class="small">These runs did not finish (the server restarted). Completed steps are kept; Resume continues from the next step.</p>
            <article v-for="r in bannerRuns" :key="r.run_id" class="resume-item">
              <div class="resume-meta">
                <span class="resume-prompt">{{ r.title || promptLine(r.prompt) || r.question || "(no prompt)" }}</span>
                <span class="muted tiny">
                  {{ relativeTime(r.started_at) }}{{ (r.resume_count ?? 0) > 0 ? ` · resumed ${r.resume_count}×` : "" }}
                </span>
              </div>
              <button type="button" class="sm" :disabled="resumeBusyId !== null || runBusy" @click="resumeInterruptedRun(r)">
                {{ resumeBusyId === r.run_id ? "Resuming…" : "Resume" }}
              </button>
            </article>
          </section>

          <p v-if="runErr" class="note note-err">{{ runErr }}</p>

          <!-- request form -->
          <section v-if="view === 'form'" class="card accent orders">
            <h2 class="orders-title">Your request</h2>
            <HordeRunForm
              :horde="selectedHorde"
              :disabled="false"
              :busy="runBusy"
              :follow-up-mode="false"
              submit-label="Send in the horde"
              @submit="runHordeWithPayload"
            />
          </section>

          <p v-else-if="view === 'loading'" class="muted">Loading the run…</p>

          <!-- progress (running, or ended early) -->
          <template v-if="view === 'running' || view === 'ended'">
            <section class="card flat progress-card">
              <p class="eyebrow">Pipeline // {{ stepperHeadline }}</p>
              <PipelineStepper :steps="stepperItems" />
            </section>

            <section v-if="view === 'running' && !shownApproval" class="card now-card" aria-live="polite">
              <div class="now-head">
                <p class="eyebrow plain">Now</p>
                <span v-if="nowStep?.since" class="now-time mono">{{ elapsed(now - nowStep.since) }}</span>
              </div>
              <h2 class="now-title">
                <span class="dot-running" aria-hidden="true"></span>
                {{ nowStep ? nowStep.label : "Starting the horde…" }}
              </h2>
              <p v-if="nowStep?.what" class="now-what">{{ nowStep.what }}</p>
              <div class="btn-row">
                <button
                  v-if="runBusy && runId"
                  type="button"
                  class="sm danger"
                  :disabled="cancelBusy"
                  title="The step in flight stops; the remaining steps are skipped"
                  @click="cancelActiveRun"
                >
                  {{ cancelBusy ? "Cancelling…" : "Cancel run" }}
                </button>
              </div>
            </section>

            <section v-if="view === 'ended'" class="note" :class="runFailure?.kind === 'cancelled' ? 'note-info' : 'note-err'">
              <p class="ended-text">
                <strong>{{ runFailure?.kind === "cancelled" || activeRunFromHistory?.status === "cancelled" ? "This run was cancelled." : "This run failed." }}</strong
                >{{ runFailure?.reason ? ` ${sentence(runFailure.reason)}` : "" }}
              </p>
              <div class="btn-row ended-actions">
                <button type="button" class="primary sm" @click="emit('open-run', null)">Start a new request</button>
              </div>
            </section>
          </template>

          <!-- finished: answers first -->
          <template v-if="view === 'done'">
            <section class="card delivery">
              <p class="eyebrow">Delivered</p>
              <template v-if="handoffMarkdown">
                <div class="handoff-rendered md-content" v-html="handoffHtml" />
              </template>
              <p v-else-if="finalShortSummary" class="delivery-summary">{{ finalShortSummary }}</p>
              <p v-else class="muted small">
                This run finished without a hand-off file. Its working files are in the output folder, under
                <code>debug/</code>.
              </p>

              <div class="deliverable">
                <div v-if="primaryArtifact" class="deliverable-file">
                  <span class="file-glyph" aria-hidden="true">▤</span>
                  <div class="file-text">
                    <span class="file-name">{{ primaryArtifact.name }}</span>
                    <code class="file-path">{{ primaryArtifact.path }}</code>
                  </div>
                </div>
                <div class="btn-row">
                  <button type="button" class="primary" @click="openOutputFolder(selectedHorde.workdir || selectedHorde.root_path)">
                    Open output folder
                  </button>
                  <button v-if="primaryArtifact && isAbsolutePath(primaryArtifact.path)" type="button" @click="openOutputFolder(primaryArtifact.path)">
                    Open file
                  </button>
                  <button v-if="handoffMarkdown" type="button" class="ghost" @click="copyPasteToClipboard">Copy as Markdown</button>
                </div>
              </div>
              <p v-if="pathAction" class="muted small">{{ pathAction }}</p>
              <p v-if="copyPasteErr" class="err">{{ copyPasteErr }}</p>
              <p v-if="selectedHorde.delivery_note" class="muted small" v-html="deliveryNoteHtml" />

              <div class="fold-rows">
                <details v-if="otherArtifacts.length">
                  <summary>Intermediate files · {{ otherArtifacts.length }}</summary>
                  <ul class="artifact-list">
                    <li v-for="a in otherArtifacts" :key="`${a[0]}-${a[1]}`">
                      <span class="chip">{{ a[0] }}</span>
                      <code>{{ a[1] }}</code>
                    </li>
                  </ul>
                </details>
                <details>
                  <summary>How it was computed · {{ stepperItems.length }} steps</summary>
                  <PipelineStepper :steps="stepperItems" />
                  <ol v-if="stepSummaries.length" class="step-notes">
                    <li v-for="s in stepSummaries" :key="s.step">
                      <strong>{{ s.label }}</strong>
                      <span v-if="s.summary" class="muted small"> — {{ s.summary }}</span>
                    </li>
                  </ol>
                </details>
                <details v-if="handoffMarkdown">
                  <summary>Markdown source</summary>
                  <textarea readonly class="paste-handoff-markdown" rows="14" spellcheck="false" :value="handoffMarkdown" />
                </details>
                <details>
                  <summary>Show the activity log · {{ runMessages.length }} events</summary>
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
                </details>
                <details v-if="runResult">
                  <summary>Raw payload</summary>
                  <pre class="json">{{ runResult }}</pre>
                </details>
              </div>
            </section>

            <section class="card flat followups">
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
                :disabled="false"
                :busy="followupBusy"
                :follow-up-mode="true"
                @submit="sendFollowup"
              />
              <div class="btn-row redo">
                <button type="button" class="ghost" :disabled="followupBusy" @click="emit('open-run', null)">Start a new request</button>
              </div>
            </section>
          </template>

          <details v-if="(view === 'running' || view === 'ended') && runMessages.length" class="log-fold">
            <summary>Show the activity log · {{ runMessages.length }} events</summary>
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
          </details>
        </div>

        <aside class="side-col" aria-label="About this horde">
          <section class="side-card">
            <h2 class="side-title">Past runs of this horde</h2>
            <ul v-if="pastRuns.length" class="past">
              <li v-for="r in pastRuns" :key="r.run_id">
                <button
                  type="button"
                  class="past-row"
                  :class="{ current: r.run_id === runId }"
                  :aria-current="r.run_id === runId ? 'true' : undefined"
                  @click="emit('open-run', r.run_id)"
                >
                  <span class="past-title">{{ r.title || "Run" }}</span>
                  <span class="past-meta">
                    <span>{{ relativeTime(r.started_at) }}</span>
                    <span class="run-status" :class="`tone-${runStatusLabel(r.status).tone}`">{{ runStatusLabel(r.status).label }}</span>
                  </span>
                </button>
              </li>
            </ul>
            <p v-else class="muted small">No runs yet. Send in the horde and its runs show up here.</p>
            <button v-if="pastRuns.length" type="button" class="link-btn" @click="emit('open-runs', hordeId)">All runs →</button>
          </section>

          <details class="side-fold">
            <summary>How this horde works · {{ selectedHorde.sub_agents.length }} penguins</summary>
            <p v-if="selectedHordeIsDag" class="small">
              Branching horde: steps that share a layer run when all earlier steps finish, possibly in parallel.
            </p>
            <ol class="squad" aria-label="Penguins in this horde">
              <li v-for="a in selectedHorde.sub_agents" :key="a.name" class="squad-member">
                <PenguinAvatar
                  :avatar="a.avatar ?? inferPenguinAvatarId(a.kind, a.name)"
                  :kind="a.kind"
                  :name="a.name"
                  variant="inline"
                  :alt="a.display_name || a.name"
                />
                <span class="sm-text">
                  <span class="sm-name">{{ a.display_name || a.name }}</span>
                  <span class="sm-desc">{{ firstSentence(a.description) || a.kind }}</span>
                </span>
              </li>
            </ol>
          </details>

          <details v-if="triggerRows.length" class="side-fold">
            <summary>Schedules and watchers · {{ triggerRows.length }}</summary>
            <p class="muted small">Declared in the horde's <code>horde.md</code>. Switching one off is kept on the server and survives restarts.</p>
            <article v-for="t in triggerRows" :key="t.index" class="trigger">
              <div class="trigger-line">
                <span class="badge" :class="t.effective_enabled ? 'badge-ok' : 'badge-muted'">{{ t.kind }}</span>
                <code>{{ t.detail }}</code>
              </div>
              <p class="muted small trigger-meta">
                {{ t.effective_enabled ? "On" : "Off" }}{{ t.overridden ? " (changed here)" : "" }}
                <template v-if="t.next_fire"> · next {{ upcomingTime(t.next_fire) }}</template>
                <template v-if="t.last_fired">
                  · last
                  <a href="#" @click.prevent="emit('open-run', t.last_fired.run_id)">{{ relativeTime(t.last_fired.time) }}</a>
                </template>
              </p>
              <div class="btn-row">
                <button type="button" class="sm" :disabled="triggerBusy !== null" @click="toggleTrigger(t)">
                  {{ triggerBusy === t.index ? "…" : t.effective_enabled ? "Switch off" : "Switch on" }}
                </button>
                <button type="button" class="sm" :disabled="triggerBusy !== null" title="Start this trigger's run now" @click="fireTriggerNow(t)">
                  Run now
                </button>
              </div>
            </article>
            <p v-if="triggerNote" class="muted small">{{ triggerNote }}</p>
          </details>

          <details class="side-fold">
            <summary>Output folder</summary>
            <code class="workdir">{{ selectedHorde.workdir || selectedHorde.root_path }}</code>
            <div class="btn-row">
              <button type="button" class="sm" @click="openOutputFolder(selectedHorde.workdir || selectedHorde.root_path)">Open output folder</button>
              <button
                type="button"
                class="sm danger"
                :disabled="cleanWorkdirBusy"
                title="Delete the workdir debug tree, legacy raw/wiki/scratch, agents_log and PASTE_ME.md (same paths as the server's clean-on-startup)"
                @click="cleanSelectedWorkdir"
              >
                {{ cleanWorkdirBusy ? "…" : "Clean working files" }}
              </button>
            </div>
            <p class="muted small">
              Clean on startup: {{ (selectedHorde.config_on_startup_effective ?? selectedHorde.config_on_startup) ? "yes" : "no" }}
            </p>
            <p v-if="pathAction && view !== 'done'" class="muted small">{{ pathAction }}</p>
          </details>
        </aside>
      </div>
    </template>
  </section>
</template>

<style scoped>
.horde-page { max-width: 76rem; }
.tiny { font-size: 0.74rem; }

.horde-head { display: flex; align-items: flex-start; gap: 1rem; margin: 0 0 1.5rem; }
.head-text { min-width: 0; }
.horde-head h1 { margin: 0.1rem 0 0.3rem; font-size: 1.9rem; overflow-wrap: anywhere; }
.head-sub { margin: 0; color: var(--muted); display: flex; flex-wrap: wrap; align-items: center; gap: 0.35rem; }
.head-sub.desc { color: var(--body); max-width: 48rem; display: block; }

/* ---------- approval: unmissable ---------- */
.approval-box {
  background: var(--red-soft);
  border: 1px solid var(--red);
  border-left: 8px solid var(--red);
  border-radius: var(--radius);
  padding: 1.1rem 1.25rem 1rem;
  margin: 0 0 1.5rem;
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

/* ---------- layout ---------- */
.horde-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 340px;
  gap: 2rem;
  align-items: start;
}
.main-col { display: grid; gap: 1rem; min-width: 0; }
.main-col > * { margin: 0; min-width: 0; }

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

/* ---------- request form ---------- */
.orders { padding: 1.25rem 1.4rem 1.4rem; }
.orders-title { font-size: 1.2rem; margin: 0 0 0.8rem; }

/* ---------- running ---------- */
.now-card { border-top-color: var(--red); }
.now-head { display: flex; justify-content: space-between; align-items: baseline; }
.now-head .eyebrow { margin: 0; }
.now-time { font-size: 0.82rem; color: var(--muted); }
.now-title { display: flex; align-items: center; gap: 0.6rem; font-size: 1.35rem; margin: 0.3rem 0 0.35rem; }
.now-what { margin: 0 0 0.75rem; color: var(--body); }
.ended-actions { margin-top: 0.6rem; }
.ended-text { margin: 0; }
.log-fold > summary { padding: 0.4rem 0; }
.log-fold .feed { margin-top: 0.5rem; }

/* ---------- hand-off ---------- */
.delivery { border-top-color: var(--ok); padding: 1.1rem 1.3rem 1.2rem; }
.handoff-rendered {
  line-height: 1.6;
  overflow-x: auto;
  padding: 0 0 0.5rem;
  color: var(--body);
}
.handoff-rendered :deep(h1) { font-size: 1.45rem; margin: 0.4rem 0 0.5rem; }
.handoff-rendered :deep(h2) { font-size: 1.15rem; margin: 1.1rem 0 0.35rem; }
.handoff-rendered :deep(h3) { font-size: 1rem; margin: 1rem 0 0.3rem; }
.delivery-summary { font-size: 1.05rem; color: var(--ink); }
.deliverable {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
  padding: 0.8rem 0.9rem;
  margin: 0.6rem 0 0.6rem;
  background: var(--ok-soft);
  border: 1px solid var(--ok);
  border-radius: var(--radius);
}
.deliverable-file { display: flex; align-items: center; gap: 0.8rem; min-width: 0; }
.file-glyph {
  width: 2.4rem;
  height: 2.4rem;
  display: inline-grid;
  place-items: center;
  font-size: 1.2rem;
  border-radius: var(--radius-sm);
  background: var(--ink);
  color: var(--paper);
  flex: 0 0 auto;
}
.file-text { display: grid; gap: 0.15rem; min-width: 0; }
.file-name { font-family: var(--font-display); font-weight: 800; font-size: 1.1rem; color: var(--ink); }
.file-path { background: transparent; border: 0; padding: 0; font-size: 0.74rem; color: var(--body); }
.fold-rows { margin-top: 0.75rem; border-top: 1px solid var(--hair); }
.fold-rows details { margin: 0; padding: 0.45rem 0; border-bottom: 1px solid var(--hair); }
.fold-rows details:last-child { border-bottom: 0; }
.fold-rows details[open] > summary { margin-bottom: 0.5rem; }
.step-notes { margin: 0.6rem 0 0; padding-left: 1.2rem; display: grid; gap: 0.3rem; font-size: 0.92rem; }
.paste-handoff-markdown {
  width: 100%;
  min-height: 12rem;
  font-family: var(--font-mono);
  font-size: 0.8rem;
  line-height: 1.5;
  background: var(--sunk);
  white-space: pre;
  overflow-x: auto;
}
.artifact-list { list-style: none; padding: 0; margin: 0.4rem 0 0; display: grid; gap: 0.35rem; }
.artifact-list li { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.redo { margin-top: 0.6rem; }

/* ---------- activity feed ---------- */
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
  padding: 0.6rem 0.9rem;
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
.msg-speaker { font-weight: 600; color: var(--ink); font-size: 0.9rem; }
.msg-role { font-family: var(--font-mono); font-size: 0.64rem; letter-spacing: 0.1em; text-transform: uppercase; color: var(--muted); }
.msg-time { margin-left: auto; font-family: var(--font-mono); font-size: 0.7rem; color: var(--muted); }
.msg-text {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--font-body);
  font-size: 0.9rem;
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

/* ---------- aside ---------- */
.side-col { display: grid; gap: 0.75rem; position: sticky; top: calc(56px + 3px + 1.25rem); }
.side-card {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius-lg);
  padding: 0.9rem 1rem 1rem;
}
.side-title {
  font-family: var(--font-mono);
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted);
  margin: 0 0 0.5rem;
}
.past { list-style: none; margin: 0 0 0.6rem; padding: 0; }
.past li + li { border-top: 1px solid var(--hair); }
.past-row {
  width: 100%;
  display: grid;
  gap: 0.15rem;
  justify-items: stretch;
  justify-content: stretch;
  text-align: left;
  padding: 0.5rem 0.4rem;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  font-weight: 400;
  white-space: normal;
}
.past-row:hover:not(:disabled) { background: var(--sunk); }
.past-row:active:not(:disabled) { transform: none; }
.past-row.current { background: var(--sunk); box-shadow: inset 3px 0 0 var(--red); }
.past-title { color: var(--ink); font-weight: 600; font-size: 0.92rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.past-meta { display: flex; justify-content: space-between; gap: 0.5rem; font-size: 0.8rem; color: var(--muted); }
.past-meta .run-status { font-size: 0.8rem; }
.side-fold {
  margin: 0;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius-lg);
  padding: 0.55rem 1rem;
}
.side-fold[open] { padding-bottom: 0.9rem; }
.side-fold[open] > summary { margin-bottom: 0.5rem; }
.squad { list-style: none; padding: 0; margin: 0; display: grid; gap: 0.5rem; }
.squad-member { display: flex; align-items: flex-start; gap: 0.55rem; }
.sm-text { display: grid; line-height: 1.3; min-width: 0; }
.sm-name { font-weight: 600; font-size: 0.9rem; color: var(--ink); }
.sm-desc { font-size: 0.82rem; color: var(--muted); }
.trigger { padding: 0.55rem 0; border-top: 1px solid var(--hair); }
.trigger-line { display: flex; align-items: center; gap: 0.45rem; flex-wrap: wrap; }
.trigger-meta { margin: 0.3rem 0 0.4rem; }
.workdir { display: block; margin-bottom: 0.5rem; }

@media (max-width: 1100px) {
  .horde-layout { grid-template-columns: minmax(0, 1fr); }
  .side-col { position: static; }
}
</style>
