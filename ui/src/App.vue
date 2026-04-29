<script setup lang="ts">
import { onMounted, ref } from "vue";
import SidebarNav from "./components/SidebarNav.vue";
import AboutPanel from "./panels/AboutPanel.vue";
import ChatPanel from "./panels/ChatPanel.vue";
import FederationManagementPanel from "./panels/FederationManagementPanel.vue";
import FederationRunPanel from "./panels/FederationRunPanel.vue";
import GraphPanel from "./panels/GraphPanel.vue";
import HomePanel from "./panels/HomePanel.vue";
import McpPanel from "./panels/McpPanel.vue";
import { api, chatStream } from "./api";

const tab = ref<"home" | "mcp" | "chat" | "federation-management" | "federation-run" | "graph" | "about">("chat");
const sidebarCollapsed = ref(false);

type ChatTurn = { role: "user" | "assistant"; content: string };
type Conversation = {
  id: string;
  title: string;
  sessionId: string | null;
  chatMeta: string | null;
  turns: ChatTurn[];
  updatedAt: number;
};
type HordeInteraction = {
  id: string;
  title: string;
  updatedAt: number;
};

const CHAT_LIST_KEY = "kowalski.ui.chat.list.v2";
const HORDE_LIST_KEY = "kowalski.ui.horde.list.v1";
const conversations = ref<Conversation[]>([]);
const activeConversationId = ref<string | null>(null);
const hordeInteractions = ref<HordeInteraction[]>([]);
const activeHordeInteractionId = ref<string | null>(null);
const chatBusy = ref(false);
const resetBusy = ref(false);
const chatErr = ref<string | null>(null);
const chatToolsStream = ref(false);
const chatUseMemory = ref(true);
const chatMessagesView = ref<string>("");
const chatMessagesBusy = ref(false);
const appVersion = ref<string>("unknown");

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

function persistHordeInteractions() {
  localStorage.setItem(HORDE_LIST_KEY, JSON.stringify(hordeInteractions.value));
}

function restoreHordeInteractions() {
  const raw = localStorage.getItem(HORDE_LIST_KEY);
  if (!raw) return;
  try {
    const parsed = JSON.parse(raw) as HordeInteraction[];
    if (Array.isArray(parsed)) {
      hordeInteractions.value = parsed
        .filter((h) => h && typeof h.id === "string")
        .map((h) => ({ ...h, updatedAt: h.updatedAt ?? Date.now() }));
      if (!activeHordeInteractionId.value && hordeInteractions.value.length) {
        activeHordeInteractionId.value = hordeInteractions.value[0].id;
      }
    }
  } catch {
    /* ignore invalid storage */
  }
}

restoreConversations();
restoreHordeInteractions();
if (!hordeInteractions.value.length) {
  const id = `horde-${Date.now()}`;
  hordeInteractions.value = [{ id, title: "New horde interaction", updatedAt: Date.now() }];
  activeHordeInteractionId.value = id;
  persistHordeInteractions();
}

function newHordeInteraction() {
  const id = `horde-${Date.now()}`;
  const item: HordeInteraction = {
    id,
    title: "New horde interaction",
    updatedAt: Date.now(),
  };
  hordeInteractions.value = [item, ...hordeInteractions.value];
  activeHordeInteractionId.value = id;
  persistHordeInteractions();
}

function selectHordeInteraction(id: string) {
  activeHordeInteractionId.value = id;
}

function upsertHordeInteraction(item: HordeInteraction) {
  const existing = hordeInteractions.value.find((h) => h.id === item.id);
  if (existing) {
    existing.title = item.title;
    existing.updatedAt = item.updatedAt;
  } else {
    hordeInteractions.value.unshift(item);
  }
  hordeInteractions.value = [...hordeInteractions.value].sort((a, b) => b.updatedAt - a.updatedAt);
  persistHordeInteractions();
}

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
      conv.turns.push({ role: "assistant", content: r.reply });
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

onMounted(async () => {
  try {
    const h = await api.health();
    if (h.version) appVersion.value = h.version;
  } catch {
    /* keep unknown */
  }
});
</script>

<template>
  <div class="app shell">
    <SidebarNav
      :active-tab="tab"
      :collapsed="sidebarCollapsed"
      :conversations="conversations"
      :active-conversation-id="activeConversationId"
      :horde-interactions="hordeInteractions"
      :active-horde-interaction-id="activeHordeInteractionId"
      :app-version="appVersion"
      @toggle-collapse="sidebarCollapsed = !sidebarCollapsed"
      @select-tab="tab = $event"
      @select-conversation="selectConversation"
      @new-conversation="newConversation"
      @select-horde-interaction="selectHordeInteraction"
      @new-horde-interaction="newHordeInteraction"
    />
    <main class="main">
      <HomePanel v-if="tab === 'home'" />
      <McpPanel v-else-if="tab === 'mcp'" />
      <ChatPanel
        v-else-if="tab === 'chat'"
        :active-conversation="activeConversation()"
        :chat-busy="chatBusy"
        :reset-busy="resetBusy"
        :chat-err="chatErr"
        :chat-tools-stream="chatToolsStream"
        :chat-use-memory="chatUseMemory"
        :chat-messages-view="chatMessagesView"
        :chat-messages-busy="chatMessagesBusy"
        @toggle-tools-stream="chatToolsStream = $event"
        @toggle-use-memory="chatUseMemory = $event"
        @inspect-chat-messages="inspectChatMessages"
        @send-chat="sendChat"
        @new-conversation="newConversation"
      />
      <FederationManagementPanel v-else-if="tab === 'federation-management'" />
      <FederationRunPanel
        v-else-if="tab === 'federation-run'"
        :active-thread-id="activeHordeInteractionId"
        @thread-upsert="upsertHordeInteraction"
      />
      <GraphPanel v-else-if="tab === 'graph'" />
      <AboutPanel v-else-if="tab === 'about'" />
    </main>
  </div>
</template>

<style>
:root {
  font-family: system-ui, sans-serif;
  color: #e8e8ec;
  background: #12141a;
}
body {
  margin: 0;
}
.app {
  min-height: 100vh;
}
.shell {
  display: flex;
}
.main {
  flex: 1;
  padding: 1.25rem 1.5rem;
  min-width: 0;
  height: 100vh;
  overflow-y: auto;
}
code {
  background: #2a3142;
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
  font-size: 0.88em;
}
</style>
