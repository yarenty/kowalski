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
  return sourceText.value.trim().length > 0;
});

// A horde with its own form sends only its answers: the operator-input block is built
// server-side from `formAnswers` (thin UI / thick core). A horde without one gets the single
// request box; ingest picks the links and file paths out of that text.
function buildPrompt(): string {
  return runForm.value ? "" : sourceText.value.trim();
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

    <template v-if="!followUpMode && !runForm">
      <label class="field">
        <span>What should the horde work on?</span>
        <textarea
          v-model="sourceText"
          rows="4"
          class="inp"
          :placeholder="horde?.prompt_tip || 'Links, file paths or text, one per line'"
          :disabled="disabled || busy"
        />
      </label>
      <p class="muted small">Links and file paths in the text are fetched and read; everything else is passed on as your request.</p>
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
