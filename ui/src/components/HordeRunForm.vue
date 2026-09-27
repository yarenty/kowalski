<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { HordeCatalogItem, HordeRunFormSpec } from "../api";
import HordeInputForm from "./HordeInputForm.vue";

const props = defineProps<{
  horde: HordeCatalogItem | null;
  disabled: boolean;
  busy: boolean;
  followUpMode: boolean;
  /** Primary button text for a new run (default "Run horde"). */
  submitLabel?: string;
}>();

const emit = defineEmits<{
  (
    e: "submit",
    payload: {
      prompt: string;
      source: string;
      question: string;
      formAnswers?: Record<string, string>;
    },
  ): void;
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

// The operator-input block is built server-side from `formAnswers` (thin UI / thick core).
// This prompt carries only the optional free-form URL / notes the operator adds alongside a form.
function buildPrompt(): string {
  const parts: string[] = [];
  const url = sourceUrl.value.trim();
  const text = sourceText.value.trim();
  if (url) parts.push(url);
  if (text) parts.push(text);
  if (!parts.length && !runForm.value && question.value.trim()) return question.value.trim();
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
    formAnswers: runForm.value ? { ...formAnswers.value } : undefined,
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
      <p v-if="!runForm" class="small">
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
        {{ busy ? "Working…" : followUpMode ? "Ask follow-up" : submitLabel || "Run horde" }}
      </button>
    </p>
  </div>
</template>

<style scoped>
.horde-run-form { display: flex; flex-direction: column; gap: 0.9rem; }
.horde-run-form > p { margin: 0; }
.field .muted { text-transform: none; letter-spacing: 0; font-family: var(--font-body); font-size: 0.8rem; font-weight: 400; }
.actions { margin: 0.1rem 0 0; }
.actions .primary { padding: 0.65rem 1.4rem; font-size: 1rem; }
</style>
