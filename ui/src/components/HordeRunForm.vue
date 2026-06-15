<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { HordeCatalogItem, HordeRunFormSpec } from "../api";
import HordeInputForm from "./HordeInputForm.vue";

const props = defineProps<{
  horde: HordeCatalogItem | null;
  disabled: boolean;
  busy: boolean;
  followUpMode: boolean;
}>();

const emit = defineEmits<{
  (e: "submit", payload: { prompt: string; source: string; question: string }): void;
}>();

const sourceUrl = ref("");
const sourceText = ref("");
const question = ref("");
const formAnswers = ref<Record<string, string>>({});

const runForm = computed((): HordeRunFormSpec | null =>
  !props.followUpMode && props.horde?.run_form?.inputs?.length
    ? props.horde.run_form
    : null,
);

watch(
  () => props.horde?.id,
  () => {
    sourceUrl.value = "";
    sourceText.value = "";
    question.value = props.horde?.default_question ?? "";
    formAnswers.value = {};
  },
  { immediate: true },
);

const formComplete = computed(() => {
  if (!runForm.value) return true;
  return runForm.value.inputs.every((f) => {
    if (!f.required) return true;
    return (formAnswers.value[f.id] ?? "").trim().length > 0;
  });
});

const canSubmit = computed(() => {
  if (!formComplete.value) return false;
  if (props.followUpMode) return question.value.trim().length > 0;
  if (runForm.value) return true;
  return sourceUrl.value.trim().length > 0 || sourceText.value.trim().length > 0;
});

function buildOperatorBlock(): string {
  const form = runForm.value;
  if (!form) return "";
  const lines = [`# Operator input (${form.display_name ?? form.step})`];
  for (const field of form.inputs) {
    const v = (formAnswers.value[field.id] ?? "").trim();
    if (v) lines.push(`**${field.label}:** ${v}`);
  }
  return lines.join("\n\n");
}

function buildPrompt(): string {
  const parts: string[] = [];
  const op = buildOperatorBlock();
  if (op) parts.push(op);
  const url = sourceUrl.value.trim();
  const text = sourceText.value.trim();
  if (url) parts.push(url);
  if (text) parts.push(text);
  if (!parts.length && question.value.trim()) return question.value.trim();
  return parts.join("\n\n");
}

function submit() {
  const q =
    question.value.trim() ||
    props.horde?.default_question ||
    "What should we do with the output?";
  emit("submit", {
    prompt: props.followUpMode ? question.value.trim() : buildPrompt(),
    source: props.followUpMode ? question.value.trim() : buildPrompt(),
    question: q,
  });
}
</script>

<template>
  <div class="horde-run-form">
    <HordeInputForm
      v-if="runForm"
      :form="runForm"
      :disabled="disabled || busy"
      @update:answers="formAnswers = $event"
    />

    <template v-if="!followUpMode">
      <p v-if="!runForm" class="muted small">
        {{ horde?.prompt_tip || "Provide a source URL and/or text for the horde to process." }}
      </p>
      <p v-else class="muted small">Optional: add a reference URL or extra notes below the form.</p>

      <label class="field">
        <span>Source URL <span v-if="runForm" class="muted">(optional)</span></span>
        <input
          v-model="sourceUrl"
          type="url"
          class="inp"
          placeholder="https://…"
          :disabled="disabled || busy"
        />
      </label>
      <label class="field">
        <span>Extra notes <span class="muted">(optional)</span></span>
        <textarea
          v-model="sourceText"
          rows="2"
          class="inp"
          placeholder="Paste requirements…"
          :disabled="disabled || busy"
        />
      </label>
      <label class="field">
        <span>Question for pipeline</span>
        <input
          v-model="question"
          type="text"
          class="inp"
          :placeholder="horde?.default_question || 'What should we extract?'"
          :disabled="disabled || busy"
        />
      </label>
    </template>

    <template v-else>
      <p class="muted small">Ask about the completed run (refines against artifacts).</p>
      <label class="field">
        <span>Follow-up question</span>
        <input
          v-model="question"
          type="text"
          class="inp"
          :disabled="disabled || busy"
          @keydown.enter.prevent="submit"
        />
      </label>
    </template>

    <p class="actions">
      <button type="button" class="primary" :disabled="disabled || busy || !canSubmit" @click="submit">
        {{ busy ? "Running…" : followUpMode ? "Ask follow-up" : "Run horde" }}
      </button>
    </p>
  </div>
</template>

<style scoped>
.horde-run-form { display: flex; flex-direction: column; gap: 0.65rem; }
.field { display: flex; flex-direction: column; gap: 0.25rem; }
.field span { font-size: 0.8rem; color: #8b92a5; }
.inp {
  width: 100%;
  box-sizing: border-box;
  background: #1a1d26;
  border: 1px solid #3d4658;
  color: #e8e8ec;
  border-radius: 6px;
  padding: 0.4rem 0.55rem;
  font: inherit;
}
.actions { margin: 0.25rem 0 0; }
.primary {
  background: #3d6cb5;
  border: none;
  color: #fff;
  padding: 0.4rem 0.85rem;
  border-radius: 6px;
  cursor: pointer;
}
.primary:disabled { opacity: 0.55; cursor: default; }
</style>
