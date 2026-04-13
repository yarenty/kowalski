<script setup lang="ts">
import { ref } from "vue";
import ChatPanel from "./panels/ChatPanel.vue";
import FederationPanel from "./panels/FederationPanel.vue";
import GraphPanel from "./panels/GraphPanel.vue";
import HomePanel from "./panels/HomePanel.vue";
import McpPanel from "./panels/McpPanel.vue";

const tab = ref<"home" | "mcp" | "chat" | "federation" | "graph">("chat");
</script>

<template>
  <div class="app">
    <header class="header">
      <h1>Kowalski</h1>
      <p class="tagline">Rust-native agents · MCP · optional Postgres</p>
      <nav class="nav">
        <button
          type="button"
          :class="{ active: tab === 'home' }"
          @click="tab = 'home'"
        >
          Home
        </button>
        <button
          type="button"
          :class="{ active: tab === 'mcp' }"
          @click="tab = 'mcp'"
        >
          MCP
        </button>
        <button type="button" :class="{ active: tab === 'chat' }" @click="tab = 'chat'">
          Chat
        </button>
        <button
          type="button"
          :class="{ active: tab === 'federation' }"
          @click="tab = 'federation'"
        >
          Federation
        </button>
        <button type="button" :class="{ active: tab === 'graph' }" @click="tab = 'graph'">
          Graph
        </button>
      </nav>
    </header>

    <main class="main">
      <HomePanel v-if="tab === 'home'" />
      <McpPanel v-else-if="tab === 'mcp'" />
      <ChatPanel v-else-if="tab === 'chat'" />
      <FederationPanel v-else-if="tab === 'federation'" />
      <GraphPanel v-else-if="tab === 'graph'" />
    </main>
  </div>
</template>

<style>
:root {
  font-family: system-ui, sans-serif;
  color: #e8e8ec;
  background: #12141a;
}
body {
  margin: 0;
}
.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}
.header {
  padding: 1rem 1.5rem;
  border-bottom: 1px solid #2a2e38;
  background: #1a1d26;
}
.header h1 {
  margin: 0;
  font-size: 1.35rem;
  font-weight: 600;
}
.tagline {
  margin: 0.35rem 0 0.75rem;
  color: #8b92a5;
  font-size: 0.9rem;
}
.nav {
  display: flex;
  gap: 0.5rem;
}
.nav button {
  background: #2a3142;
  border: 1px solid #3d4658;
  color: #c8cfdd;
  padding: 0.4rem 0.75rem;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
}
.nav button.active {
  background: #3d5a8c;
  border-color: #5a7ab8;
  color: #fff;
}
.main {
  flex: 1;
  padding: 1.25rem 1.5rem;
  max-width: 52rem;
}
code {
  background: #2a3142;
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
  font-size: 0.88em;
}
</style>
