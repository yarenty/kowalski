<script setup lang="ts">
import { ref } from "vue";

/** A key or password field with a show/hide button, so a pasted key can be checked before saving. */
defineProps<{ modelValue: string; placeholder?: string; label?: string }>();
const emit = defineEmits<{ (e: "update:modelValue", v: string): void }>();
const shown = ref(false);
</script>

<template>
  <span class="secret">
    <input
      :type="shown ? 'text' : 'password'"
      :value="modelValue"
      :placeholder="placeholder"
      :aria-label="label"
      autocomplete="off"
      spellcheck="false"
      @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    />
    <button
      type="button"
      class="eye"
      :aria-label="shown ? 'Hide key' : 'Show key'"
      :aria-pressed="shown"
      :title="shown ? 'Hide key' : 'Show key'"
      @click="shown = !shown"
    >
      <svg v-if="!shown" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z" />
        <circle cx="12" cy="12" r="3" />
      </svg>
      <svg v-else width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M3 3l18 18" />
        <path d="M10.6 5.1A9.9 9.9 0 0 1 12 5c6.4 0 10 7 10 7a17 17 0 0 1-3.2 4.1M6.6 6.6A17 17 0 0 0 2 12s3.6 7 10 7a9.6 9.6 0 0 0 5.4-1.6" />
        <path d="M9.9 9.9a3 3 0 0 0 4.2 4.2" />
      </svg>
    </button>
  </span>
</template>

<style scoped>
.secret {
  position: relative;
  display: flex;
  align-items: center;
}
.secret input {
  flex: 1;
  min-width: 0;
  padding-right: 2.6rem;
}
.eye {
  position: absolute;
  right: 0.3rem;
  display: grid;
  place-items: center;
  width: 2.1rem;
  height: 2.1rem;
  padding: 0;
  border: 0;
  border-radius: var(--radius, 6px);
  background: transparent;
  color: var(--muted);
  cursor: pointer;
}
.eye:hover {
  color: var(--ink);
  background: var(--sunk);
}
.eye:focus-visible {
  outline: 2px solid var(--red);
  outline-offset: 1px;
}
</style>
