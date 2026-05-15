<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, type McpServer } from "../api";

const kind = defineModel<string>("kind", { required: true });
const displayName = defineModel<string>("displayName", { required: true });
const description = defineModel<string>("description", { required: true });
const output = defineModel<string>("output", { required: true });
const contextPathsText = defineModel<string>("contextPathsText", { required: true });
const toolIds = defineModel<string[]>("toolIds", { required: true });
const modelId = defineModel<string>("modelId", { required: true });

defineProps<{
  penguinName: string;
  readonly: boolean;
}>();

const kindOptions = ["ingest", "process", "deliver", "ask", "lint", "step", "custom"] as const;
const toolIdInput = ref("");
const mcpServers = ref<McpServer[]>([]);
const modelOptions = ref<string[]>([]);
const defaultModel = ref("");

onMounted(() => {
  void api.mcpServers().then((s) => { mcpServers.value = s; }).catch(() => {});
  void api.models().then((r) => {
    modelOptions.value = r.models;
    defaultModel.value = r.default_model;
  }).catch(() => {});
});

const mcpServerNames = computed(() => mcpServers.value.map((s) => s.name).filter(Boolean));

const outputHint = computed(() => {
  const k = kind.value;
  if (k === "ingest") return "e.g. debug/raw/";
  if (k === "deliver") return "e.g. HANDOFF.md";
  return "e.g. debug/stage-name.md";
});

function onKindChange() {
  if (!output.value.trim()) {
    if (kind.value === "ingest") output.value = "debug/raw/";
    else if (kind.value === "deliver") output.value = "HANDOFF.md";
  }
}

function addToolId() {
  const id = toolIdInput.value.trim();
  if (!id || toolIds.value.includes(id)) return;
  toolIds.value = [...toolIds.value, id];
  toolIdInput.value = "";
}

function removeToolId(id: string) {
  toolIds.value = toolIds.value.filter((t) => t !== id);
}
</script>

<template>
  <div class="penguin-form">
    <label class="field">
      <span>Display name</span>
      <input v-model="displayName" type="text" :disabled="readonly" />
    </label>

    <label class="field">
      <span>Kind</span>
      <select v-model="kind" class="mono" :disabled="readonly" @change="onKindChange">
        <option v-for="k in kindOptions" :key="k" :value="k">{{ k }}</option>
      </select>
    </label>

    <label class="field">
      <span>Description</span>
      <textarea v-model="description" rows="2" :disabled="readonly" />
    </label>

    <label class="field">
      <span>Output path <span class="muted">({{ outputHint }})</span></span>
      <input v-model="output" type="text" :disabled="readonly" class="mono" />
    </label>

    <label class="field">
      <span>Context paths</span>
      <textarea
        v-model="contextPathsText"
        rows="2"
        :disabled="readonly"
        class="mono small"
        placeholder="@artifact@ or workdir-relative paths, one per line"
      />
    </label>

    <label class="field">
      <span>Model <span class="muted small">(optional override)</span></span>
      <select v-model="modelId" :disabled="readonly">
        <option value="">Default ({{ defaultModel || "server" }})</option>
        <option v-for="m in modelOptions" :key="m" :value="m">{{ m }}</option>
      </select>
    </label>

    <div class="field">
      <span>
        Tool IDs
        <span v-if="mcpServerNames.length" class="muted small">· MCP: {{ mcpServerNames.join(", ") }}</span>
      </span>
      <div v-if="toolIds.length" class="chips">
        <span v-for="tid in toolIds" :key="tid" class="chip">
          {{ tid }}
          <button v-if="!readonly" type="button" class="chip-x" aria-label="Remove" @click="removeToolId(tid)">×</button>
        </span>
      </div>
      <p v-if="!readonly" class="tool-add">
        <input v-model="toolIdInput" type="text" class="mono" placeholder="tool-id" @keydown.enter.prevent="addToolId" />
        <button type="button" @click="addToolId">Add</button>
      </p>
    </div>
  </div>
</template>

<style scoped>
.penguin-form { display: flex; flex-direction: column; gap: 0.5rem; }
.field { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.82rem; }
.field span { color: #8b92a5; }
.field input,
.field textarea,
.field select {
  background: #12161f;
  border: 1px solid #2e3648;
  border-radius: 4px;
  color: #e2e8f4;
  padding: 0.35rem 0.45rem;
  font: inherit;
}
.chips { display: flex; flex-wrap: wrap; gap: 0.35rem; margin-top: 0.25rem; }
.chip {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  background: #2a3142;
  border-radius: 4px;
  padding: 0.15rem 0.4rem;
  font-size: 0.75rem;
  font-family: ui-monospace, monospace;
}
.chip-x {
  border: none;
  background: transparent;
  color: #8b92a5;
  cursor: pointer;
  padding: 0;
}
.tool-add { display: flex; gap: 0.35rem; margin: 0.35rem 0 0; }
.tool-add input { flex: 1; }
</style>
