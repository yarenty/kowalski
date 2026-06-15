<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { HordeRunFormSpec, OperatorInputField } from "../api";

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
        v-else
        :value="answers[field.id] ?? ''"
        :type="field.type === 'url' ? 'url' : 'text'"
        :placeholder="field.placeholder ?? ''"
        :disabled="disabled"
        class="inp"
        @input="setField(field.id, ($event.target as HTMLInputElement).value)"
      />
    </label>
  </div>
</template>

<style scoped>
.horde-input-form {
  border: 1px solid #4a6fa5;
  border-radius: 8px;
  padding: 0.75rem;
  background: linear-gradient(180deg, #1a2438 0%, #151a24 100%);
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}
.form-head h4 { margin: 0; font-size: 0.95rem; color: #d8e4ff; }
.field { display: flex; flex-direction: column; gap: 0.25rem; }
.field span { font-size: 0.82rem; color: #9aa3b8; }
.field.required span { color: #c8d4ef; }
.req { color: #f2a07c; margin-left: 0.15rem; }
.inp {
  width: 100%;
  box-sizing: border-box;
  background: #12161f;
  border: 1px solid #3d4658;
  color: #e8e8ec;
  border-radius: 6px;
  padding: 0.4rem 0.55rem;
  font: inherit;
}
.mono { font-family: ui-monospace, monospace; }
</style>
