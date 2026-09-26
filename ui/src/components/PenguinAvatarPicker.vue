<script setup lang="ts">
import {
  PENGUIN_AVATAR_IDS,
  PENGUIN_DISPLAY,
  penguinAvatarLabel,
  penguinAvatarUrl,
} from "../penguins";

const model = defineModel<string>({ required: true });

defineProps<{
  readonly?: boolean;
}>();

const pickerSizePx = `${PENGUIN_DISPLAY.picker}px`;
</script>

<template>
  <div class="avatar-picker">
    <span class="label">Avatar</span>
    <div class="grid" role="listbox" aria-label="Choose penguin avatar">
      <button
        v-for="id in PENGUIN_AVATAR_IDS"
        :key="id"
        type="button"
        class="pick"
        :class="{ selected: model === id }"
        :disabled="readonly"
        :title="penguinAvatarLabel(id)"
        :aria-selected="model === id"
        role="option"
        @click="model = id"
      >
        <img
          :src="penguinAvatarUrl(id)"
          :alt="penguinAvatarLabel(id)"
          class="pick-img"
          :width="PENGUIN_DISPLAY.picker"
          :height="PENGUIN_DISPLAY.picker"
        />
        <span class="pick-label">{{ penguinAvatarLabel(id) }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.avatar-picker { display: flex; flex-direction: column; gap: 0.35rem; }
.label { font-size: 0.72rem; color: var(--muted); font-family: var(--font-mono); letter-spacing: 0.08em; text-transform: uppercase; font-weight: 600; }
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(4.5rem, 1fr));
  gap: 0.35rem;
  max-height: 10rem;
  overflow-y: auto;
  padding: 0.15rem;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--sunk);
}
.pick {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.15rem;
  padding: 0.3rem 0.2rem;
  border: 1px solid transparent;
  border-radius: 6px;
  background: var(--surface);
  cursor: pointer;
  color: var(--body);
  font-weight: 400;
}
.pick:hover:not(:disabled) { border-color: var(--muted); background: var(--surface); }
.pick.selected {
  border-color: var(--red);
  box-shadow: inset 0 0 0 1px var(--red);
  background: var(--red-soft);
  color: var(--ink);
}
.pick:disabled { opacity: 0.65; cursor: default; }
.pick img,
.pick-img {
  width: v-bind(pickerSizePx);
  height: v-bind(pickerSizePx);
  max-width: v-bind(pickerSizePx);
  max-height: v-bind(pickerSizePx);
  object-fit: contain;
  border-radius: 4px;
  background: var(--rail);
}
.pick-label {
  font-size: 0.58rem;
  line-height: 1.1;
  text-align: center;
  text-transform: capitalize;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
