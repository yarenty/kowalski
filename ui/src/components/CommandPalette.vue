<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { api, type HordeCatalogItem, type RunSummary } from "../api";
import HordeIcon from "./HordeIcon.vue";
import { categoryOf } from "../hordeIcons";
import { firstSentence, hordeById, hordes, refreshHordes } from "../hordesStore";
import { relativeTime, runStatusLabel } from "../runs";

const emit = defineEmits<{
  (e: "close"): void;
  (e: "open-horde", id: string): void;
  (e: "open-run", hordeId: string, runId: string): void;
}>();

const query = ref("");
const recentRuns = ref<RunSummary[]>([]);
const active = ref(0);
const input = ref<HTMLInputElement | null>(null);
const dialog = ref<HTMLElement | null>(null);
const list = ref<HTMLElement | null>(null);
const returnFocus = document.activeElement as HTMLElement | null;

type Item =
  | { kind: "horde"; key: string; horde: HordeCatalogItem }
  | { kind: "run"; key: string; run: RunSummary };

function matches(hay: Array<string | undefined | null>, words: string[]): boolean {
  const text = hay.filter(Boolean).join(" ").toLowerCase();
  return words.every((w) => text.includes(w));
}

const words = computed(() => query.value.trim().toLowerCase().split(/\s+/).filter(Boolean));
const hordeItems = computed((): Item[] => {
  const w = words.value;
  const all = [...hordes.value].sort((a, b) => a.display_name.localeCompare(b.display_name));
  const hits = w.length
    ? all.filter((h) => matches([h.display_name, h.id, h.description, h.category, categoryOf(h.category).label], w))
    : all;
  return hits.slice(0, w.length ? 8 : 6).map((h) => ({ kind: "horde", key: `h:${h.id}`, horde: h }));
});
const runItems = computed((): Item[] => {
  const w = words.value;
  const hits = w.length
    ? recentRuns.value.filter((r) => matches([r.title, hordeById(r.horde_id)?.display_name, r.horde_id], w))
    : recentRuns.value;
  return hits.slice(0, w.length ? 8 : 5).map((r) => ({ kind: "run", key: `r:${r.run_id}`, run: r }));
});
const items = computed(() => [...hordeItems.value, ...runItems.value]);
watch(items, () => {
  if (active.value >= items.value.length) active.value = Math.max(0, items.value.length - 1);
});
watch(query, () => (active.value = 0));

function optionId(i: number) {
  return `kp-opt-${i}`;
}

function choose(item: Item | undefined) {
  if (!item) return;
  if (item.kind === "horde") emit("open-horde", item.horde.id);
  else emit("open-run", item.run.horde_id, item.run.run_id);
  close(false);
}

function close(restore = true) {
  emit("close");
  if (restore) void nextTick(() => returnFocus?.focus?.());
}

async function move(delta: number) {
  const n = items.value.length;
  if (!n) return;
  active.value = (active.value + delta + n) % n;
  await nextTick();
  document.getElementById(optionId(active.value))?.scrollIntoView({ block: "nearest" });
}

function onKey(e: KeyboardEvent) {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    void move(1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    void move(-1);
  } else if (e.key === "Enter") {
    e.preventDefault();
    choose(items.value[active.value]);
  } else if (e.key === "Escape") {
    e.preventDefault();
    close();
  } else if (e.key === "Tab") {
    // Focus trap: the dialog holds the input and the (mouse-driven) list only.
    e.preventDefault();
    input.value?.focus();
  }
}

onMounted(async () => {
  document.body.style.overflow = "hidden";
  await nextTick();
  input.value?.focus();
  void refreshHordes();
  try {
    recentRuns.value = (await api.runs({ limit: 50 })).runs;
  } catch {
    recentRuns.value = [];
  }
});
onUnmounted(() => {
  document.body.style.overflow = "";
});
</script>

<template>
  <div class="scrim" @mousedown.self="close()">
    <div
      ref="dialog"
      class="palette"
      role="dialog"
      aria-modal="true"
      aria-label="Find a horde or a run"
      @keydown="onKey"
    >
      <div class="search-row">
        <svg class="ico" viewBox="0 0 24 24" aria-hidden="true"><path d="M10.5 4a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13zM20 20l-4.8-4.8" /></svg>
        <input
          ref="input"
          v-model="query"
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls="kp-list"
          aria-autocomplete="list"
          :aria-activedescendant="items.length ? optionId(active) : undefined"
          placeholder="Find a horde or a run…"
          spellcheck="false"
          autocomplete="off"
        />
        <kbd>Esc</kbd>
      </div>

      <div id="kp-list" ref="list" class="results" role="listbox" aria-label="Results">
        <template v-if="hordeItems.length">
          <p class="section" role="presentation">Hordes</p>
          <div
            v-for="(it, i) in hordeItems"
            :id="optionId(i)"
            :key="it.key"
            class="opt"
            role="option"
            :aria-selected="active === i"
            @mousemove="active = i"
            @click="choose(it)"
          >
            <template v-if="it.kind === 'horde'">
              <HordeIcon :category="it.horde.category" :icon="it.horde.icon" :size="32" />
              <span class="opt-text">
                <span class="opt-title">{{ it.horde.display_name }}</span>
                <span class="opt-sub">{{ firstSentence(it.horde.description) }}</span>
              </span>
            </template>
          </div>
        </template>
        <template v-if="runItems.length">
          <p class="section" role="presentation">Runs</p>
          <div
            v-for="(it, j) in runItems"
            :id="optionId(hordeItems.length + j)"
            :key="it.key"
            class="opt"
            role="option"
            :aria-selected="active === hordeItems.length + j"
            @mousemove="active = hordeItems.length + j"
            @click="choose(it)"
          >
            <template v-if="it.kind === 'run'">
              <HordeIcon :category="hordeById(it.run.horde_id)?.category" :icon="hordeById(it.run.horde_id)?.icon" :size="32" />
              <span class="opt-text">
                <span class="opt-title">{{ it.run.title || "Run" }}</span>
                <span class="opt-sub">{{ hordeById(it.run.horde_id)?.display_name ?? it.run.horde_id }} · {{ relativeTime(it.run.started_at) }}</span>
              </span>
              <span class="run-status" :class="`tone-${runStatusLabel(it.run.status).tone}`">{{ runStatusLabel(it.run.status).label }}</span>
            </template>
          </div>
        </template>
        <p v-if="!items.length" class="empty">Nothing matches “{{ query }}”.</p>
      </div>

      <footer class="hints" aria-hidden="true">
        <span><kbd>↑</kbd><kbd>↓</kbd> move</span>
        <span><kbd>↵</kbd> open</span>
        <span><kbd>Esc</kbd> close</span>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.scrim {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: var(--scrim);
  display: flex;
  justify-content: center;
  align-items: flex-start;
  padding: 12vh 1rem 1rem;
}
.palette {
  width: min(640px, 100%);
  max-height: 72vh;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border: 1px solid var(--line);
  border-top: 3px solid var(--red);
  border-radius: var(--radius-lg);
  box-shadow: var(--pop-shadow);
  overflow: hidden;
}
.search-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--hair);
}
.search-row input {
  border: 0;
  padding: 0.25rem 0;
  font-size: 1.1rem;
  background: transparent;
}
.search-row input:focus-visible { box-shadow: none; border: 0; }
.ico { width: 20px; height: 20px; flex: 0 0 auto; fill: none; stroke: var(--muted); stroke-width: 1.8; stroke-linecap: round; }
kbd {
  font-size: 0.68rem;
  padding: 0.08rem 0.35rem;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  color: var(--muted);
  background: var(--paper);
}
.results { overflow-y: auto; padding: 0.35rem 0.5rem 0.6rem; }
.section {
  margin: 0.7rem 0.5rem 0.3rem;
  font-family: var(--font-mono);
  font-size: 0.68rem;
  font-weight: 600;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--muted);
}
.opt {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 0.55rem;
  border-radius: var(--radius);
  cursor: pointer;
}
.opt[aria-selected="true"] { background: var(--sunk); box-shadow: inset 3px 0 0 var(--red); }
.opt-text { display: grid; min-width: 0; flex: 1; }
.opt-title { font-weight: 600; color: var(--ink); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.opt-sub { font-size: 0.84rem; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.empty { margin: 1rem 0.5rem; color: var(--muted); }
.hints {
  display: flex;
  gap: 1.1rem;
  padding: 0.55rem 1rem;
  border-top: 1px solid var(--hair);
  background: var(--paper);
  font-size: 0.78rem;
  color: var(--muted);
}
.hints kbd { margin-right: 0.2rem; }
</style>
