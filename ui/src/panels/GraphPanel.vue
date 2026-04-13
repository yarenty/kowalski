<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api } from "../api";

const graphStatus = ref<Record<string, unknown> | null>(null);
const graphErr = ref<string | null>(null);

async function loadGraphStatus() {
  graphErr.value = null;
  try {
    graphStatus.value = await api.graphStatus();
  } catch (e) {
    graphStatus.value = null;
    graphErr.value = e instanceof Error ? e.message : String(e);
  }
}

onMounted(() => {
  void loadGraphStatus();
});
</script>

<template>
  <section class="panel">
    <h2>Graph</h2>
    <p class="hint">
      <code>GET /api/graph/status</code> probes Postgres for <code>vector</code> and
      <code>age</code> extensions when <code>memory.database_url</code> is set and the CLI is
      built with <code>--features postgres</code>.
    </p>
    <p><button type="button" class="primary" @click="loadGraphStatus">Load graph status</button></p>
    <details>
      <summary>Raw graph JSON</summary>
      <pre v-if="graphStatus" class="json json-scroll">{{ JSON.stringify(graphStatus, null, 2) }}</pre>
    </details>
    <p v-if="graphErr" class="err">{{ graphErr }}</p>
  </section>
</template>

<style scoped>
.panel h2 { margin-top: 0; font-size: 1.1rem; }
.hint { font-size: 0.9rem; color: #8b92a5; }
.json { background: #1a1d26; border: 1px solid #2a2e38; border-radius: 6px; padding: 0.75rem; overflow-x: auto; font-size: 0.82rem; line-height: 1.45; color: #c8cfdd; }
.json-scroll { max-height: 18rem; overflow: auto; }
.err { color: #e88; font-size: 0.9rem; }
details { margin: 0.45rem 0; }
details > summary { cursor: pointer; color: #9aa8c0; font-size: 0.86rem; }
button { background: #2a3142; border: 1px solid #3d4658; color: #c8cfdd; padding: 0.4rem 0.75rem; border-radius: 6px; cursor: pointer; }
button.primary { background: #3d5a8c; border-color: #5a7ab8; color: #fff; }
code { background: #2a3142; padding: 0.15rem 0.4rem; border-radius: 4px; font-size: 0.88em; }
</style>
