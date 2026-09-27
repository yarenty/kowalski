<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { api, type RunFilter, type RunSummary, type RunsListResponse } from "../api";
import HordeIcon from "../components/HordeIcon.vue";
import { hordeById, hordes, useHordePolling } from "../hordesStore";
import { clockOf, dayLabel, relativeTime, runStatusLabel } from "../runs";

const props = defineProps<{ initialFilter?: RunFilter; initialHorde?: string | null }>();
const emit = defineEmits<{ (e: "open-run", hordeId: string, runId: string): void }>();

const PAGE = 25;
const filter = ref<RunFilter>(props.initialFilter ?? "all");
const hordeFilter = ref<string>(props.initialHorde ?? "");
const runs = ref<RunSummary[]>([]);
const counts = ref<RunsListResponse["counts"] | null>(null);
const loading = ref(false);
const loadingMore = ref(false);
const err = ref<string | null>(null);
const hasMore = ref(false);

watch(
  () => [props.initialFilter, props.initialHorde] as const,
  ([f, h]) => {
    if (f) filter.value = f;
    hordeFilter.value = h ?? "";
  },
);

async function load(reset = true) {
  if (reset) loading.value = true;
  else loadingMore.value = true;
  err.value = null;
  try {
    const r = await api.runs({
      status: filter.value,
      horde: hordeFilter.value || undefined,
      limit: PAGE,
      offset: reset ? 0 : runs.value.length,
    });
    runs.value = reset ? r.runs : [...runs.value, ...r.runs];
    counts.value = r.counts;
    hasMore.value = r.runs.length === PAGE;
  } catch (e) {
    err.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
    loadingMore.value = false;
  }
}
watch([filter, hordeFilter], () => void load(true));

const chips = computed(() => [
  { id: "all" as const, label: "All", n: counts.value?.all ?? 0, alert: false },
  { id: "needs_you" as const, label: "Needs you", n: counts.value?.needs_you ?? 0, alert: true },
  { id: "running" as const, label: "Running", n: counts.value?.running ?? 0, alert: false },
  { id: "failed" as const, label: "Failed", n: counts.value?.failed ?? 0, alert: false },
]);
const visibleChips = computed(() => chips.value.filter((c) => c.id === "all" || c.id === filter.value || c.n > 0 || c.id === "needs_you" || c.id === "failed"));

/** Runs grouped under day headers, newest first (the API already sorts). */
const groups = computed(() => {
  const out: Array<{ label: string; runs: RunSummary[] }> = [];
  for (const r of runs.value) {
    const label = dayLabel(r.started_at);
    const last = out[out.length - 1];
    if (last && last.label === label) last.runs.push(r);
    else out.push({ label, runs: [r] });
  }
  return out;
});

const hordeOptions = computed(() => [...hordes.value].sort((a, b) => a.display_name.localeCompare(b.display_name)));

function timeFor(r: RunSummary): string {
  const label = dayLabel(r.started_at);
  return label === "Today" ? relativeTime(r.started_at) : clockOf(r.started_at);
}

let stopPolling: (() => void) | null = null;
let timer: ReturnType<typeof setInterval> | null = null;
onMounted(() => {
  stopPolling = useHordePolling();
  void load(true);
  // Keep the first page fresh (statuses change while runs work); older pages stay as loaded.
  timer = setInterval(() => {
    if (runs.value.length <= PAGE && !loadingMore.value) void load(true);
  }, 15000);
});
onUnmounted(() => {
  stopPolling?.();
  if (timer) clearInterval(timer);
});
</script>

<template>
  <section class="page runs-page">
    <header class="runs-head">
      <h1>Runs</h1>
      <div class="filters">
        <div class="filter-chips" role="group" aria-label="Filter runs by status">
          <button
            v-for="c in visibleChips"
            :key="c.id"
            type="button"
            class="filter-chip"
            :class="{ alert: c.alert && c.n > 0 }"
            :aria-pressed="filter === c.id"
            @click="filter = c.id"
          >
            {{ c.label }} <span class="n">· {{ c.n }}</span>
          </button>
        </div>
        <label class="horde-select">
          <span class="sr-only">Horde</span>
          <select v-model="hordeFilter" aria-label="Filter runs by horde">
            <option value="">Any horde</option>
            <option v-for="h in hordeOptions" :key="h.id" :value="h.id">{{ h.display_name }}</option>
          </select>
        </label>
      </div>
    </header>

    <p v-if="err" class="note note-err">Could not load runs: {{ err }}</p>
    <p v-else-if="loading && !runs.length" class="muted">Loading runs…</p>
    <div v-else-if="!runs.length" class="empty-state">
      <h3>{{ filter === "all" ? "No runs yet" : "Nothing here" }}</h3>
      <p v-if="filter === 'all'">Runs show up here once you send in a horde, or a schedule fires one.</p>
      <p v-else>No runs match this filter.</p>
    </div>

    <div v-else class="run-card">
      <template v-for="g in groups" :key="g.label">
        <h2 class="day">{{ g.label }}</h2>
        <ul class="rows">
          <li v-for="r in g.runs" :key="r.run_id">
            <button type="button" class="row" @click="emit('open-run', r.horde_id, r.run_id)">
              <HordeIcon :category="hordeById(r.horde_id)?.category" :icon="hordeById(r.horde_id)?.icon" :size="32" />
              <span class="row-text">
                <span class="row-title">{{ r.title || "Run" }}</span>
                <span class="row-horde">{{ hordeById(r.horde_id)?.display_name ?? r.horde_id }}</span>
              </span>
              <span class="run-status" :class="`tone-${runStatusLabel(r.status).tone}`">{{ runStatusLabel(r.status).label }}</span>
              <time class="row-time" :datetime="r.started_at ?? undefined">{{ timeFor(r) }}</time>
            </button>
          </li>
        </ul>
      </template>
    </div>

    <div v-if="hasMore" class="more">
      <button type="button" :disabled="loadingMore" @click="load(false)">{{ loadingMore ? "Loading…" : "Load older" }}</button>
    </div>
  </section>
</template>

<style scoped>
.runs-page { max-width: 60rem; }
.runs-head { margin: 0.5rem 0 1.25rem; }
.runs-head h1 { font-size: 2.1rem; margin: 0 0 0.9rem; }
.filters { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; flex-wrap: wrap; }
.horde-select select { width: auto; min-width: 12rem; padding: 0.4rem 0.6rem; font-size: 0.88rem; }
.sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }

.run-card {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius-lg);
  overflow: hidden;
}
.day {
  margin: 0;
  padding: 0.5rem 1rem;
  background: var(--sunk);
  border-bottom: 1px solid var(--hair);
  font-family: var(--font-mono);
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted);
}
.rows { list-style: none; margin: 0; padding: 0; }
.rows li + li { border-top: 1px solid var(--hair); }
.rows:not(:last-child) { border-bottom: 1px solid var(--hair); }
.row {
  width: 100%;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto 6.5rem;
  align-items: center;
  gap: 0.9rem;
  padding: 0.7rem 1rem;
  border: 0;
  border-radius: 0;
  background: transparent;
  text-align: left;
  font-weight: 400;
  white-space: normal;
}
.row:hover:not(:disabled) { background: var(--paper); }
.row:active:not(:disabled) { transform: none; }
.row:focus-visible { outline-offset: -3px; }
.row-text { display: grid; gap: 0.1rem; min-width: 0; }
.row-title { font-weight: 600; color: var(--ink); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.row-horde { font-size: 0.84rem; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.row-time { text-align: right; font-family: var(--font-mono); font-size: 0.76rem; color: var(--muted); }
.more { margin-top: 1rem; display: flex; justify-content: center; }

@media (max-width: 640px) {
  .row { grid-template-columns: auto minmax(0, 1fr) auto; }
  .row-time { display: none; }
}
</style>
