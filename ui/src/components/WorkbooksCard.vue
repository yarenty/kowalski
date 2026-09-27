<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, type TableskiFile, type TableskiFiles } from "../api";

/**
 * The workbooks a tableski-backed horde can query: what is on the connected tableski account,
 * with upload (button or drop) and remove. kowalski forwards to tableski with Setup's sign-in.
 */
const emit = defineEmits<{ (e: "setup"): void }>();

const data = ref<TableskiFiles | null>(null);
const loading = ref(true);
const busy = ref<string | null>(null);
const problem = ref<string | null>(null);
const notice = ref<string | null>(null);
const dragging = ref(false);
const input = ref<HTMLInputElement | null>(null);

const ACCEPT = ".xlsx,.xls,.xlsm,.ods,.csv,.tsv,.parquet,.json,.ndjson,.jsonl";

/** The server's message without transport noise ("409 Conflict: …", JSON wrapping). */
function clean(e: unknown): string {
  const raw = e instanceof Error ? e.message : String(e);
  return raw.replace(/^\d{3}\s[^:]*:\s*/, "").replace(/^"|"$/g, "").trim() || "Something went wrong.";
}

const notConnected = computed(() => !!problem.value && /not connected|connect it in Setup|reconnect/i.test(problem.value));
const files = computed(() => data.value?.files ?? []);

async function load() {
  loading.value = true;
  problem.value = null;
  try {
    data.value = await api.tableskiFiles();
  } catch (e) {
    problem.value = clean(e);
  } finally {
    loading.value = false;
  }
}

function fileName(f: TableskiFile) {
  return (f.name as string) || (f.file_name as string) || "file";
}
function tables(f: TableskiFile) {
  return Array.isArray(f.tables) ? f.tables : [];
}
function size(f: TableskiFile) {
  const b = Number(f.bytes ?? f.size ?? 0);
  if (!b) return "";
  return b > 1024 * 1024 ? `${(b / 1024 / 1024).toFixed(1)} MB` : `${Math.max(1, Math.round(b / 1024))} KB`;
}
function expires(f: TableskiFile) {
  if (!f.expires_at) return "";
  const hours = Math.round((new Date(f.expires_at).getTime() - Date.now()) / 3_600_000);
  return hours > 0 ? `kept ${hours} h more` : "expires soon";
}

async function upload(list: FileList | null | undefined) {
  const file = list?.[0];
  if (!file) return;
  busy.value = "upload";
  problem.value = null;
  notice.value = null;
  try {
    const rec = await api.tableskiUpload(file);
    const made = tables(rec);
    notice.value = made.length
      ? `${fileName(rec)} is ready: ${made.map((t) => t.name).join(", ")}.`
      : `${fileName(rec)} uploaded.`;
    await load();
  } catch (e) {
    problem.value = clean(e);
  } finally {
    busy.value = null;
    if (input.value) input.value.value = "";
  }
}

async function remove(f: TableskiFile) {
  if (!window.confirm(`Remove ${fileName(f)} from tableski? Its tables disappear for every horde.`)) return;
  busy.value = f.id;
  problem.value = null;
  notice.value = null;
  try {
    await api.tableskiRemove(f.id);
    await load();
  } catch (e) {
    problem.value = clean(e);
  } finally {
    busy.value = null;
  }
}

function onDrop(ev: DragEvent) {
  dragging.value = false;
  void upload(ev.dataTransfer?.files);
}

onMounted(load);
</script>

<template>
  <section class="card workbooks" aria-labelledby="wb-title">
    <div class="wb-head">
      <h2 id="wb-title">Your workbooks</h2>
      <span class="muted small">on tableski · every sheet becomes a table</span>
    </div>

    <p v-if="loading" class="muted">Looking at your tableski account…</p>

    <template v-else-if="notConnected">
      <p>{{ problem }}</p>
      <p><button type="button" class="primary" @click="emit('setup')">Open Setup</button></p>
    </template>

    <template v-else>
      <ul v-if="files.length" class="files">
        <li v-for="f in files" :key="f.id" class="file">
          <svg class="doc" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <rect x="3.5" y="4.5" width="17" height="15" rx="2" /><path d="M3.5 9.5h17M3.5 14.5h17M9.5 9.5v10" />
          </svg>
          <div class="file-text">
            <span class="file-name">{{ fileName(f) }}</span>
            <span class="muted small">
              <template v-if="tables(f).length">
                {{ tables(f).map((t) => `${t.name} (${t.rows.toLocaleString()} rows)`).join(" · ") }}
              </template>
              <template v-else>no tables found</template>
              <template v-if="size(f)"> · {{ size(f) }}</template>
              <template v-if="expires(f)"> · {{ expires(f) }}</template>
            </span>
          </div>
          <button type="button" class="ghost sm" :disabled="busy !== null" :aria-label="`Remove ${fileName(f)}`" @click="remove(f)">
            {{ busy === f.id ? "Removing…" : "Remove" }}
          </button>
        </li>
      </ul>
      <p v-else class="muted">No workbooks yet. Add the file your questions are about.</p>

      <label
        class="drop"
        :class="{ over: dragging, busy: busy === 'upload' }"
        @dragover.prevent="dragging = true"
        @dragleave="dragging = false"
        @drop.prevent="onDrop"
      >
        <input ref="input" type="file" :accept="ACCEPT" :disabled="busy !== null" class="sr-only" @change="upload(($event.target as HTMLInputElement).files)" />
        <strong>{{ busy === "upload" ? "Uploading…" : "Add a workbook" }}</strong>
        <span class="muted small">Drop an Excel, CSV or Parquet file here, or click to choose</span>
        <span class="muted small hint">Works best with one table per sheet, column names in the first row, no title rows or merged cells.</span>
      </label>

      <p v-if="notice" class="note note-ok" role="status">{{ notice }}</p>
      <p v-if="problem" class="note note-err" role="alert">{{ problem }}</p>
    </template>
  </section>
</template>

<style scoped>
.workbooks { display: flex; flex-direction: column; gap: 0.8rem; }
.wb-head { display: flex; align-items: baseline; gap: 0.8rem; flex-wrap: wrap; }
.wb-head h2 { margin: 0; font-size: 1.05rem; }
.files { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; border: 1px solid var(--line); border-radius: var(--radius, 6px); }
.file { display: flex; align-items: center; gap: 0.75rem; padding: 0.65rem 0.8rem; border-top: 1px solid var(--hair); }
.file:first-child { border-top: 0; }
.doc { color: var(--muted); flex-shrink: 0; }
.file-text { display: flex; flex-direction: column; gap: 0.1rem; min-width: 0; flex: 1; }
.file-name { font-weight: 600; color: var(--ink); overflow-wrap: anywhere; }
.drop {
  display: flex; flex-direction: column; align-items: center; gap: 0.2rem;
  padding: 1rem; border: 2px dashed var(--line); border-radius: var(--radius, 6px);
  background: var(--sunk); cursor: pointer; text-align: center;
}
.drop:hover, .drop.over { border-color: var(--red); background: var(--red-soft); }
.drop:focus-within { outline: 2px solid var(--red); outline-offset: 2px; }
.drop.busy { cursor: progress; opacity: 0.8; }
.sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
.small { font-size: 0.85rem; }
</style>
