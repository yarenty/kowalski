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
  <section class="panel">
    <h2>MCP</h2>
    <p>
      <button type="button" class="primary" @click="loadServers">Reload server list</button>
      <button type="button" :disabled="pingBusy" @click="runMcpPing">
        {{ pingBusy ? "Pinging..." : "Ping all (initialize + tools/list)" }}
      </button>
    </p>
    <p v-if="serversErr" class="err">{{ serversErr }}</p>
    <div v-if="servers.length" class="cards">
      <article v-for="s in servers" :key="`${s.name}-${s.url}`" class="card">
        <header>
          <strong>{{ s.name }}</strong>
          <span class="status-badge status-neutral">{{ s.transport.toUpperCase() }}</span>
        </header>
        <p class="muted">{{ s.url }}</p>
      </article>
    </div>
    <p v-else-if="!serversErr" class="muted">No servers or empty config.</p>
    <details>
      <summary>Raw MCP servers JSON</summary>
      <pre v-if="servers.length" class="json json-scroll">{{ JSON.stringify(servers, null, 2) }}</pre>
    </details>

    <h3>Ping results</h3>
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
    <p v-if="pingErr" class="err">{{ pingErr }}</p>
  </section>
</template>

<style scoped>
.panel h2 { margin-top: 0; font-size: 1.1rem; }
.panel h3 { font-size: 1rem; margin-top: 1.25rem; }
.cards { display: grid; gap: 0.45rem; }
.card { border: 1px solid #2a2e38; border-radius: 8px; background: #171b22; padding: 0.55rem 0.65rem; }
.card header { display: flex; align-items: center; justify-content: space-between; }
.status-badge { border-radius: 999px; font-size: 0.72rem; padding: 0.12rem 0.45rem; border: 1px solid transparent; }
.status-ok { color: #8de3a8; border-color: #2f7c47; background: #153323; }
.status-error { color: #ffb0b0; border-color: #8d3a3a; background: #381b1b; }
.status-neutral { color: #b9c8ef; border-color: #41598e; background: #1c2844; }
.json { background: #1a1d26; border: 1px solid #2a2e38; border-radius: 6px; padding: 0.75rem; overflow-x: auto; font-size: 0.82rem; line-height: 1.45; color: #c8cfdd; }
.json-scroll { max-height: 18rem; overflow: auto; }
.muted { color: #6a7285; font-size: 0.9rem; }
.err { color: #e88; font-size: 0.9rem; }
details { margin: 0.45rem 0; }
details > summary { cursor: pointer; color: #9aa8c0; font-size: 0.86rem; }
button { background: #2a3142; border: 1px solid #3d4658; color: #c8cfdd; padding: 0.4rem 0.75rem; border-radius: 6px; cursor: pointer; margin-right: 0.5rem; }
button.primary { background: #3d5a8c; border-color: #5a7ab8; color: #fff; }
</style>
