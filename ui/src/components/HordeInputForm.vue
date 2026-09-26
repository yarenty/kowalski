<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { HordeRunFormSpec } from "../api";

const props = defineProps<{
  form: HordeRunFormSpec;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:answers", value: Record<string, string>): void;
}>();

const answers = ref<Record<string, string>>({});

function initAnswers(form: HordeRunFormSpec) {
  const next: Record<string, string> = {};
  for (const field of form.inputs) {
    next[field.id] = field.default ?? "";
  }
  answers.value = next;
  emit("update:answers", { ...next });
}

watch(
  () => props.form,
  (f) => {
    if (f) initAnswers(f);
  },
  { immediate: true, deep: true },
);

function setField(id: string, value: string) {
  answers.value = { ...answers.value, [id]: value };
  emit("update:answers", { ...answers.value });
}

const missingRequired = computed(() =>
  props.form.inputs.filter(
    (f) => f.required && !(answers.value[f.id] ?? "").trim(),
  ),
);

defineExpose({ missingRequired, answers });
</script>

<template>
  <div class="horde-input-form">
    <header class="form-head">
      <h4>Operator input</h4>
      <p class="muted small">
        Step <span class="mono">{{ form.step }}</span>
        <span v-if="form.display_name"> — {{ form.display_name }}</span>
      </p>
    </header>

    <label
      v-for="field in form.inputs"
      :key="field.id"
      class="field"
      :class="{ required: field.required }"
    >
      <span>
        {{ field.label }}
        <span v-if="field.required" class="req">*</span>
      </span>

      <textarea
        v-if="field.type === 'textarea'"
        :value="answers[field.id] ?? ''"
        :placeholder="field.placeholder ?? ''"
        :disabled="disabled"
        rows="3"
        class="inp"
        @input="setField(field.id, ($event.target as HTMLTextAreaElement).value)"
      />

      <select
        v-else-if="field.type === 'choice' && field.options?.length"
        :value="answers[field.id] ?? ''"
        :disabled="disabled"
        class="inp"
        @change="setField(field.id, ($event.target as HTMLSelectElement).value)"
      >
        <option value="">— select —</option>
        <option v-for="opt in field.options" :key="opt" :value="opt">{{ opt }}</option>
      </select>

      <input
        v-else-if="field.type === 'path'"
        :value="answers[field.id] ?? ''"
        type="text"
        :placeholder="field.placeholder ?? '/path/to/project'"
        :disabled="disabled"
        class="inp mono-path"
        spellcheck="false"
        autocomplete="off"
        @input="setField(field.id, ($event.target as HTMLInputElement).value)"
      />

      <input
        v-else
        :value="answers[field.id] ?? ''"
        :type="field.type === 'url' ? 'url' : 'text'"
        :placeholder="field.placeholder ?? ''"
        :disabled="disabled"
        class="inp"
        @input="setField(field.id, ($event.target as HTMLInputElement).value)"
      />
      <p v-if="field.type === 'path'" class="muted small path-hint">
        Absolute path to an existing directory on the machine running the server (repo root).
      </p>
    </label>
  </div>
</template>

<style scoped>
.horde-input-form {
  border: 1px solid var(--line);
  border-left: 4px solid var(--ink);
  border-radius: var(--radius);
  padding: 0.9rem 1rem;
  background: var(--sunk);
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
}
.form-head h4 { margin: 0 0 0.1rem; font-size: 1rem; }
.form-head p { margin: 0; }
.field.required > span:first-child { color: var(--ink); }
.req { color: var(--red-ink); margin-left: 0.15rem; }
.mono-path { font-family: var(--font-mono); font-size: 0.88rem; }
.path-hint { margin: 0.15rem 0 0; }
</style>
