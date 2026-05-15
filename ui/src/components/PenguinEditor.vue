<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { RookeryPenguinSpec } from "../api";
import { api, type McpServer } from "../api";

const props = defineProps<{
  penguin: RookeryPenguinSpec;
  readonly: boolean;
  saveBusy: boolean;
}>();

const emit = defineEmits<{
  (
    e: "save",
    patch: {
      kind: string;
      display_name: string;
      description: string;
      prompt_body: string;
      agent_body: string | null;
      output: string;
      context_paths: string[];
      tool_ids: string[];
    },
  ): void;
}>();

const kind = ref("");
const displayName = ref("");
const description = ref("");
const promptBody = ref("");
const agentBody = ref("");
const output = ref("");
const contextPathsText = ref("");
const toolIds = ref<string[]>([]);
const toolIdInput = ref("");
const showAdvanced = ref(false);
const mcpServers = ref<McpServer[]>([]);

function loadFromPenguin(p: RookeryPenguinSpec) {
  kind.value = p.kind;
  displayName.value = p.display_name;
  description.value = p.description;
  promptBody.value = p.prompt_body;
  agentBody.value = p.agent_body ?? "";
  output.value = p.output;
  contextPathsText.value = (p.context_paths ?? []).join("\n");
  toolIds.value = [...(p.tool_ids ?? [])];
}

watch(
  () => props.penguin,
  (p) => {
    if (p) loadFromPenguin(p);
  },
  { immediate: true },
);

void api
  .mcpServers()
  .then((s) => {
    mcpServers.value = s;
  })
  .catch(() => {
    mcpServers.value = [];
  });

const mcpServerNames = computed(() =>
  mcpServers.value.map((s) => s.name).filter(Boolean),
);

function parseContextPaths(): string[] {
  return contextPathsText.value
    .split(/[\n,]+/)
    .map((s) => s.trim())
    .filter(Boolean);
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

function save() {
  emit("save", {
    kind: kind.value.trim(),
    display_name: displayName.value.trim(),
    description: description.value.trim(),
    prompt_body: promptBody.value,
    agent_body: agentBody.value.trim() ? agentBody.value : null,
    output: output.value.trim(),
    context_paths: parseContextPaths(),
    tool_ids: toolIds.value,
  });
}
</script>

<template>
  <div class="penguin-editor">
    <div class="editor-head">
      <h4>{{ penguin.display_name }}</h4>
      <span class="mono muted small">agents/{{ penguin.name }}.md · prompts/{{ penguin.name }}.md</span>
    </div>

    <label class="field">
      <span>Display name</span>
      <input v-model="displayName" type="text" :disabled="readonly" />
    </label>
    <label class="field">
      <span>Kind</span>
      <input v-model="kind" type="text" :disabled="readonly" class="mono" />
    </label>
    <label class="field">
      <span>Description</span>
      <textarea v-model="description" rows="2" :disabled="readonly" />
    </label>
    <label class="field">
      <span>Output path (workdir-relative)</span>
      <input v-model="output" type="text" :disabled="readonly" class="mono" />
    </label>
    <label class="field">
      <span>Context paths (one per line or comma-separated)</span>
      <textarea v-model="contextPathsText" rows="2" :disabled="readonly" class="mono small" />
    </label>

    <label class="field">
      <span>Prompt body <code>prompts/{{ penguin.name }}.md</code></span>
      <textarea v-model="promptBody" rows="8" :disabled="readonly" class="mono editor-ta" />
    </label>

    <div class="field">
      <span>Tool IDs <span v-if="mcpServerNames.length" class="muted small">(MCP: {{ mcpServerNames.join(", ") }})</span></span>
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

    <button type="button" class="linkish" @click="showAdvanced = !showAdvanced">
      {{ showAdvanced ? "Hide" : "Show" }} agent markdown body
    </button>
    <label v-if="showAdvanced" class="field">
      <span>Agent body (after frontmatter in <code>agents/{{ penguin.name }}.md</code>)</span>
      <textarea v-model="agentBody" rows="6" :disabled="readonly" class="mono editor-ta" />
    </label>

    <p v-if="!readonly" class="actions">
      <button type="button" class="primary" :disabled="saveBusy" @click="save">
        {{ saveBusy ? "Saving…" : "Save penguin" }}
      </button>
    </p>
  </div>
</template>

<style scoped>
.penguin-editor {
  border: 1px solid #3d5a8c;
  border-radius: 8px;
  padding: 0.65rem;
  background: #1a2230;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  max-height: min(70vh, 640px);
  overflow: auto;
}
.editor-head h4 { margin: 0; font-size: 0.95rem; }
.field { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.82rem; }
.field span { color: #8b92a5; }
.field input,
.field textarea {
  background: #12161f;
  border: 1px solid #2e3648;
  border-radius: 4px;
  color: #e2e8f4;
  padding: 0.35rem 0.45rem;
  font: inherit;
}
.editor-ta { font-size: 0.78rem; line-height: 1.35; resize: vertical; }
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
  line-height: 1;
}
.tool-add { display: flex; gap: 0.35rem; margin: 0.35rem 0 0; }
.tool-add input { flex: 1; }
.linkish {
  background: none;
  border: none;
  color: #7aa2e8;
  cursor: pointer;
  font-size: 0.8rem;
  padding: 0;
  text-align: left;
}
.actions { margin: 0.25rem 0 0; }
.actions .primary {
  background: #3d6cb5;
  border: none;
  color: #fff;
  padding: 0.35rem 0.75rem;
  border-radius: 6px;
  cursor: pointer;
}
.actions .primary:disabled { opacity: 0.6; cursor: default; }
</style>
