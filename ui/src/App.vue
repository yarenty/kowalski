<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import CommandPalette from "./components/CommandPalette.vue";
import ThreadList from "./components/ThreadList.vue";
import TopBar from "./components/TopBar.vue";
import AboutPanel from "./panels/AboutPanel.vue";
import ChatPanel from "./panels/ChatPanel.vue";
import FederationManagementPanel from "./panels/FederationManagementPanel.vue";
import FederationRunPanel from "./panels/FederationRunPanel.vue";
import GraphPanel from "./panels/GraphPanel.vue";
import HomePanel from "./panels/HomePanel.vue";
import HordesHomePanel from "./panels/HordesHomePanel.vue";
import McpPanel from "./panels/McpPanel.vue";
import RookeryPanel, { type RookeryUiSession } from "./panels/RookeryPanel.vue";
import RunsPanel from "./panels/RunsPanel.vue";
import SetupPanel from "./panels/SetupPanel.vue";
import {
  api,
  chatStream,
  getApiToken,
  rookeryChatStream,
  setApiToken,
  type RookerySessionResponse,
  type RookerySessionStatus,
  type RunFilter,
  type RunSummary,
} from "./api";
import { routeFromUrl, urlForRoute, type Route, type TabId } from "./nav";

/** Where the operator is (tab + open horde page / run), mirrored into the URL. */
const route = ref<Route>(routeFromUrl());
const tab = computed(() => route.value.tab);
/** Runs page filters when it is opened from a link ("All runs →", the needs-you strip). */
const runsPreset = ref<{ filter: RunFilter; horde: string | null }>({ filter: "all", horde: null });

function navigate(next: Partial<Route>, opts: { replace?: boolean } = {}) {
  const r: Route = { tab: route.value.tab, horde: route.value.horde, run: route.value.run, ...next };
  if (r.tab !== "federation-run") {
    r.horde = null;
    r.run = null;
  }
  if (!r.horde) r.run = null;
  route.value = r;
  const url = urlForRoute(r);
  if (url !== window.location.pathname + window.location.search + window.location.hash) {
    if (opts.replace) window.history.replaceState(null, "", url);
    else window.history.pushState(null, "", url);
  }
}
function onPopState() {
  route.value = routeFromUrl();
}

function selectTab(next: TabId) {
  if (next === "runs") runsPreset.value = { filter: "all", horde: null };
  navigate({ tab: next, horde: null, run: null });
  window.scrollTo(0, 0);
}
function openHorde(id: string) {
  navigate({ tab: "federation-run", horde: id, run: null });
  window.scrollTo(0, 0);
}
function openRun(hordeId: string, runId: string | null) {
  navigate({ tab: "federation-run", horde: hordeId, run: runId });
  window.scrollTo(0, 0);
}
function openRuns(filter: RunFilter = "all", horde: string | null = null) {
  runsPreset.value = { filter, horde };
  navigate({ tab: "runs", horde: null, run: null });
  window.scrollTo(0, 0);
}

// ---------- ⌘K picker ----------
const pickerOpen = ref(false);
function onGlobalKey(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && !e.altKey && !e.shiftKey && e.key.toLowerCase() === "k") {
    e.preventDefault();
    pickerOpen.value = !pickerOpen.value;
  }
}

// ---------- runs waiting for the operator (home strip + top bar badge) ----------
const waitingRun = ref<RunSummary | null>(null);
const waitingCount = ref(0);
async function loadWaiting() {
  try {
    const r = await api.runs({ status: "needs_you", limit: 1 });
    waitingRun.value = r.runs[0] ?? null;
    waitingCount.value = r.counts.needs_you;
  } catch {
    waitingRun.value = null;
    waitingCount.value = 0;
  }
}
let waitTimer: ReturnType<typeof setInterval> | null = null;

type ChatTurn = { role: "user" | "assistant"; content: string };
type Conversation = {
  id: string;
  title: string;
  sessionId: string | null;
  chatMeta: string | null;
  turns: ChatTurn[];
  updatedAt: number;
};

const CHAT_LIST_KEY = "kowalski.ui.chat.list.v2";
const ROOKERY_LIST_KEY = "kowalski.ui.rookery.list.v1";
const conversations = ref<Conversation[]>([]);
const activeConversationId = ref<string | null>(null);
const chatBusy = ref(false);
const resetBusy = ref(false);
const chatErr = ref<string | null>(null);
// Tool-aware chat is the default, so connected tools (tableski, web, files) get used; the
// operator's choice is remembered per browser.
const CHAT_TOOLS_KEY = "kowalski.ui.chat.tools.v1";
const chatToolsStream = ref(localStorage.getItem(CHAT_TOOLS_KEY) !== "off");
/** Template handlers cannot reach `localStorage` (not a template global), so persist here. */
function setChatToolsStream(v: boolean) {
  chatToolsStream.value = v;
  try {
    localStorage.setItem(CHAT_TOOLS_KEY, v ? "on" : "off");
  } catch {
    /* storage unavailable */
  }
}
const chatUseMemory = ref(true);
const chatMessagesView = ref<string>("");
const chatMessagesBusy = ref(false);
const appVersion = ref<string>("unknown");
const rookerySessions = ref<RookeryUiSession[]>([]);
const activeRookerySessionId = ref<string | null>(null);
const rookeryBusy = ref(false);
const rookeryNewBusy = ref(false);
const rookeryProposeBusy = ref(false);
const rookeryBirthBusy = ref(false);
const rookerySaveHordeBusy = ref(false);
const rookeryPenguinSaveBusy = ref(false);
const rookeryValidateBusy = ref(false);
const rookeryErr = ref<string | null>(null);
const rookeryValidateNote = ref<string | null>(null);
const rookeryBirthOverwrite = ref(false);

function persistConversations() {
  localStorage.setItem(CHAT_LIST_KEY, JSON.stringify(conversations.value));
}

function restoreConversations() {
  const raw = localStorage.getItem(CHAT_LIST_KEY);
  if (!raw) return;
  try {
    const parsed = JSON.parse(raw) as Conversation[];
    if (Array.isArray(parsed)) {
      conversations.value = parsed
        .filter((c) => c && typeof c.id === "string" && Array.isArray(c.turns))
        .map((c) => ({ ...c, updatedAt: c.updatedAt ?? Date.now() }));
    }
  } catch {
    /* ignore invalid storage */
  }
}

function persistRookerySessions() {
  // The server owns the draft/status/pipeline/summary (PLAN.md §R1). Persist only a thin
  // session-id list (plus local display turns/title); server-owned state is fetched via GET.
  const minimal = rookerySessions.value.map((r) => ({
    id: r.id,
    serverSessionId: r.serverSessionId,
    title: r.title,
    turns: r.turns,
    updatedAt: r.updatedAt,
  }));
  localStorage.setItem(ROOKERY_LIST_KEY, JSON.stringify(minimal));
}

function restoreRookerySessions() {
  const raw = localStorage.getItem(ROOKERY_LIST_KEY);
  if (!raw) return;
  try {
    const parsed = JSON.parse(raw) as Array<Partial<RookeryUiSession>>;
    if (Array.isArray(parsed)) {
      rookerySessions.value = parsed
        .filter((r) => r && typeof r.id === "string" && typeof r.serverSessionId === "string")
        .map((r) => ({
          id: r.id as string,
          serverSessionId: r.serverSessionId as string,
          title: r.title ?? "Rookery session",
          turns: Array.isArray(r.turns) ? r.turns : [],
          // Server-owned fields are placeholders until hydrated from GET on select.
          status: "interviewing" as RookerySessionStatus,
          summary: null,
          pipeline: [],
          draft: null,
          hordeRoot: null,
          outputRoot: null,
          birthNote: null,
          parseError: null,
          updatedAt: r.updatedAt ?? Date.now(),
        }));
      if (!activeRookerySessionId.value && rookerySessions.value.length) {
        activeRookerySessionId.value = rookerySessions.value[0].id;
      }
      if (activeRookerySessionId.value) {
        void refreshRookerySession(activeRookerySessionId.value);
      }
    }
  } catch {
    /* ignore */
  }
}

function applyRookeryServerState(local: RookeryUiSession, remote: RookerySessionResponse) {
  local.status = remote.status;
  local.summary = remote.summary;
  local.pipeline = remote.pipeline ?? [];
  local.draft = remote.draft ?? null;
  local.hordeRoot = remote.horde_root;
  local.outputRoot = remote.output_root;
  if (remote.draft?.display_name && (local.title === "New Rookery session" || !local.title)) {
    local.title = remote.draft.display_name;
  }
}

function activeRookerySession(): RookeryUiSession | null {
  if (!activeRookerySessionId.value) return null;
  return rookerySessions.value.find((r) => r.id === activeRookerySessionId.value) ?? null;
}

function isRookerySessionNotFound(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return /session not found/i.test(message);
}

/**
 * Ensure the backend still has this session. The server persists sessions across restarts
 * (PLAN.md §R1), so this normally just confirms existence. If the server genuinely lost the
 * session (e.g. its state dir was cleared), create a fresh empty one — the client no longer
 * round-trips the draft/history, since the server is the source of truth.
 */
async function ensureRookeryBackendSession(session: RookeryUiSession): Promise<boolean> {
  try {
    await api.rookerySession(session.serverSessionId);
    return false;
  } catch (e) {
    if (!isRookerySessionNotFound(e)) throw e;
  }
  const r = await api.rookeryCreateSession();
  session.serverSessionId = r.session.session_id;
  session.turns = [];
  applyRookeryServerState(session, r.session);
  persistRookerySessions();
  return true;
}

restoreConversations();
restoreRookerySessions();

function activeConversation(): Conversation | null {
  if (!activeConversationId.value) return null;
  return conversations.value.find((c) => c.id === activeConversationId.value) ?? null;
}

function createConversation(): Conversation {
  const id = `conv-${Date.now()}`;
  return {
    id,
    title: "New conversation",
    sessionId: null,
    chatMeta: null,
    turns: [],
    updatedAt: Date.now(),
  };
}

async function newConversation() {
  resetBusy.value = true;
  chatErr.value = null;
  const conv = createConversation();
  conversations.value.unshift(conv);
  activeConversationId.value = conv.id;
  try {
    const r = await api.chatReset();
    conv.sessionId = r.conversation_id;
    conv.chatMeta = `new session · ${r.model}`;
  } catch (e) {
    chatErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    conv.updatedAt = Date.now();
    persistConversations();
    resetBusy.value = false;
  }
}

async function sendChat(payload: { message: string; stream: boolean }) {
  let conv = activeConversation();
  if (!conv) {
    conv = createConversation();
    conversations.value.unshift(conv);
    activeConversationId.value = conv.id;
  }
  const msg = payload.message.trim();
  if (!msg) return;
  const priorTurns = [...conv.turns];
  conv.turns.push({ role: "user", content: msg });
  chatBusy.value = true;
  chatErr.value = null;
  conv.chatMeta = null;
  try {
    const isConversationNotFound = (error: unknown): boolean => {
      const message = error instanceof Error ? error.message : String(error);
      return message.includes("conversation not found");
    };

    const ensureBackendSession = async (): Promise<void> => {
      const r = await api.chatReset();
      conv!.sessionId = r.conversation_id;
      conv!.chatMeta = `new session · ${r.model}`;
      const syncMessages = [
        { role: "system", content: "You are a helpful assistant.", tool_calls: null },
        ...priorTurns.map((t) => ({ role: t.role, content: t.content, tool_calls: null })),
      ];
      await api.chatSync(syncMessages, conv!.sessionId);
    };

    if (payload.stream) {
      const assistantTurn: ChatTurn = { role: "assistant", content: "" };
      conv.turns.push(assistantTurn);
      const runStream = async () => {
        let streamConversationNotFound = false;
        await chatStream(
          msg,
          (ev) => {
            if (ev.type === "start") {
              conv!.sessionId = ev.conversation_id;
              const memMeta =
                ev.memory_source !== undefined
                  ? ` · memory=${ev.memory_source}:${ev.memory_items_count ?? 0}`
                  : "";
              conv!.chatMeta = chatToolsStream.value
                ? `SSE · tools_stream · ${ev.model}${memMeta}`
                : `SSE · ${ev.model}${memMeta}`;
            } else if (ev.type === "token") {
              assistantTurn.content += ev.content;
            } else if (ev.type === "assistant") {
              assistantTurn.content = ev.content;
            } else if (ev.type === "error") {
              chatErr.value = ev.message;
              assistantTurn.content = `[error] ${ev.message}`;
              if (ev.message.includes("conversation not found")) {
                streamConversationNotFound = true;
              }
            }
          },
          {
            toolsStream: chatToolsStream.value,
            useMemory: chatUseMemory.value,
            conversationId: conv!.sessionId,
          },
        );
        if (streamConversationNotFound) {
          throw new Error(assistantTurn.content.replace(/^\[error\]\s*/, ""));
        }
      };
      try {
        await runStream();
      } catch (e) {
        if (!isConversationNotFound(e)) throw e;
        await ensureBackendSession();
        assistantTurn.content = "";
        chatErr.value = null;
        await runStream();
      }
      if (!assistantTurn.content.trim()) assistantTurn.content = "(no assistant output)";
    } else {
      const runChat = async () =>
        api.chat(msg, {
          useMemory: chatUseMemory.value,
          conversationId: conv!.sessionId,
        });
      let r;
      try {
        r = await runChat();
      } catch (e) {
        if (!isConversationNotFound(e)) throw e;
        await ensureBackendSession();
        r = await runChat();
      }
      conv.chatMeta = `${r.mode} · ${r.model} · memory=${r.memory_source}:${r.memory_items_count}`;
      conv.turns.push({ role: "assistant", content: r.reply || "(no assistant output)" });
    }
    if (!conv.title || conv.title === "New conversation") {
      conv.title = msg.slice(0, 42) || "Conversation";
    }
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    chatErr.value = message;
    conv.turns.push({ role: "assistant", content: `[error] ${message}` });
  } finally {
    conv.updatedAt = Date.now();
    conversations.value = [...conversations.value].sort((a, b) => b.updatedAt - a.updatedAt);
    persistConversations();
    chatBusy.value = false;
  }
}

async function inspectChatMessages() {
  chatMessagesBusy.value = true;
  chatErr.value = null;
  try {
    const payload = await api.chatMessages(activeConversation()?.sessionId ?? null);
    chatMessagesView.value = JSON.stringify(payload, null, 2);
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    if (message.includes("conversation not found")) {
      chatErr.value =
        "Conversation no longer exists on backend (likely server restart). Send a message to auto-create a fresh backend session for this thread.";
    } else {
      chatErr.value = message;
    }
    chatMessagesView.value = "";
  } finally {
    chatMessagesBusy.value = false;
  }
}

function selectConversation(id: string) {
  activeConversationId.value = id;
}

async function newRookerySession() {
  rookeryNewBusy.value = true;
  rookeryErr.value = null;
  try {
    const r = await api.rookeryCreateSession();
    const localId = `rookery-ui-${Date.now()}`;
    const item: RookeryUiSession = {
      id: localId,
      serverSessionId: r.session.session_id,
      title: "New Rookery session",
      turns: [],
      status: r.session.status,
      summary: r.session.summary,
      pipeline: r.session.pipeline,
      draft: r.session.draft,
      hordeRoot: r.session.horde_root,
      outputRoot: r.session.output_root,
      birthNote: null,
      parseError: null,
      updatedAt: Date.now(),
    };
    rookerySessions.value = [item, ...rookerySessions.value];
    activeRookerySessionId.value = localId;
    persistRookerySessions();
  } catch (e) {
    rookeryErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    rookeryNewBusy.value = false;
  }
}

function selectRookerySession(id: string) {
  activeRookerySessionId.value = id;
  void refreshRookerySession(id);
}

async function refreshRookerySession(localId: string) {
  const session = rookerySessions.value.find((r) => r.id === localId);
  if (!session) return;
  try {
    const reconnected = await ensureRookeryBackendSession(session);
    if (!reconnected) {
      const remote = await api.rookerySession(session.serverSessionId);
      applyRookeryServerState(session, remote);
    }
    session.updatedAt = Date.now();
    persistRookerySessions();
    rookeryErr.value = null;
  } catch (e) {
    rookeryErr.value = e instanceof Error ? e.message : String(e);
  }
}

function deleteRookerySession(id: string) {
  const session = rookerySessions.value.find((r) => r.id === id);
  rookerySessions.value = rookerySessions.value.filter((r) => r.id !== id);
  if (session) {
    void api.rookeryDeleteSession(session.serverSessionId).catch(() => {
      /* best effort */
    });
  }
  if (!rookerySessions.value.length) {
    activeRookerySessionId.value = null;
  } else if (activeRookerySessionId.value === id) {
    activeRookerySessionId.value = rookerySessions.value[0].id;
  }
  persistRookerySessions();
}

async function sendRookeryChat(message: string) {
  let session = activeRookerySession();
  if (!session) {
    await newRookerySession();
    session = activeRookerySession();
  }
  if (!session) return;
  const msg = message.trim();
  if (!msg) return;
  session.turns.push({ role: "user", content: msg });
  rookeryBusy.value = true;
  rookeryErr.value = null;
  const assistantTurn = { role: "assistant" as const, content: "" };
  session.turns.push(assistantTurn);
  try {
    await ensureRookeryBackendSession(session);
    await rookeryChatStream(session.serverSessionId, msg, (ev) => {
      if (ev.type === "token") {
        assistantTurn.content += ev.content;
      } else if (ev.type === "assistant") {
        assistantTurn.content = ev.content;
      } else if (ev.type === "error") {
        rookeryErr.value = ev.message;
        assistantTurn.content = `[error] ${ev.message}`;
      }
    });
    if (!assistantTurn.content.trim()) {
      const r = await api.rookeryChat(session.serverSessionId, msg);
      assistantTurn.content = r.reply;
      applyRookeryServerState(session, r.session);
    } else {
      const remote = await api.rookerySession(session.serverSessionId);
      applyRookeryServerState(session, remote);
    }
    if (session.title === "New Rookery session") {
      session.title = msg.slice(0, 42) || session.title;
    }
  } catch (e) {
    const errMsg = e instanceof Error ? e.message : String(e);
    rookeryErr.value = errMsg;
    assistantTurn.content = `[error] ${errMsg}`;
  } finally {
    session.updatedAt = Date.now();
    rookerySessions.value = [...rookerySessions.value].sort((a, b) => b.updatedAt - a.updatedAt);
    persistRookerySessions();
    rookeryBusy.value = false;
  }
}

async function proposeRookery() {
  const session = activeRookerySession();
  if (!session) return;
  rookeryProposeBusy.value = true;
  rookeryErr.value = null;
  try {
    await ensureRookeryBackendSession(session);
    const r = await api.rookeryPropose(session.serverSessionId);
    applyRookeryServerState(session, r.session);
    session.parseError = r.parse_error;
    if (r.session.summary) session.summary = r.session.summary;
  } catch (e) {
    rookeryErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    session.updatedAt = Date.now();
    persistRookerySessions();
    rookeryProposeBusy.value = false;
  }
}

async function savePenguinRookery(payload: {
  name: string;
  patch: {
    kind: string;
    display_name: string;
    description: string;
    prompt_body: string;
    agent_body: string | null;
    output: string;
    context_paths: string[];
    tool_ids: string[];
    model_id: string | null;
    avatar: string;
  };
}) {
  const session = activeRookerySession();
  if (!session) return;
  rookeryPenguinSaveBusy.value = true;
  rookeryErr.value = null;
  try {
    await ensureRookeryBackendSession(session);
    const body: Parameters<typeof api.rookeryPatchPenguin>[2] = {
      kind: payload.patch.kind,
      display_name: payload.patch.display_name,
      description: payload.patch.description,
      prompt_body: payload.patch.prompt_body,
      output: payload.patch.output,
      context_paths: payload.patch.context_paths,
      tool_ids: payload.patch.tool_ids,
      avatar: payload.patch.avatar,
    };
    if (payload.patch.agent_body) {
      body.agent_body = payload.patch.agent_body;
    } else {
      body.clear_agent_body = true;
    }
    if (payload.patch.model_id) {
      body.model_id = payload.patch.model_id;
    } else {
      body.clear_model_id = true;
    }
    const r = await api.rookeryPatchPenguin(
      session.serverSessionId,
      payload.name,
      body,
    );
    applyRookeryServerState(session, r.session);
  } catch (e) {
    rookeryErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    session.updatedAt = Date.now();
    persistRookerySessions();
    rookeryPenguinSaveBusy.value = false;
  }
}

async function validateRookeryDraft() {
  const session = activeRookerySession();
  if (!session) return;
  rookeryValidateBusy.value = true;
  rookeryErr.value = null;
  rookeryValidateNote.value = null;
  try {
    await ensureRookeryBackendSession(session);
    const r = await api.rookeryValidateDraft(session.serverSessionId);
    applyRookeryServerState(session, r.session);
    rookeryValidateNote.value = r.ok
      ? "Draft validates OK."
      : `Validation failed: ${r.errors ?? "unknown"}`;
    if (!r.ok) rookeryErr.value = r.errors ?? "validation failed";
  } catch (e) {
    rookeryErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    session.updatedAt = Date.now();
    persistRookerySessions();
    rookeryValidateBusy.value = false;
  }
}

async function saveHordeRookery() {
  const session = activeRookerySession();
  if (!session) return;
  rookerySaveHordeBusy.value = true;
  rookeryErr.value = null;
  try {
    await ensureRookeryBackendSession(session);
    const r = await api.rookerySaveHorde(session.serverSessionId);
    applyRookeryServerState(session, r.session);
    session.hordeRoot = r.horde_root;
    session.birthNote = r.validate_ok
      ? `Saved OK · horde id ${r.horde_id}`
      : `Saved with validate errors: ${r.validate_errors ?? "unknown"}`;
    if (!r.validate_ok) rookeryErr.value = r.validate_errors ?? "validate failed";
  } catch (e) {
    rookeryErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    session.updatedAt = Date.now();
    persistRookerySessions();
    rookerySaveHordeBusy.value = false;
  }
}

async function giveBirthRookery() {
  const session = activeRookerySession();
  if (!session) return;
  rookeryBirthBusy.value = true;
  rookeryErr.value = null;
  try {
    await ensureRookeryBackendSession(session);
    const r = await api.rookeryGiveBirth(session.serverSessionId, {
      overwrite: rookeryBirthOverwrite.value,
    });
    applyRookeryServerState(session, r.session);
    session.hordeRoot = r.horde_root;
    session.birthNote = r.validate_ok
      ? `Validated OK · horde id ${r.horde_id}`
      : `Born with validate errors: ${r.validate_errors ?? "unknown"}`;
    if (!r.validate_ok) rookeryErr.value = r.validate_errors ?? "validate failed";
  } catch (e) {
    rookeryErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    session.updatedAt = Date.now();
    persistRookerySessions();
    rookeryBirthBusy.value = false;
  }
}

/** First-run token prompt: `/api/health` is open, everything else needs the bearer token. */
async function ensureApiToken() {
  try {
    await api.agents();
    return;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (!msg.startsWith("401")) return;
  }
  const entered = window.prompt(
    "Kowalski API token required (see the server log for the db/api_token file path — " +
      "you can also set it later under Admin → Diagnostics):",
    getApiToken(),
  );
  if (entered?.trim()) setApiToken(entered.trim());
}

onMounted(async () => {
  try {
    const h = await api.health();
    if (h.version) appVersion.value = h.version;
  } catch {
    /* keep unknown */
  }
  await ensureApiToken();
  // first run (no config yet) or returning from the tableski sign-in: the setup screen
  const fromOAuth = new URLSearchParams(window.location.search).has("setup");
  try {
    const s = await api.setupStatus();
    if (!s.configured || fromOAuth) navigate({ tab: "setup" }, { replace: true });
  } catch {
    if (fromOAuth) navigate({ tab: "setup" }, { replace: true });
  }
  void loadWaiting();
  waitTimer = setInterval(() => void loadWaiting(), 15000);
});

onMounted(() => {
  window.addEventListener("popstate", onPopState);
  window.addEventListener("keydown", onGlobalKey);
});
onUnmounted(() => {
  window.removeEventListener("popstate", onPopState);
  window.removeEventListener("keydown", onGlobalKey);
  if (waitTimer) clearInterval(waitTimer);
});
</script>

<template>
  <div class="app">
    <TopBar
      :active-tab="tab"
      :app-version="appVersion"
      :needs-you="waitingCount"
      @select-tab="selectTab"
      @open-picker="pickerOpen = true"
    />
    <main class="main" :class="{ 'with-threads': tab === 'chat' || tab === 'rookery' }">
      <SetupPanel v-if="tab === 'setup'" @done="appVersion = appVersion" />
      <HomePanel v-else-if="tab === 'home'" />
      <McpPanel v-else-if="tab === 'mcp'" />
      <template v-else-if="tab === 'rookery'">
        <ThreadList
          label="Build sessions"
          new-label="New build session"
          empty-text="No build sessions yet."
          :items="rookerySessions"
          :active-id="activeRookerySessionId"
          deletable
          @select="selectRookerySession"
          @new="newRookerySession"
          @delete="deleteRookerySession"
        />
        <RookeryPanel
          :active-session="activeRookerySession()"
          :chat-busy="rookeryBusy"
          :propose-busy="rookeryProposeBusy"
          :birth-busy="rookeryBirthBusy"
          :save-horde-busy="rookerySaveHordeBusy"
          :penguin-save-busy="rookeryPenguinSaveBusy"
          :validate-busy="rookeryValidateBusy"
          :validate-note="rookeryValidateNote"
          :new-busy="rookeryNewBusy"
          :err="rookeryErr"
          :birth-overwrite="rookeryBirthOverwrite"
          @send-chat="sendRookeryChat"
          @propose="proposeRookery"
          @validate-draft="validateRookeryDraft"
          @give-birth="giveBirthRookery"
          @save-horde="saveHordeRookery"
          @save-penguin="savePenguinRookery"
          @new-session="newRookerySession"
          @open-horde="selectTab('federation-run')"
          @toggle-birth-overwrite="rookeryBirthOverwrite = $event"
        />
      </template>
      <template v-else-if="tab === 'chat'">
        <ThreadList
          label="Conversations"
          new-label="New conversation"
          empty-text="No conversations yet."
          :items="conversations"
          :active-id="activeConversationId"
          @select="selectConversation"
          @new="newConversation"
        />
        <ChatPanel
          :active-conversation="activeConversation()"
          :chat-busy="chatBusy"
          :reset-busy="resetBusy"
          :chat-err="chatErr"
          :chat-tools-stream="chatToolsStream"
          :chat-use-memory="chatUseMemory"
          :chat-messages-view="chatMessagesView"
          :chat-messages-busy="chatMessagesBusy"
          @toggle-tools-stream="setChatToolsStream"
          @toggle-use-memory="chatUseMemory = $event"
          @inspect-chat-messages="inspectChatMessages"
          @send-chat="sendChat"
          @new-conversation="newConversation"
        />
      </template>
      <FederationManagementPanel v-else-if="tab === 'federation-management'" @new-chat-session="newConversation" />
      <RunsPanel
        v-else-if="tab === 'runs'"
        :initial-filter="runsPreset.filter"
        :initial-horde="runsPreset.horde"
        @open-run="openRun"
      />
      <FederationRunPanel
        v-else-if="tab === 'federation-run' && route.horde"
        :horde-id="route.horde"
        :run-id="route.run"
        @go-home="selectTab('federation-run')"
        @open-setup="selectTab('setup')"
        @open-run="openRun(route.horde!, $event)"
        @run-started="navigate({ run: $event }, { replace: true })"
        @open-runs="openRuns('all', $event)"
        @open-build="selectTab('rookery')"
        @new-chat-session="newConversation"
        @runs-changed="loadWaiting"
      />
      <HordesHomePanel
        v-else-if="tab === 'federation-run'"
        :waiting="waitingRun"
        :waiting-count="waitingCount"
        @open-horde="openHorde"
        @open-run="openRun"
        @open-runs="openRuns"
        @open-build="selectTab('rookery')"
        @open-picker="pickerOpen = true"
      />
      <GraphPanel v-else-if="tab === 'graph'" />
      <AboutPanel v-else-if="tab === 'about'" />
    </main>
    <CommandPalette v-if="pickerOpen" @close="pickerOpen = false" @open-horde="openHorde" @open-run="openRun" />
  </div>
</template>

<style>
.app {
  min-height: 100vh;
  background: var(--paper);
}
.main {
  min-width: 0;
  padding: 1.75rem 2.25rem 3rem;
}
.main.with-threads {
  display: grid;
  grid-template-columns: 15rem minmax(0, 1fr);
  gap: 1.5rem;
  align-items: start;
}
@media (max-width: 1100px) {
  .main {
    padding: 1.25rem 1.25rem 2rem;
  }
}
@media (max-width: 900px) {
  .main.with-threads {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
