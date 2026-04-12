<script setup lang="ts">
import { ref } from "vue";

const tab = ref<"home" | "mcp" | "chat">("home");
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
        <button
          type="button"
          :class="{ active: tab === 'chat' }"
          @click="tab = 'chat'"
        >
          Chat
        </button>
      </nav>
    </header>

    <main class="main">
      <section v-if="tab === 'home'" class="panel">
        <h2>Welcome</h2>
        <p>
          This is the Vue shell for Kowalski. Use the CLI for day-to-day work:
          <code>kowalski-cli doctor</code>, <code>mcp ping</code>,
          <code>config check</code>.
        </p>
        <p>
          Wire this UI to an HTTP API when the backend is exposed (see
          <code>vite.config.ts</code> proxy for <code>/api</code>).
        </p>
      </section>

      <section v-else-if="tab === 'mcp'" class="panel">
        <h2>MCP status</h2>
        <p>
          Placeholder: list MCP servers from config and health (connect to your
          backend or embed config later).
        </p>
      </section>

      <section v-else class="panel">
        <h2>Chat</h2>
        <p>Placeholder: chat session when a Kowalski HTTP chat API is available.</p>
      </section>
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
.panel h2 {
  margin-top: 0;
  font-size: 1.1rem;
}
.panel p {
  line-height: 1.55;
  color: #b8c0d0;
}
code {
  background: #2a3142;
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
  font-size: 0.88em;
}
</style>
