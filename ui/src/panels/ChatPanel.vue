<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import DOMPurify from "dompurify";
import { marked } from "marked";

type ChatTurn = { role: "user" | "assistant"; content: string };
type Conversation = {
  id: string;
  title: string;
  sessionId: string | null;
  chatMeta: string | null;
  turns: ChatTurn[];
};

const props = defineProps<{
  activeConversation: Conversation | null;
  chatBusy: boolean;
  resetBusy: boolean;
  chatErr: string | null;
  chatToolsStream: boolean;
}>();

const emit = defineEmits<{
  (e: "send-chat", payload: { message: string; stream: boolean }): void;
  (e: "new-conversation"): void;
  (e: "toggle-tools-stream", value: boolean): void;
}>();

const chatIn = ref("");
const transcriptEl = ref<HTMLElement | null>(null);

async function revealLatestAssistantTurn() {
  await nextTick();
  const root = transcriptEl.value;
  if (!root) return;
  const turns = root.querySelectorAll<HTMLElement>(".chat-turn");
  if (!turns.length) return;
  const last = turns[turns.length - 1];
  if (last.classList.contains("turn-assistant")) {
    root.scrollTop = root.scrollHeight;
  } else {
    root.scrollTop = root.scrollHeight;
  }
}

function renderAssistantMarkdown(content: string): string {
  const html = marked.parse(content, { breaks: true, gfm: true }) as string;
  const safe = DOMPurify.sanitize(html);
  const container = document.createElement("div");
  container.innerHTML = safe;
  const codeBlocks = container.querySelectorAll("pre > code");
  codeBlocks.forEach((codeEl) => {
    const pre = codeEl.parentElement;
    if (!pre || !pre.parentElement) return;
    const wrap = document.createElement("div");
    wrap.className = "code-block-wrap";
    const btn = document.createElement("button");
    btn.className = "copy-code-btn";
    btn.type = "button";
    btn.textContent = "Copy";
    pre.parentElement.replaceChild(wrap, pre);
    wrap.appendChild(btn);
    wrap.appendChild(pre);
  });
  return container.innerHTML;
}

async function onTranscriptClick(ev: MouseEvent) {
  const target = ev.target as HTMLElement | null;
  if (!target || !target.classList.contains("copy-code-btn")) return;
  const wrap = target.closest(".code-block-wrap");
  const code = wrap?.querySelector("pre code");
  const text = code?.textContent ?? "";
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    target.textContent = "Copied";
    setTimeout(() => {
      target.textContent = "Copy";
    }, 1200);
  } catch {
    target.textContent = "Failed";
    setTimeout(() => {
      target.textContent = "Copy";
    }, 1200);
  }
}

watch(
  () => props.activeConversation?.turns,
  () => {
    void revealLatestAssistantTurn();
  },
  { deep: true },
);

function send(stream: boolean) {
  const msg = chatIn.value.trim();
  if (!msg) return;
  emit("send-chat", { message: msg, stream });
  chatIn.value = "";
}
</script>

<template>
  <section class="panel chat-layout">
    <h2>Chat</h2>
    <p class="hint">
      Ask follow-ups in the same session. Use <strong>New conversation</strong> to start a fresh one.
    </p>
    <div class="meta">
      <p v-if="activeConversation?.sessionId" class="muted">Session: {{ activeConversation.sessionId }}</p>
      <p v-if="activeConversation?.chatMeta" class="muted">{{ activeConversation.chatMeta }}</p>
    </div>

    <div ref="transcriptEl" class="chat-history" @click="onTranscriptClick">
      <article
        v-for="(turn, idx) in activeConversation?.turns ?? []"
        :key="idx"
        class="chat-turn"
        :class="`turn-${turn.role}`"
      >
        <header>{{ turn.role === "user" ? "You" : "Assistant" }}</header>
        <pre v-if="turn.role === 'user'" class="chat-turn-content">{{ turn.content }}</pre>
        <div
          v-else
          class="chat-turn-content md-content"
          v-html="renderAssistantMarkdown(turn.content)"
        />
      </article>
      <p v-if="!(activeConversation?.turns?.length)" class="muted">
        Select conversation from left or start a new one.
      </p>
    </div>

    <div class="composer">
      <label class="chk">
        <input
          :checked="chatToolsStream"
          type="checkbox"
          @change="emit('toggle-tools-stream', ($event.target as HTMLInputElement).checked)"
        />
        Tool-aware stream (<code>tools_stream</code>)
      </label>
      <textarea
        v-model="chatIn"
        rows="3"
        class="ta"
        placeholder="Type your message..."
      />
      <p class="actions">
        <button type="button" class="primary" :disabled="chatBusy" @click="send(false)">
          {{ chatBusy ? "Sending..." : "Send" }}
        </button>
        <button type="button" :disabled="chatBusy" @click="send(true)">
          {{ chatBusy ? "Sending..." : "Send (SSE)" }}
        </button>
        <button type="button" :disabled="resetBusy" @click="emit('new-conversation')">
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
  min-height: 0;
  max-height: calc(100vh - 25rem);
  overflow: auto;
  display: grid;
  align-content: start;
  gap: 0.5rem;
  border: 1px solid #2a2e38;
  border-radius: 8px;
  padding: 0.6rem 0.6rem 5.5rem;
  background: #141820;
}
.chat-turn {
  align-self: start;
  border: 1px solid #2a2e38;
  border-radius: 8px;
  padding: 0.55rem 0.65rem;
  background: #171b22;
  width: fit-content;
  max-width: 100%;
}
.turn-user { border-color: #3d5a8c; }
.turn-user { justify-self: end; }
.chat-turn header { color: #9aa8c0; font-size: 0.8rem; margin-bottom: 0.2rem; }
.chat-turn-content {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  color: #d2d9e8;
  font-size: 0.85rem;
  line-height: 1.4;
}
.md-content {
  white-space: normal;
}
.md-content :deep(p) {
  margin: 0.35rem 0;
}
.md-content :deep(pre) {
  background: #10141b;
  border: 1px solid #2a3142;
  border-radius: 6px;
  padding: 0.5rem;
  overflow-x: auto;
  margin: 0;
}
.md-content :deep(.code-block-wrap) { margin: 0.45rem 0; }
.md-content :deep(.copy-code-btn) {
  display: inline-block;
  margin: 0 0 0.25rem;
  background: #2a3142;
  border: 1px solid #3d4658;
  color: #c8cfdd;
  border-radius: 6px;
  padding: 0.15rem 0.5rem;
  font-size: 0.75rem;
  cursor: pointer;
}
.md-content :deep(code) {
  background: #2a3142;
  border-radius: 4px;
  padding: 0.1rem 0.3rem;
}
.md-content :deep(ul),
.md-content :deep(ol) {
  margin: 0.35rem 0 0.35rem 1.1rem;
}
.composer {
  position: sticky;
  bottom: 0;
  margin-top: auto;
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
