<script setup lang="ts">
import { ref, watch } from "vue";
import type { RookeryPenguinSpec } from "../api";
import PenguinForm from "./PenguinForm.vue";

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
      model_id: string | null;
    },
  ): void;
}>();

const editorTab = ref<"form" | "markdown">("form");
const kind = ref("");
const displayName = ref("");
const description = ref("");
const promptBody = ref("");
const agentBody = ref("");
const output = ref("");
const contextPathsText = ref("");
const toolIds = ref<string[]>([]);
const modelId = ref("");

function loadFromPenguin(p: RookeryPenguinSpec) {
  kind.value = p.kind;
  displayName.value = p.display_name;
  description.value = p.description;
  promptBody.value = p.prompt_body;
  agentBody.value = p.agent_body ?? "";
  output.value = p.output;
  contextPathsText.value = (p.context_paths ?? []).join("\n");
  toolIds.value = [...(p.tool_ids ?? [])];
  modelId.value = p.model_id ?? "";
}

watch(
  () => props.penguin,
  (p) => {
    if (p) loadFromPenguin(p);
  },
  { immediate: true },
);

function parseContextPaths(): string[] {
  return contextPathsText.value
    .split(/[\n,]+/)
    .map((s) => s.trim())
    .filter(Boolean);
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
    model_id: modelId.value.trim() || null,
  });
}
</script>

<template>
  <div class="penguin-editor">
    <div class="editor-head">
      <h4>{{ penguin.display_name }}</h4>
      <span class="mono muted small">agents/{{ penguin.name }}.md · prompts/{{ penguin.name }}.md</span>
    </div>

    <div class="tabs">
      <button type="button" :class="{ active: editorTab === 'form' }" @click="editorTab = 'form'">Form</button>
      <button type="button" :class="{ active: editorTab === 'markdown' }" @click="editorTab = 'markdown'">Advanced</button>
    </div>

    <PenguinForm
      v-show="editorTab === 'form'"
      v-model:kind="kind"
      v-model:display-name="displayName"
      v-model:description="description"
      v-model:output="output"
      v-model:context-paths-text="contextPathsText"
      v-model:tool-ids="toolIds"
      v-model:model-id="modelId"
      :penguin-name="penguin.name"
      :readonly="readonly"
    />

    <div v-show="editorTab === 'markdown'" class="markdown-pane">
      <label class="field">
        <span>Prompt body <code>prompts/{{ penguin.name }}.md</code></span>
        <textarea v-model="promptBody" rows="10" :disabled="readonly" class="mono editor-ta" />
      </label>
      <label class="field">
        <span>Agent body (after frontmatter)</span>
        <textarea v-model="agentBody" rows="6" :disabled="readonly" class="mono editor-ta" />
      </label>
    </div>

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
.tabs { display: flex; gap: 0.35rem; }
.tabs button {
  background: #2a3142;
  border: 1px solid #3d4658;
  color: #c8cfdd;
  padding: 0.25rem 0.55rem;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.8rem;
}
.tabs button.active { background: #3d6cb5; border-color: #5a8fd4; color: #fff; }
.markdown-pane { display: flex; flex-direction: column; gap: 0.5rem; }
.field { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.82rem; }
.field span { color: #8b92a5; }
.field textarea {
  background: #12161f;
  border: 1px solid #2e3648;
  border-radius: 4px;
  color: #e2e8f4;
  padding: 0.35rem 0.45rem;
  font: inherit;
}
.editor-ta { font-size: 0.78rem; line-height: 1.35; resize: vertical; }
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
