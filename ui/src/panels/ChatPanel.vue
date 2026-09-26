<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import DOMPurify from "dompurify";
import { marked } from "marked";
import PenguinAvatar from "../components/PenguinAvatar.vue";

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
  chatUseMemory: boolean;
  chatMessagesView: string;
  chatMessagesBusy: boolean;
}>();

const emit = defineEmits<{
  (e: "send-chat", payload: { message: string; stream: boolean }): void;
  (e: "new-conversation"): void;
  (e: "toggle-tools-stream", value: boolean): void;
  (e: "toggle-use-memory", value: boolean): void;
  (e: "inspect-chat-messages"): void;
}>();

const chatIn = ref("");

/** Tool names called in the last inspected payload ("View sent messages"), shown as chips. */
const toolCalls = computed((): string[] => {
  if (!props.chatMessagesView) return [];
  try {
    const payload = JSON.parse(props.chatMessagesView) as {
      messages?: Array<{ role?: string; tool_calls?: unknown[] | null }>;
    };
    const names: string[] = [];
    for (const m of payload.messages ?? []) {
      for (const call of m.tool_calls ?? []) {
        const c = call as { name?: string; function?: { name?: string } };
        const name = c.function?.name ?? c.name;
        if (name) names.push(name);
      }
    }
    return names;
  } catch {
    return [];
  }
});
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
    btn.setAttribute("aria-label", "Copy code");
    btn.setAttribute("title", "Copy code");
    pre.parentElement.replaceChild(wrap, pre);
    wrap.appendChild(btn);
    wrap.appendChild(pre);
  });
  return container.innerHTML;
}

async function onTranscriptClick(ev: MouseEvent) {
  const target = ev.target as HTMLElement | null;
  const btn = target?.closest(".copy-code-btn") as HTMLElement | null;
  if (!btn) return;
  const wrap = btn.closest(".code-block-wrap");
  const code = wrap?.querySelector("pre code");
  const text = code?.textContent ?? "";
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    btn.classList.add("copied");
    setTimeout(() => {
      btn.classList.remove("copied");
    }, 1200);
  } catch {
    btn.classList.remove("copied");
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
  <section class="page chat-layout">
    <header class="page-head">
      <div>
        <p class="eyebrow">Chat // one penguin, your tools</p>
        <h1>Chat</h1>
        <p class="lead">
          Ask anything; follow-ups stay in the same conversation. Start a <strong>New conversation</strong> for a fresh topic.
        </p>
      </div>
      <div class="page-head-actions">
        <button type="button" :disabled="resetBusy" @click="emit('new-conversation')">
          {{ resetBusy ? "Resetting…" : "New conversation" }}
        </button>
      </div>
    </header>

    <div v-if="activeConversation?.sessionId || activeConversation?.chatMeta" class="meta">
      <span v-if="activeConversation?.sessionId" class="chip" title="Session id">{{ activeConversation.sessionId }}</span>
      <span v-if="activeConversation?.chatMeta" class="chip">{{ activeConversation.chatMeta }}</span>
    </div>

    <div ref="transcriptEl" class="chat-history" @click="onTranscriptClick">
      <article
        v-for="(turn, idx) in activeConversation?.turns ?? []"
        :key="idx"
        class="chat-turn"
        :class="`turn-${turn.role}`"
      >
        <header class="turn-head">
          <PenguinAvatar
            v-if="turn.role === 'assistant'"
            avatar="assistant"
            variant="inline"
            alt="Assistant"
          />
          <span class="turn-who">{{ turn.role === "user" ? "You" : "Kowalski" }}</span>
        </header>
        <pre v-if="turn.role === 'user'" class="chat-turn-content">{{ turn.content }}</pre>
        <div
          v-else
          class="chat-turn-content md-content"
          :class="{ 'is-error': turn.content.startsWith('[error]') }"
          v-html="renderAssistantMarkdown(turn.content)"
        />
      </article>
      <div v-if="!(activeConversation?.turns?.length)" class="empty-state chat-empty">
        <h3>Start a conversation</h3>
        <p>Type a question below — for example “Summarise https://example.com in five bullet points”.</p>
      </div>
      <p v-if="chatBusy" class="typing" aria-live="polite"><span class="dot-running" aria-hidden="true"></span> Kowalski is thinking…</p>
    </div>

    <div class="composer">
      <div v-if="toolCalls.length" class="tools-used" aria-label="Tools used in this conversation">
        <span class="lbl">Tools used</span>
        <span v-for="(t, i) in toolCalls" :key="`${t}-${i}`" class="chip tool-chip">⚙ {{ t }}</span>
      </div>
      <label class="field composer-field">
        <span class="sr-only">Message</span>
        <textarea
          v-model="chatIn"
          rows="3"
          class="ta"
          placeholder="Type your message…  (Ctrl/Cmd + Enter to send)"
          @keydown.enter.ctrl.prevent="send(false)"
          @keydown.enter.meta.prevent="send(false)"
        />
      </label>
      <div class="composer-row">
        <div class="toggles">
          <label class="chk">
            <input
              :checked="chatToolsStream"
              type="checkbox"
              @change="emit('toggle-tools-stream', ($event.target as HTMLInputElement).checked)"
            />
            Use tools <code>tools_stream</code>
          </label>
          <label class="chk">
            <input
              :checked="chatUseMemory"
              type="checkbox"
              @change="emit('toggle-use-memory', ($event.target as HTMLInputElement).checked)"
            />
            Use memory <code>use_memory</code>
          </label>
        </div>
        <div class="btn-row">
          <button type="button" class="ghost sm" :disabled="chatMessagesBusy" @click="emit('inspect-chat-messages')">
            {{ chatMessagesBusy ? "Loading…" : "View sent messages" }}
          </button>
          <button type="button" :disabled="chatBusy" @click="send(true)">
            {{ chatBusy ? "Sending…" : "Send (SSE)" }}
          </button>
          <button type="button" class="primary" :disabled="chatBusy" @click="send(false)">
            {{ chatBusy ? "Sending…" : "Send" }}
          </button>
        </div>
      </div>
      <details v-if="chatMessagesView" class="raw-messages">
        <summary>Last sent messages payload</summary>
        <pre class="json">{{ chatMessagesView }}</pre>
      </details>
      <p v-if="chatErr" class="note note-err">{{ chatErr }}</p>
    </div>
  </section>
</template>

<style scoped>
.chat-layout { display: flex; flex-direction: column; min-height: calc(100vh - 4.5rem); max-width: 60rem; }
.chat-layout .page-head { margin-bottom: 0.9rem; }
.meta { display: flex; gap: 0.4rem; flex-wrap: wrap; margin-bottom: 0.75rem; }
.meta .chip { font-size: 0.7rem; color: var(--muted); }
.chat-history {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 0.5rem 0 1.5rem;
}
.chat-turn { max-width: min(46rem, 92%); }
.turn-head { display: flex; align-items: center; gap: 0.45rem; margin-bottom: 0.3rem; }
.turn-who {
  font-family: var(--font-mono);
  font-size: 0.68rem;
  font-weight: 600;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted);
}
.chat-turn-content {
  margin: 0;
  font-size: 0.98rem;
  line-height: 1.6;
  color: var(--body);
  word-break: break-word;
}
.turn-user { align-self: flex-end; }
.turn-user .turn-head { justify-content: flex-end; }
.turn-user .chat-turn-content {
  white-space: pre-wrap;
  font-family: var(--font-body);
  background: var(--ink);
  color: var(--paper);
  padding: 0.7rem 0.95rem;
  border-radius: var(--radius-lg) var(--radius-lg) 2px var(--radius-lg);
}
.turn-assistant .chat-turn-content {
  background: var(--surface);
  border: 1px solid var(--line);
  border-left: 3px solid var(--red);
  padding: 0.6rem 1rem;
  border-radius: 2px var(--radius-lg) var(--radius-lg) var(--radius-lg);
}
.turn-assistant .chat-turn-content.is-error { border-left-color: var(--red); background: var(--red-soft); color: var(--ink); }
.md-content :deep(.code-block-wrap) { position: relative; margin: 0.5rem 0; }
.md-content :deep(.code-block-wrap pre) { margin: 0; padding-right: 2.4rem; }
.md-content :deep(.copy-code-btn) {
  position: absolute;
  top: 0.4rem;
  right: 0.4rem;
  width: 1.7rem;
  height: 1.7rem;
  padding: 0;
  border: 1px solid var(--line);
  background: var(--surface);
  color: var(--body);
  border-radius: var(--radius-sm);
  font-weight: 400;
}
.md-content :deep(.copy-code-btn)::before { content: "⧉"; font-size: 0.9rem; }
.md-content :deep(.copy-code-btn.copied) { border-color: var(--ok); color: var(--ok); }
.md-content :deep(.copy-code-btn.copied)::before { content: "✓"; }
.chat-empty { margin-top: 1rem; }
.typing { display: flex; align-items: center; gap: 0.5rem; color: var(--muted); font-size: 0.9rem; margin: 0; }
.composer {
  position: sticky;
  bottom: -2.5rem;
  background: var(--paper);
  border-top: 2px solid var(--ink);
  padding: 0.9rem 0 1.25rem;
}
.tools-used { display: flex; align-items: center; gap: 0.35rem; flex-wrap: wrap; margin-bottom: 0.6rem; }
.tools-used .lbl { margin-right: 0.25rem; }
.tool-chip { background: var(--steel-soft); color: var(--steel); border-color: transparent; }
.ta { min-height: 5rem; font-size: 1rem; }
.composer-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
  margin-top: 0.6rem;
}
.toggles { display: flex; gap: 1rem; flex-wrap: wrap; }
.toggles code { font-size: 0.72rem; color: var(--muted); }
.raw-messages { margin-top: 0.75rem; }
.raw-messages .json { max-height: 16rem; }
.note { margin-top: 0.75rem; }
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
}
</style>
