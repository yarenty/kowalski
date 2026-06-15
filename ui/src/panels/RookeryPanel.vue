<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import DOMPurify from "dompurify";
import { marked } from "marked";
import type { RookeryDraft, RookeryPenguinSpec, RookerySessionStatus } from "../api";
import PenguinCanvas from "../components/PenguinCanvas.vue";
import PenguinEditor from "../components/PenguinEditor.vue";
import PenguinAvatar from "../components/PenguinAvatar.vue";

export type RookeryTurn = { role: "user" | "assistant"; content: string };

export type RookeryUiSession = {
  id: string;
  serverSessionId: string;
  title: string;
  turns: RookeryTurn[];
  status: RookerySessionStatus;
  summary: string | null;
  pipeline: string[];
  draft: RookeryDraft | null;
  hordeRoot: string | null;
  outputRoot: string | null;
  birthNote: string | null;
  parseError: string | null;
  updatedAt: number;
};

const props = defineProps<{
  activeSession: RookeryUiSession | null;
  chatBusy: boolean;
  proposeBusy: boolean;
  birthBusy: boolean;
  saveHordeBusy: boolean;
  penguinSaveBusy: boolean;
  validateBusy: boolean;
  validateNote: string | null;
  newBusy: boolean;
  err: string | null;
  birthOverwrite: boolean;
}>();

const emit = defineEmits<{
  (e: "send-chat", message: string): void;
  (e: "propose"): void;
  (e: "validate-draft"): void;
  (e: "give-birth"): void;
  (e: "save-horde"): void;
  (e: "new-session"): void;
  (e: "open-horde"): void;
  (e: "toggle-birth-overwrite", value: boolean): void;
  (
    e: "save-penguin",
    payload: {
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
    },
  ): void;
}>();

const chatIn = ref("");
const transcriptEl = ref<HTMLElement | null>(null);
const selectedPenguinName = ref<string | null>(null);

watch(
  () => props.activeSession?.id,
  () => {
    selectedPenguinName.value = null;
  },
);

const selectedPenguin = computed((): RookeryPenguinSpec | null => {
  const session = props.activeSession;
  if (!session?.draft || !selectedPenguinName.value) return null;
  return session.draft.penguins.find((p) => p.name === selectedPenguinName.value) ?? null;
});

function onSelectPenguin(name: string) {
  selectedPenguinName.value = selectedPenguinName.value === name ? null : name;
}

function renderMarkdown(content: string): string {
  const html = marked.parse(content, { breaks: true, gfm: true }) as string;
  return DOMPurify.sanitize(html);
}

function statusLabel(status: RookerySessionStatus): string {
  if (status === "proposed") return "Ready to review";
  if (status === "born") return "Born";
  return "Interviewing";
}

async function scrollTranscript() {
  await nextTick();
  const el = transcriptEl.value;
  if (el) el.scrollTop = el.scrollHeight;
}

watch(
  () => props.activeSession?.turns,
  () => {
    void scrollTranscript();
  },
  { deep: true },
);

function send() {
  const msg = chatIn.value.trim();
  if (!msg) return;
  emit("send-chat", msg);
  chatIn.value = "";
}
</script>

<template>
  <section class="panel rookery-layout">
    <header class="rookery-head">
      <div>
        <h2>Rookery</h2>
        <p class="hint">Describe a workflow; propose a linear horde; give birth to markdown on disk.</p>
      </div>
      <div class="head-actions">
        <button type="button" class="secondary" :disabled="newBusy" @click="emit('new-session')">
          {{ newBusy ? "Starting…" : "New session" }}
        </button>
      </div>
    </header>

    <div v-if="!activeSession" class="empty muted">
      Select a session in the sidebar or start a new one.
    </div>

    <div v-else class="split">
      <div class="chat-col">
        <p class="status-row">
          <span class="badge" :class="`status-${activeSession.status}`">{{
            statusLabel(activeSession.status)
          }}</span>
          <span class="muted mono">{{ activeSession.serverSessionId }}</span>
        </p>

        <div ref="transcriptEl" class="chat-history">
          <article
            v-for="(turn, idx) in activeSession.turns"
            :key="idx"
            class="chat-turn"
            :class="`turn-${turn.role}`"
          >
            <header class="turn-head">
          <PenguinAvatar
            v-if="turn.role === 'assistant'"
            avatar="coordinator"
            variant="inline"
            alt="Rookery builder"
          />
              <span>{{ turn.role === "user" ? "You" : "Builder" }}</span>
            </header>
            <pre v-if="turn.role === 'user'" class="chat-turn-content">{{ turn.content }}</pre>
            <div
              v-else
              class="chat-turn-content md-content"
              v-html="renderMarkdown(turn.content)"
            />
          </article>
          <p v-if="!activeSession.turns.length" class="muted">
            Example: “Build a 3-step pipeline that ingests URLs, summarizes them, and writes a handoff file.”
          </p>
        </div>

        <div class="composer">
          <textarea
            v-model="chatIn"
            rows="3"
            class="ta"
            placeholder="Describe what you want to build…"
            :disabled="activeSession.status === 'born'"
            @keydown.enter.exact.prevent="send"
          />
          <p class="actions">
            <button
              type="button"
              class="primary"
              :disabled="chatBusy || activeSession.status === 'born'"
              @click="send"
            >
              {{ chatBusy ? "Sending…" : "Send" }}
            </button>
            <button
              type="button"
              :disabled="proposeBusy || chatBusy || activeSession.status === 'born'"
              @click="emit('propose')"
            >
              {{ proposeBusy ? "Proposing…" : "Propose horde" }}
            </button>
          </p>
        </div>
      </div>

      <div class="summary-col">
        <h3>Proposed pipeline</h3>
        <PenguinCanvas
          :pipeline="activeSession.pipeline"
          :penguins="activeSession.draft?.penguins ?? null"
          :session-status="activeSession.status"
          :selected-name="selectedPenguinName"
          @select-penguin="onSelectPenguin"
        />

        <PenguinEditor
          v-if="selectedPenguin && activeSession.draft"
          :penguin="selectedPenguin"
          :readonly="activeSession.status === 'interviewing'"
          :save-busy="penguinSaveBusy"
          @save="emit('save-penguin', { name: selectedPenguin.name, patch: $event })"
        />

        <p v-if="activeSession.summary" class="summary-md md-content" v-html="renderMarkdown(activeSession.summary)" />
        <p v-else-if="!activeSession.pipeline.length" class="muted">
          Run <strong>Propose horde</strong> after the interview to see the plan here.
        </p>

        <p v-if="activeSession.parseError" class="warn">{{ activeSession.parseError }}</p>
        <p v-if="validateNote" class="ok-note">{{ validateNote }}</p>

        <div
          v-if="activeSession.draft && (activeSession.status === 'proposed' || activeSession.status === 'born')"
          class="validate-row"
        >
          <button type="button" :disabled="validateBusy || proposeBusy" @click="emit('validate-draft')">
            {{ validateBusy ? "Validating…" : "Validate draft" }}
          </button>
        </div>

        <div v-if="activeSession.status === 'proposed'" class="birth-box">
          <label class="chk">
            <input
              type="checkbox"
              :checked="birthOverwrite"
              @change="emit('toggle-birth-overwrite', ($event.target as HTMLInputElement).checked)"
            />
            Overwrite existing horde directory
          </label>
          <button type="button" class="primary birth-btn" :disabled="birthBusy" @click="emit('give-birth')">
            {{ birthBusy ? "Giving birth…" : "Give birth" }}
          </button>
        </div>

        <div v-if="activeSession.status === 'born'" class="born-box">
          <p class="ok">Horde written to disk.</p>
          <p v-if="activeSession.hordeRoot" class="mono path">{{ activeSession.hordeRoot }}</p>
          <p v-if="activeSession.birthNote" class="muted">{{ activeSession.birthNote }}</p>
          <p class="actions">
            <button type="button" :disabled="saveHordeBusy" @click="emit('save-horde')">
              {{ saveHordeBusy ? "Saving…" : "Save horde to disk" }}
            </button>
            <button type="button" @click="emit('open-horde')">Open Horde tab</button>
          </p>
          <p class="muted small">
            New hordes appear in the catalog after server restart or when discovery paths include
            <code>examples/</code>.
          </p>
        </div>

        <p v-if="activeSession.outputRoot" class="muted small">
          Default output root: <code>{{ activeSession.outputRoot }}</code>
        </p>
      </div>
    </div>

    <p v-if="err" class="err">{{ err }}</p>
  </section>
</template>

<style scoped>
.panel h2 { margin: 0; font-size: 1.1rem; }
.hint { font-size: 0.9rem; color: #8b92a5; margin: 0.25rem 0 0; }
.rookery-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
  margin-bottom: 0.75rem;
}
.head-actions button.secondary {
  background: #2a3142;
  border: 1px solid #3d4658;
  color: #c8cfdd;
  padding: 0.35rem 0.65rem;
  border-radius: 6px;
  cursor: pointer;
}
.split {
  display: grid;
  grid-template-columns: minmax(0, 2fr) minmax(0, 3fr);
  gap: 1rem;
  min-height: calc(100vh - 10rem);
}
.chat-col, .summary-col {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-height: 0;
}
.summary-col {
  border: 1px solid #2a2e38;
  border-radius: 8px;
  padding: 0.75rem;
  background: #141820;
}
.summary-col h3 { margin: 0 0 0.35rem; font-size: 1rem; }
.status-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; margin: 0; }
.badge {
  font-size: 0.75rem;
  padding: 0.15rem 0.45rem;
  border-radius: 4px;
  border: 1px solid #3d4658;
}
.status-interviewing { background: #2a3142; color: #c8cfdd; }
.status-proposed { background: #2a4a3a; border-color: #3d7a58; color: #b8e6c8; }
.status-born { background: #3d3a2a; border-color: #7a6f3d; color: #e6dcb8; }
.mono { font-family: ui-monospace, monospace; font-size: 0.78rem; }
.chat-history {
  flex: 1;
  min-height: 12rem;
  max-height: calc(100vh - 22rem);
  overflow: auto;
  display: grid;
  align-content: start;
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
  max-width: 95%;
}
.turn-user { border-color: #6f8fc7; justify-self: end; margin-left: auto; }
.chat-turn header { color: #9aa8c0; font-size: 0.8rem; margin-bottom: 0.2rem; }
.turn-head,
.msg-head {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}
.chat-turn-content { margin: 0; white-space: pre-wrap; word-break: break-word; color: #d2d9e8; font-size: 0.85rem; }
.md-content :deep(p) { margin: 0.35rem 0; }
.md-content :deep(ul), .md-content :deep(ol) { margin: 0.35rem 0 0.35rem 1.1rem; }
.composer .ta {
  width: 100%;
  box-sizing: border-box;
  background: #171b22;
  border: 1px solid #3d4658;
  color: #e8e8ec;
  border-radius: 6px;
  padding: 0.5rem;
  resize: vertical;
}
.actions { display: flex; flex-wrap: wrap; gap: 0.5rem; margin: 0.35rem 0 0; }
.actions button {
  background: #2a3142;
  border: 1px solid #3d4658;
  color: #c8cfdd;
  padding: 0.35rem 0.65rem;
  border-radius: 6px;
  cursor: pointer;
}
.actions button.primary { background: #3d5a8c; border-color: #5a7ab8; color: #fff; }
.actions button:disabled { opacity: 0.5; cursor: not-allowed; }
.penguin-detail {
  border: 1px solid #3d5a8c;
  border-radius: 8px;
  padding: 0.6rem 0.65rem;
  background: #1a2230;
}
.penguin-detail h4 { margin: 0 0 0.25rem; font-size: 0.95rem; }
.detail-grid {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.2rem 0.65rem;
  margin: 0.5rem 0 0;
  font-size: 0.82rem;
}
.detail-grid dt { color: #8b92a5; margin: 0; }
.detail-grid dd { margin: 0; color: #d2d9e8; }
.editor-hint { margin: 0.5rem 0 0; }
.summary-md { margin: 0.5rem 0 0; max-height: 8rem; overflow: auto; }
.birth-box { margin-top: 0.75rem; padding-top: 0.75rem; border-top: 1px solid #2a2e38; }
.birth-btn { width: 100%; margin-top: 0.5rem; }
.chk { display: flex; align-items: center; gap: 0.35rem; font-size: 0.85rem; color: #8b92a5; }
.born-box { margin-top: 0.5rem; }
.ok { color: #8de3a8; margin: 0; }
.path { word-break: break-all; }
.warn { color: #f2b8c1; font-size: 0.85rem; }
.ok-note { color: #7dcea0; font-size: 0.85rem; margin: 0.35rem 0; }
.validate-row { margin: 0.35rem 0; }
.validate-row button {
  background: #2a3142;
  border: 1px solid #3d4658;
  color: #c8cfdd;
  padding: 0.3rem 0.6rem;
  border-radius: 6px;
  cursor: pointer;
}
.muted { color: #6a7285; font-size: 0.9rem; }
.small { font-size: 0.8rem; }
.err { color: #f2b8c1; margin-top: 0.5rem; }
.empty { padding: 2rem 0; text-align: center; }
@media (max-width: 900px) {
  .split { grid-template-columns: 1fr; }
}
</style>
