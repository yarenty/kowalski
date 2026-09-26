<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, type McpPingResult, type McpServer } from "../api";

const servers = ref<McpServer[]>([]);
const serversErr = ref<string | null>(null);
const pingBusy = ref(false);
const pingResults = ref<McpPingResult[] | null>(null);
const pingErr = ref<string | null>(null);

function statusLabel(ok: boolean): string {
  return ok ? "OK" : "ERROR";
}
function statusClass(ok: boolean): string {
  return ok ? "status-ok" : "status-error";
}

async function loadServers() {
  serversErr.value = null;
  try {
    servers.value = await api.mcpServers();
  } catch (e) {
    servers.value = [];
    serversErr.value = e instanceof Error ? e.message : String(e);
  }
}

async function runMcpPing() {
  pingBusy.value = true;
  pingErr.value = null;
  pingResults.value = null;
  try {
    pingResults.value = await api.mcpPing();
  } catch (e) {
    pingErr.value = e instanceof Error ? e.message : String(e);
  } finally {
    pingBusy.value = false;
  }
}

onMounted(() => {
  void loadServers();
});
</script>

<template>
  <section class="page">
    <header class="page-head">
      <div>
        <p class="eyebrow">Admin // tool servers</p>
        <h1>MCP servers</h1>
        <p class="lead">External tool servers your penguins can call (for example tableski for spreadsheets).</p>
      </div>
      <div class="page-head-actions">
        <button type="button" @click="loadServers">Reload server list</button>
        <button type="button" class="primary" :disabled="pingBusy" @click="runMcpPing">
          {{ pingBusy ? "Pinging…" : "Ping all (initialize + tools/list)" }}
        </button>
      </div>
    </header>
    <p v-if="serversErr" class="note note-err">{{ serversErr }}</p>
    <div v-if="servers.length" class="cards">
      <article v-for="s in servers" :key="`${s.name}-${s.url}`" class="card">
        <header>
          <strong>{{ s.name }}</strong>
          <span class="status-badge status-neutral">{{ s.transport.toUpperCase() }}</span>
        </header>
        <p class="muted">{{ s.url }}</p>
      </article>
    </div>
    <div v-else-if="!serversErr" class="empty-state"><p>No MCP servers configured yet. Connect tableski on the Setup screen, or add servers to your config file.</p></div>
    <details>
      <summary>Raw MCP servers JSON</summary>
      <pre v-if="servers.length" class="json json-scroll">{{ JSON.stringify(servers, null, 2) }}</pre>
    </details>

    <h2 class="section-title">Ping results</h2>
    <div v-if="pingResults?.length" class="cards">
      <article v-for="r in pingResults" :key="`${r.name}-${r.url}`" class="card">
        <header>
          <strong>{{ r.name }}</strong>
          <span class="status-badge" :class="statusClass(r.ok)">{{ statusLabel(r.ok) }}</span>
        </header>
        <p class="muted">{{ r.transport }} · {{ r.url }}</p>
        <p v-if="r.ok" class="muted">tools: {{ r.tool_count ?? 0 }}</p>
        <p v-else class="err">{{ r.error }}</p>
      </article>
    </div>
    <details>
      <summary>Raw MCP ping JSON</summary>
      <pre v-if="pingResults" class="json json-scroll">{{ JSON.stringify(pingResults, null, 2) }}</pre>
    </details>
    <p v-if="pingErr" class="note note-err">{{ pingErr }}</p>
  </section>
</template>

<style scoped>
.cards { grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr)); }
.card { border-top-color: var(--steel); }
.card p { margin: 0.15rem 0; overflow-wrap: anywhere; }
</style>
