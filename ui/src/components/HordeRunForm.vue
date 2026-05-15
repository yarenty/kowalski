<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { HordeCatalogItem } from "../api";

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

watch(
  () => props.horde?.id,
  () => {
    sourceUrl.value = "";
    sourceText.value = "";
    question.value = props.horde?.default_question ?? "";
  },
  { immediate: true },
);

const canSubmit = computed(() => {
  if (props.followUpMode) return question.value.trim().length > 0;
  return sourceUrl.value.trim().length > 0 || sourceText.value.trim().length > 0;
});

function buildPrompt(): string {
  const parts: string[] = [];
  const url = sourceUrl.value.trim();
  const text = sourceText.value.trim();
  if (url) parts.push(url);
  if (text) parts.push(text);
  if (!parts.length && question.value.trim()) return question.value.trim();
  return parts.join("\n\n");
}

function submit() {
  const prompt = buildPrompt();
  if (!prompt.trim() && !props.followUpMode) return;
  const q = question.value.trim() || props.horde?.default_question || "What should we do with the output?";
  emit("submit", {
    prompt: props.followUpMode ? question.value.trim() : prompt,
    source: props.followUpMode ? question.value.trim() : prompt,
    question: q,
  });
}
</script>

<template>
  <div class="horde-run-form">
    <template v-if="!followUpMode">
      <p class="muted small">{{ horde?.prompt_tip || "Provide a source URL and/or text for the horde to process." }}</p>
      <label class="field">
        <span>Source URL</span>
        <input
          v-model="sourceUrl"
          type="url"
          class="inp"
          placeholder="https://…"
          :disabled="disabled || busy"
          @keydown.enter.prevent="submit"
        />
      </label>
      <label class="field">
        <span>Source text <span class="muted">(optional)</span></span>
        <textarea
          v-model="sourceText"
          rows="3"
          class="inp"
          placeholder="Paste notes or requirements…"
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
        <slot name="submit-label">{{ busy ? "Running…" : followUpMode ? "Ask follow-up" : "Run horde" }}</slot>
      </button>
    </p>
  </div>
</template>

<style scoped>
.horde-run-form { display: flex; flex-direction: column; gap: 0.5rem; }
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
