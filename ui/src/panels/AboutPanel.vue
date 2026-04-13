<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, type Doctor, type Health } from "../api";
import logo from "../assets/logo01.png";

const health = ref<Health | null>(null);
const doctor = ref<Doctor | null>(null);
const err = ref<string | null>(null);

async function loadInfo() {
  err.value = null;
  try {
    const [h, d] = await Promise.all([api.health(), api.doctor()]);
    health.value = h;
    doctor.value = d;
  } catch (e) {
    err.value = e instanceof Error ? e.message : String(e);
  }
}

onMounted(() => {
  void loadInfo();
});
</script>

<template>
  <section class="panel">
    <h2>About Kowalski</h2>
    <img class="logo" :src="logo" alt="Kowalski logo" />
    <p>
      Kowalski is a Rust-native multi-agent framework focused on practical operator workflows:
      chat orchestration, MCP integrations, optional Postgres graph/federation features, and a
      lightweight Vue operator interface.
    </p>
    <p class="muted">
      Current build: <strong>{{ health?.version ?? "unknown" }}</strong>
      <span v-if="health?.model"> · model: {{ health.model }}</span>
    </p>
    <p class="muted" v-if="doctor">
      Provider: {{ doctor.llm.provider }} · configured model: {{ doctor.llm.model }}
    </p>
    <p class="muted" v-if="doctor">
      Ollama: {{ doctor.ollama.ok ? "reachable" : "not reachable" }} ({{ doctor.ollama.url }})
    </p>
    <p v-if="err" class="err">{{ err }}</p>
  </section>
</template>

<style scoped>
.panel {
  max-width: 52rem;
  margin: 0 auto;
  text-align: center;
}
.panel h2 { margin-top: 0; font-size: 1.1rem; }
.logo {
  width: min(320px, 100%);
  border-radius: 12px;
  border: 1px solid #2a2e38;
  margin: 0.35rem auto 0.7rem;
  display: block;
}
.muted { color: #8b92a5; }
.err { color: #e88; font-size: 0.9rem; }
</style>
