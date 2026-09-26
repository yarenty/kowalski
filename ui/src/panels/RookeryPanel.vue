<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import DOMPurify from "dompurify";
import { marked } from "marked";
import type { HordeEdge, RookeryDraft, RookeryPenguinSpec, RookerySessionStatus } from "../api";
import { isDagHorde } from "../hordeGraph";
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

const draftEdges = computed((): HordeEdge[] => props.activeSession?.draft?.edges ?? []);

const isDagDraft = computed(
  () =>
    !!props.activeSession?.pipeline.length &&
    isDagHorde(props.activeSession.pipeline, draftEdges.value),
);

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
  <section class="page rookery-layout">
    <header class="page-head">
      <div>
        <p class="eyebrow">Build // the Rookery</p>
        <h1>Build a horde</h1>
        <p class="lead">
          Describe the job in plain words. The builder asks questions, proposes a team of penguins (in a line or
          with parallel branches), and writes it to disk when you give birth.
        </p>
      </div>
      <div class="page-head-actions">
        <button type="button" class="primary" :disabled="newBusy" @click="emit('new-session')">
          {{ newBusy ? "Starting…" : "New session" }}
        </button>
      </div>
    </header>

    <div v-if="!activeSession" class="empty-state">
      <h3>No build session open</h3>
      <p>Start a session and describe the work you want a horde to do, step by step.</p>
      <button type="button" class="primary" :disabled="newBusy" @click="emit('new-session')">Start building</button>
    </div>

    <div v-else class="split">
      <div class="chat-col">
        <p class="status-row">
          <span class="badge" :class="`status-${activeSession.status}`">{{
            statusLabel(activeSession.status)
          }}</span>
          <span class="chip">{{ activeSession.serverSessionId }}</span>
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
          <p v-if="!activeSession.turns.length" class="muted example">
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
        <p class="eyebrow">Proposed pipeline</p>
        <PenguinCanvas
          :pipeline="activeSession.pipeline"
          :edges="draftEdges"
          :penguins="activeSession.draft?.penguins ?? null"
          :session-status="activeSession.status"
          :selected-name="selectedPenguinName"
          @select-penguin="onSelectPenguin"
        />

        <div v-if="isDagDraft && draftEdges.length" class="edges-box">
          <h4>Scheduling edges</h4>
          <p class="muted small">
            Fork/join DAG — parallel branches run when all predecessors finish (read-only).
          </p>
          <ul class="edge-list">
            <li v-for="(edge, idx) in draftEdges" :key="idx" class="mono">
              {{ edge.from }} → {{ edge.to }}
            </li>
          </ul>
        </div>

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

        <p v-if="activeSession.parseError" class="note note-warn small">{{ activeSession.parseError }}</p>
        <p v-if="validateNote" class="note note-ok small">{{ validateNote }}</p>

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
          <p class="ok born-title">✓ Horde written to disk.</p>
          <p v-if="activeSession.hordeRoot" class="mono path">{{ activeSession.hordeRoot }}</p>
          <p v-if="activeSession.birthNote" class="muted">{{ activeSession.birthNote }}</p>
          <p class="actions">
            <button type="button" :disabled="saveHordeBusy" @click="emit('save-horde')">
              {{ saveHordeBusy ? "Saving…" : "Save horde to disk" }}
            </button>
            <button type="button" class="primary" @click="emit('open-horde')">Open in Hordes</button>
          </p>
          <p class="muted small">
            The Hordes screen picks up new hordes from its horde folders without a restart; if yours does not appear, check that it was saved inside one of them.
          </p>
        </div>

        <p v-if="activeSession.outputRoot" class="muted small">
          Default output root: <code>{{ activeSession.outputRoot }}</code>
        </p>
      </div>
    </div>

    <p v-if="err" class="note note-err">{{ err }}</p>
  </section>
</template>

<style scoped>
.split {
  display: grid;
  grid-template-columns: minmax(0, 2fr) minmax(0, 3fr);
  gap: 1.25rem;
  align-items: start;
}
.chat-col, .summary-col { display: flex; flex-direction: column; gap: 0.75rem; min-width: 0; }
.summary-col {
  background: var(--surface);
  border: 1px solid var(--line);
  border-top: 3px solid var(--ink);
  border-radius: var(--radius);
  padding: 1rem 1.1rem;
}
.summary-col > * { margin: 0; }
.edges-box {
  padding: 0.6rem 0.75rem;
  border: 1px solid var(--hair);
  border-radius: var(--radius);
  background: var(--sunk);
}
.edges-box h4 { margin: 0 0 0.2rem; font-size: 0.9rem; }
.edge-list { margin: 0.35rem 0 0; padding-left: 1.1rem; font-size: 0.8rem; color: var(--ink); }
.edge-list li { margin: 0.15rem 0; }
.status-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; margin: 0; }
.badge.status-interviewing::before { background: var(--steel); }
.badge.status-interviewing { border-color: var(--steel); }
.badge.status-proposed { border-color: var(--warn); }
.badge.status-proposed::before { background: var(--warn); }
.badge.status-born { border-color: var(--ok); }
.badge.status-born::before { background: var(--ok); }
.mono { font-family: var(--font-mono); font-size: 0.8rem; }
.chat-history {
  min-height: 14rem;
  max-height: calc(100vh - 24rem);
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 0.9rem;
  background: var(--surface);
}
.chat-turn { max-width: 92%; }
.turn-head { display: flex; align-items: center; gap: 0.4rem; margin-bottom: 0.25rem; }
.turn-head span {
  font-family: var(--font-mono);
  font-size: 0.66rem;
  font-weight: 600;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted);
}
.chat-turn-content { margin: 0; word-break: break-word; font-size: 0.94rem; line-height: 1.55; color: var(--body); }
.turn-user { align-self: flex-end; }
.turn-user .turn-head { justify-content: flex-end; }
.turn-user .chat-turn-content {
  white-space: pre-wrap;
  font-family: var(--font-body);
  background: var(--ink);
  color: var(--paper);
  padding: 0.6rem 0.85rem;
  border-radius: var(--radius-lg) var(--radius-lg) 2px var(--radius-lg);
}
.turn-assistant .chat-turn-content {
  background: var(--sunk);
  border-left: 3px solid var(--red);
  padding: 0.5rem 0.85rem;
  border-radius: 2px var(--radius) var(--radius) var(--radius);
}
.example { font-style: italic; }
.composer .ta { min-height: 4.5rem; }
.actions { display: flex; flex-wrap: wrap; gap: 0.5rem; margin: 0.5rem 0 0; }
.summary-md { max-height: 12rem; overflow: auto; }
.validate-row { margin: 0; }
.birth-box {
  display: grid;
  gap: 0.6rem;
  padding-top: 0.9rem;
  border-top: 1px solid var(--hair);
}
.birth-btn { width: 100%; padding: 0.7rem; font-size: 1rem; }
.born-box {
  padding: 0.8rem 0.9rem;
  border: 1px solid var(--ok);
  border-radius: var(--radius);
  background: var(--ok-soft);
  color: var(--ink);
}
.born-box > * { margin: 0 0 0.4rem; }
.born-box .muted { color: var(--body); }
.born-title { color: var(--ink); font-weight: 600; }
.path { word-break: break-all; }
@media (max-width: 1100px) {
  .split { grid-template-columns: 1fr; }
  .chat-history { max-height: 60vh; }
}
</style>
