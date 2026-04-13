<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from "vue";
import { api, chatStream } from "../api";

type ChatTurn = { role: "user" | "assistant"; content: string };
const CHAT_STORAGE_KEY = "kowalski.ui.chat.v1";

const chatIn = ref("");
const chatMeta = ref<string | null>(null);
const sessionId = ref<string | null>(null);
const chatBusy = ref(false);
const resetBusy = ref(false);
const chatErr = ref<string | null>(null);
const chatToolsStream = ref(false);
const chatTurns = ref<ChatTurn[]>([]);
const transcriptEl = ref<HTMLElement | null>(null);

function persistChatState() {
  localStorage.setItem(
    CHAT_STORAGE_KEY,
    JSON.stringify({
      turns: chatTurns.value.slice(-200),
      sessionId: sessionId.value,
      chatMeta: chatMeta.value,
    }),
  );
}

function restoreChatState() {
  const raw = localStorage.getItem(CHAT_STORAGE_KEY);
  if (!raw) return;
  try {
    const parsed = JSON.parse(raw) as {
      turns?: ChatTurn[];
      sessionId?: string | null;
      chatMeta?: string | null;
    };
    if (Array.isArray(parsed.turns)) {
      chatTurns.value = parsed.turns.filter(
        (t) =>
          (t.role === "user" || t.role === "assistant") &&
          typeof t.content === "string",
      );
    }
    sessionId.value = parsed.sessionId ?? null;
    chatMeta.value = parsed.chatMeta ?? null;
  } catch {
    /* ignore bad storage */
  }
}

async function scrollTranscriptBottom() {
  await nextTick();
  if (transcriptEl.value) {
    transcriptEl.value.scrollTop = transcriptEl.value.scrollHeight;
  }
}

async function sendChat() {
  const msg = chatIn.value.trim();
  if (!msg) return;
  chatTurns.value.push({ role: "user", content: msg });
  chatBusy.value = true;
  chatErr.value = null;
  chatMeta.value = null;
  try {
    const r = await api.chat(msg);
    chatMeta.value = `${r.mode} · ${r.model}`;
    chatTurns.value.push({ role: "assistant", content: r.reply });
    chatIn.value = "";
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    chatErr.value = message;
    chatTurns.value.push({ role: "assistant", content: `[error] ${message}` });
  } finally {
    chatBusy.value = false;
    persistChatState();
    void scrollTranscriptBottom();
  }
}

async function sendChatStream() {
  const msg = chatIn.value.trim();
  if (!msg) return;
  chatTurns.value.push({ role: "user", content: msg });
  const assistantTurn: ChatTurn = { role: "assistant", content: "" };
  chatTurns.value.push(assistantTurn);
  chatBusy.value = true;
  chatErr.value = null;
  chatMeta.value = null;
  try {
    await chatStream(
      msg,
      (ev) => {
        if (ev.type === "start") {
          sessionId.value = ev.conversation_id;
          chatMeta.value = chatToolsStream.value
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
      { toolsStream: chatToolsStream.value },
    );
    if (!assistantTurn.content.trim()) assistantTurn.content = "(no assistant output)";
    chatIn.value = "";
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    chatErr.value = message;
    assistantTurn.content = `[error] ${message}`;
  } finally {
    chatBusy.value = false;
    persistChatState();
    void scrollTranscriptBottom();
  }
}

async function resetChat() {
  resetBusy.value = true;
  chatErr.value = null;
  try {
    const r = await api.chatReset();
    sessionId.value = r.conversation_id;
    chatMeta.value = `new session · ${r.model}`;
    chatIn.value = "";
    chatTurns.value = [];
    persistChatState();
  } catch (e) {
    chatErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    resetBusy.value = false;
  }
}

watch(chatTurns, () => {
  void scrollTranscriptBottom();
}, { deep: true });

onMounted(() => {
  restoreChatState();
  void scrollTranscriptBottom();
});
</script>

<template>
  <section class="panel chat-layout">
    <h2>Chat</h2>
    <p class="hint">
      Ask follow-ups in the same session. Use <strong>New conversation</strong> to clear context.
    </p>
    <div class="meta">
      <p v-if="sessionId" class="muted">Session: {{ sessionId }}</p>
      <p v-if="chatMeta" class="muted">{{ chatMeta }}</p>
    </div>

    <div ref="transcriptEl" class="chat-history">
      <article
        v-for="(turn, idx) in chatTurns"
        :key="idx"
        class="chat-turn"
        :class="`turn-${turn.role}`"
      >
        <header>{{ turn.role === "user" ? "You" : "Assistant" }}</header>
        <pre class="chat-turn-content">{{ turn.content }}</pre>
      </article>
      <p v-if="!chatTurns.length" class="muted">(no messages yet)</p>
    </div>

    <div class="composer">
      <label class="chk">
        <input v-model="chatToolsStream" type="checkbox" />
        Tool-aware stream (<code>tools_stream</code>)
      </label>
      <textarea
        v-model="chatIn"
        rows="3"
        class="ta"
        placeholder="Type your message..."
      />
      <p class="actions">
        <button type="button" class="primary" :disabled="chatBusy" @click="sendChat">
          {{ chatBusy ? "Sending..." : "Send" }}
        </button>
        <button type="button" :disabled="chatBusy" @click="sendChatStream">
          {{ chatBusy ? "Sending..." : "Send (SSE)" }}
        </button>
        <button type="button" :disabled="resetBusy" @click="resetChat">
          {{ resetBusy ? "Resetting..." : "New conversation" }}
        </button>
      </p>
      <p v-if="chatErr" class="err">{{ chatErr }}</p>
    </div>
  </section>
</template>

<style scoped>
.panel h2 { margin-top: 0; font-size: 1.1rem; }
.hint { font-size: 0.9rem; color: #8b92a5; margin-bottom: 0.5rem; }
.muted { color: #6a7285; font-size: 0.9rem; margin: 0; }
.chat-layout { display: flex; flex-direction: column; gap: 0.5rem; min-height: calc(100vh - 12rem); }
.meta { min-height: 1.2rem; }
.chat-history {
  flex: 1;
  min-height: 16rem;
  max-height: calc(100vh - 25rem);
  overflow: auto;
  display: grid;
  gap: 0.5rem;
  border: 1px solid #2a2e38;
  border-radius: 8px;
  padding: 0.6rem;
  background: #141820;
}
.chat-turn {
  border: 1px solid #2a2e38;
  border-radius: 8px;
  padding: 0.55rem 0.65rem;
  background: #171b22;
}
.turn-user { border-color: #3d5a8c; }
.chat-turn header { color: #9aa8c0; font-size: 0.8rem; margin-bottom: 0.2rem; }
.chat-turn-content {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  color: #d2d9e8;
  font-size: 0.85rem;
  line-height: 1.4;
}
.composer {
  position: sticky;
  bottom: 0;
  background: #12141a;
  border-top: 1px solid #2a2e38;
  padding-top: 0.5rem;
}
.chk { display: inline-flex; align-items: center; gap: 0.45rem; font-size: 0.9rem; color: #b8c0d0; }
.ta {
  width: 100%;
  box-sizing: border-box;
  background: #1a1d26;
  border: 1px solid #3d4658;
  color: #e8e8ec;
  border-radius: 6px;
  padding: 0.5rem 0.65rem;
  font: inherit;
  margin-top: 0.45rem;
}
.actions { display: flex; gap: 0.5rem; flex-wrap: wrap; margin: 0.5rem 0 0; }
button { background: #2a3142; border: 1px solid #3d4658; color: #c8cfdd; padding: 0.4rem 0.75rem; border-radius: 6px; cursor: pointer; }
button.primary { background: #3d5a8c; border-color: #5a7ab8; color: #fff; }
button:disabled { opacity: 0.6; cursor: not-allowed; }
.err { color: #e88; font-size: 0.9rem; }
code { background: #2a3142; padding: 0.15rem 0.4rem; border-radius: 4px; font-size: 0.88em; }
</style>
