<script setup lang="ts">
import { onMounted, ref } from "vue";
import SidebarNav from "./components/SidebarNav.vue";
import AboutPanel from "./panels/AboutPanel.vue";
import ChatPanel from "./panels/ChatPanel.vue";
import FederationPanel from "./panels/FederationPanel.vue";
import GraphPanel from "./panels/GraphPanel.vue";
import HomePanel from "./panels/HomePanel.vue";
import McpPanel from "./panels/McpPanel.vue";
import { api, chatStream } from "./api";

const tab = ref<"home" | "mcp" | "chat" | "federation" | "graph" | "about">("chat");
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

const CHAT_LIST_KEY = "kowalski.ui.chat.list.v2";
const conversations = ref<Conversation[]>([]);
const activeConversationId = ref<string | null>(null);
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

restoreConversations();

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
  conv.turns.push({ role: "user", content: msg });
  chatBusy.value = true;
  chatErr.value = null;
  conv.chatMeta = null;
  try {
    if (payload.stream) {
      const assistantTurn: ChatTurn = { role: "assistant", content: "" };
      conv.turns.push(assistantTurn);
      await chatStream(
        msg,
        (ev) => {
          if (ev.type === "start") {
            conv!.sessionId = ev.conversation_id;
            conv!.chatMeta = chatToolsStream.value
              ? `SSE · tools_stream · ${ev.model}`
              : `SSE · ${ev.model}`;
          } else if (ev.type === "token") {
            assistantTurn.content += ev.content;
          } else if (ev.type === "assistant") {
            assistantTurn.content = ev.content;
          } else if (ev.type === "error") {
            chatErr.value = ev.message;
            assistantTurn.content = `[error] ${ev.message}`;
          }
        },
        { toolsStream: chatToolsStream.value, useMemory: chatUseMemory.value },
      );
      if (!assistantTurn.content.trim()) assistantTurn.content = "(no assistant output)";
    } else {
      const r = await api.chat(msg, { useMemory: chatUseMemory.value });
      conv.chatMeta = `${r.mode} · ${r.model}`;
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
    const payload = await api.chatMessages();
    chatMessagesView.value = JSON.stringify(payload, null, 2);
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    chatErr.value = message;
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
      :app-version="appVersion"
      @toggle-collapse="sidebarCollapsed = !sidebarCollapsed"
      @select-tab="tab = $event"
      @select-conversation="selectConversation"
      @new-conversation="newConversation"
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
      <FederationPanel v-else-if="tab === 'federation'" />
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
}
code {
  background: #2a3142;
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
  font-size: 0.88em;
}
</style>
