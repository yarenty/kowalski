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
  <section class="page">
    <header class="page-head">
      <div>
        <p class="eyebrow">Admin // graph memory</p>
        <h1>Graph</h1>
      </div>
      <div class="page-head-actions">
        <button type="button" class="primary" @click="loadGraphStatus">Load graph status</button>
      </div>
    </header>
    <p class="note note-info">
      <code>GET /api/graph/status</code> probes Postgres for <code>vector</code> and
      <code>age</code> extensions when <code>memory.database_url</code> is set and the CLI is
      built with <code>--features postgres</code>.
    </p>
    <details>
      <summary>Raw graph JSON</summary>
      <pre v-if="graphStatus" class="json json-scroll">{{ JSON.stringify(graphStatus, null, 2) }}</pre>
    </details>
    <p v-if="graphErr" class="note note-err">{{ graphErr }}</p>
  </section>
</template>

<style scoped>
</style>
