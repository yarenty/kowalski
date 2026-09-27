<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { api, type HordeCatalogItem, type RunSummary } from "../api";
import HordeIcon from "../components/HordeIcon.vue";
import { categoryOf, type HordeCategory } from "../hordeIcons";
import { firstSentence, hordeById, hordes, hordesError, hordesLoaded, useHordePolling } from "../hordesStore";
import { relativeTime, runStatusLabel, upcomingTime } from "../runs";

const props = defineProps<{ waiting: RunSummary | null; waitingCount: number }>();
const emit = defineEmits<{
  (e: "open-horde", id: string): void;
  (e: "open-run", hordeId: string, runId: string): void;
  (e: "open-runs", filter: "all" | "needs_you"): void;
  (e: "open-build"): void;
  (e: "open-picker"): void;
}>();

/** At most this many tiles before "Show all": the page stays calm with 100 hordes. */
const TILE_CAP = 12;
const PINS_KEY = "kowalski.ui.hordes.pinned.v1";

type ChipId = "pinned" | HordeCategory | "all";

/** Operator pins; `null` = never set, so the shipped `featured` hordes stand in. */
const pins = ref<string[] | null>(readPins());
function readPins(): string[] | null {
  try {
    const raw = localStorage.getItem(PINS_KEY);
    if (raw == null) return null;
    const v = JSON.parse(raw) as unknown;
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : null;
  } catch {
    return null;
  }
}
function writePins(v: string[]) {
  pins.value = v;
  try {
    localStorage.setItem(PINS_KEY, JSON.stringify(v));
  } catch {
    /* storage unavailable: pins last for this page load */
  }
}
const pinnedIds = computed(() => new Set(pins.value ?? hordes.value.filter((h) => h.featured).map((h) => h.id)));
function isPinned(h: HordeCatalogItem) {
  return pinnedIds.value.has(h.id);
}
function togglePin(h: HordeCatalogItem) {
  const next = new Set(pinnedIds.value);
  if (next.has(h.id)) next.delete(h.id);
  else next.add(h.id);
  writePins([...next]);
}

const chip = ref<ChipId>("pinned");
const showAll = ref(false);
watch(chip, () => (showAll.value = false));

const chips = computed(() => {
  const count = (c: HordeCategory) => hordes.value.filter((h) => categoryOf(h.category).id === c).length;
  const list: Array<{ id: ChipId; label: string; n?: number }> = [{ id: "pinned", label: "Pinned" }];
  for (const c of ["spreadsheets", "web", "documents"] as const) {
    list.push({ id: c, label: categoryOf(c).label, n: count(c) });
  }
  if (count("code") > 0) list.push({ id: "code", label: categoryOf("code").label, n: count("code") });
  list.push({ id: "all", label: "All", n: hordes.value.length });
  return list;
});

const filtered = computed(() => {
  const all = [...hordes.value].sort((a, b) => a.display_name.localeCompare(b.display_name));
  if (chip.value === "all") return all;
  if (chip.value === "pinned") return all.filter(isPinned);
  return all.filter((h) => categoryOf(h.category).id === chip.value);
});
const visible = computed(() => (showAll.value ? filtered.value : filtered.value.slice(0, TILE_CAP)));
const compact = computed(() => chip.value === "all");

// ---------- the "needs you" strip (App polls `GET /api/runs?status=needs_you`) ----------
const firstWaiting = computed(() => props.waiting);
const waitingCount = computed(() => props.waitingCount);
const waitingName = computed(() => {
  const w = firstWaiting.value;
  return w ? hordeById(w.horde_id)?.display_name ?? w.horde_id : "";
});

// ---------- one quiet status line per tile ----------
const lastRun = ref<Record<string, RunSummary | null>>({});
const nextFire = ref<Record<string, string | null>>({});
async function loadTileStatus(list: HordeCatalogItem[]) {
  await Promise.all(
    list.map(async (h) => {
      const cron = (h.triggers ?? []).find((t) => t.kind === "cron" && t.effective_enabled);
      if (cron && !(h.id in nextFire.value)) {
        try {
          const t = await api.hordeTriggers(h.id);
          nextFire.value = { ...nextFire.value, [h.id]: t.triggers.find((x) => x.index === cron.index)?.next_fire ?? null };
        } catch {
          nextFire.value = { ...nextFire.value, [h.id]: null };
        }
      }
      if (!(h.id in lastRun.value)) {
        try {
          const r = await api.runs({ horde: h.id, limit: 1 });
          lastRun.value = { ...lastRun.value, [h.id]: r.runs[0] ?? null };
        } catch {
          lastRun.value = { ...lastRun.value, [h.id]: null };
        }
      }
    }),
  );
}
watch(visible, (list) => void loadTileStatus(list), { immediate: true });

function statusLine(h: HordeCatalogItem): { text: string; tone: string } {
  const triggers = h.triggers ?? [];
  const cron = triggers.find((t) => t.kind === "cron" && t.effective_enabled);
  if (cron && nextFire.value[h.id]) return { text: `Next: ${upcomingTime(nextFire.value[h.id])}`, tone: "" };
  const watchT = triggers.find((t) => t.kind === "watch" && t.effective_enabled);
  if (watchT) return { text: `Watching ${watchT.watch?.path ?? watchT.detail.replace(/^watch\s+/, "")}`, tone: "" };
  const r = lastRun.value[h.id];
  if (r) {
    const s = runStatusLabel(r.status);
    const when = relativeTime(r.started_at);
    if (s.tone === "red" || s.tone === "running") return { text: `${s.label} · ${when}`, tone: s.tone };
    return { text: `Last run ${when}`, tone: "" };
  }
  if (h.id in lastRun.value) return { text: "Not run yet", tone: "" };
  return { text: "", tone: "" };
}

let stopPolling: (() => void) | null = null;
onMounted(() => {
  stopPolling = useHordePolling();
});
onUnmounted(() => stopPolling?.());

const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
const shortcut = isMac ? "⌘K" : "Ctrl K";
</script>

<template>
  <section class="page home">
    <aside v-if="firstWaiting" class="needs-you" role="status" aria-live="polite">
      <span class="ny-dot" aria-hidden="true"></span>
      <p class="ny-text">
        <strong>{{ waitingName }}</strong> is waiting for your approval<template v-if="firstWaiting.title"> on “{{ firstWaiting.title }}”</template>.
        <span v-if="waitingCount > 1" class="muted"> {{ waitingCount - 1 }} more {{ waitingCount - 1 === 1 ? "run waits" : "runs wait" }}.</span>
      </p>
      <div class="ny-actions">
        <button type="button" class="primary sm" @click="emit('open-run', firstWaiting.horde_id, firstWaiting.run_id)">Review</button>
        <button type="button" class="link-btn" @click="emit('open-runs', 'needs_you')">All runs →</button>
      </div>
    </aside>

    <header class="home-head">
      <h1>What do you need done?</h1>
      <div class="filter-chips" role="group" aria-label="Show hordes">
        <button
          v-for="c in chips"
          :key="c.id"
          type="button"
          class="filter-chip"
          :aria-pressed="chip === c.id"
          @click="chip = c.id"
        >
          {{ c.label }}<span v-if="c.n != null" class="n">{{ c.n }}</span>
        </button>
      </div>
    </header>

    <p v-if="hordesError && !hordes.length" class="note note-err">Could not load hordes: {{ hordesError }}</p>
    <p v-else-if="!hordesLoaded" class="muted">Loading hordes…</p>
    <div v-else-if="!hordes.length" class="empty-state">
      <h3>No hordes yet</h3>
      <p>Build your first horde by describing the job in plain words.</p>
      <button type="button" class="primary" @click="emit('open-build')">Build a horde</button>
    </div>
    <div v-else-if="!visible.length" class="empty-state">
      <template v-if="chip === 'pinned'">
        <h3>Nothing pinned</h3>
        <p>Star a horde to keep it here.</p>
        <button type="button" @click="chip = 'all'">Show all hordes</button>
      </template>
      <template v-else>
        <h3>No hordes in this group</h3>
        <button type="button" @click="chip = 'all'">Show all hordes</button>
      </template>
    </div>

    <ul v-else class="tiles" :class="{ compact }" aria-label="Hordes">
      <li v-for="h in visible" :key="h.id" class="tile-cell">
        <button type="button" class="tile" @click="emit('open-horde', h.id)">
          <HordeIcon :category="h.category" :icon="h.icon" :size="compact ? 40 : 52" />
          <span class="tile-text">
            <span class="tile-name">{{ h.display_name }}</span>
            <span class="tile-desc">{{ firstSentence(h.description) }}</span>
            <span v-if="h.load_error" class="tile-status tone-red">Latest edit failed to load</span>
            <span v-else-if="statusLine(h).text" class="tile-status" :class="statusLine(h).tone && `tone-${statusLine(h).tone}`">
              {{ statusLine(h).text }}
            </span>
          </span>
        </button>
        <button
          type="button"
          class="pin"
          :class="{ on: isPinned(h) }"
          :aria-pressed="isPinned(h)"
          :aria-label="isPinned(h) ? `Unpin ${h.display_name}` : `Pin ${h.display_name}`"
          :title="isPinned(h) ? 'Unpin' : 'Pin to the home screen'"
          @click="togglePin(h)"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3.5l2.6 5.3 5.9.9-4.3 4.1 1 5.8L12 16.9l-5.2 2.7 1-5.8-4.3-4.1 5.9-.9z" /></svg>
        </button>
      </li>
    </ul>

    <div v-if="filtered.length > visible.length" class="more">
      <button type="button" @click="showAll = true">Show all {{ filtered.length }}</button>
      <span class="muted small">or press <kbd>{{ shortcut }}</kbd> to find one by name</span>
    </div>

    <p v-if="hordes.length" class="foot">
      Not here? <button type="button" class="link-btn" @click="emit('open-picker')">Find one of {{ hordes.length }} hordes ({{ shortcut }})</button>
      <span aria-hidden="true"> · </span>
      <button type="button" class="link-btn" @click="emit('open-build')">Build a new one</button>
    </p>
  </section>
</template>

<style scoped>
.home { max-width: 76rem; }

.needs-you {
  display: flex;
  align-items: center;
  gap: 0.8rem;
  flex-wrap: wrap;
  background: var(--red-soft);
  border: 1px solid var(--red);
  border-left: 6px solid var(--red);
  border-radius: var(--radius);
  padding: 0.7rem 1rem;
  margin: 0 0 1.75rem;
}
.ny-dot { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: var(--red); flex: 0 0 auto; }
.ny-text { margin: 0; color: var(--ink); flex: 1 1 20rem; }
.ny-text .muted { color: var(--body); }
.ny-actions { display: flex; align-items: center; gap: 1rem; }

.home-head { margin: 0.5rem 0 1.4rem; }
.home-head h1 { font-size: 2.1rem; margin: 0 0 0.9rem; }

.tiles {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1rem;
}
.tiles.compact { grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 0.75rem; }
.tile-cell { position: relative; min-width: 0; }
.tile {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: flex-start;
  gap: 1rem;
  text-align: left;
  white-space: normal;
  padding: 1.15rem 2.6rem 1.15rem 1.15rem;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius-lg);
  color: var(--body);
  font-weight: 400;
}
.tile:hover:not(:disabled) { background: var(--surface); border-color: var(--ink); }
.tile:active:not(:disabled) { transform: none; }
.tiles.compact .tile { padding: 0.85rem 2.3rem 0.85rem 0.85rem; gap: 0.75rem; }
.tile-text { display: grid; gap: 0.3rem; min-width: 0; }
.tile-name {
  font-family: var(--font-display);
  font-weight: 700;
  font-size: 1.2rem;
  line-height: 1.2;
  letter-spacing: -0.01em;
  color: var(--ink);
}
.tiles.compact .tile-name { font-size: 1rem; }
.tile-desc {
  font-size: 0.9rem;
  line-height: 1.4;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tile-status { font-size: 0.8rem; color: var(--muted); font-family: var(--font-mono); letter-spacing: 0.01em; margin-top: 0.15rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.tile-status.tone-red { color: var(--red-ink); font-weight: 600; }
.tile-status.tone-running { color: var(--ink); }
.tiles.compact .tile-desc { font-size: 0.84rem; }

.pin {
  position: absolute;
  top: 0.55rem;
  right: 0.55rem;
  width: 2rem;
  height: 2rem;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--muted);
}
.pin svg { width: 18px; height: 18px; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linejoin: round; }
.pin:hover:not(:disabled) { background: var(--sunk); color: var(--ink); }
.pin.on { color: var(--ink); }
.pin.on svg { fill: currentColor; }

.more { display: flex; align-items: center; gap: 0.9rem; flex-wrap: wrap; margin-top: 1rem; }
kbd {
  font-size: 0.72rem;
  padding: 0.05rem 0.35rem;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--surface);
}
.foot { margin-top: 2.25rem; padding-top: 1rem; border-top: 1px solid var(--hair); color: var(--muted); }
.foot .link-btn { font-size: inherit; }

@media (max-width: 1100px) {
  .tiles { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .tiles.compact { grid-template-columns: repeat(3, minmax(0, 1fr)); }
}
@media (max-width: 640px) {
  .tiles, .tiles.compact { grid-template-columns: minmax(0, 1fr); }
  .home-head h1 { font-size: 1.6rem; }
}
</style>
