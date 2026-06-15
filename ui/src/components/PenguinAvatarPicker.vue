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
.label { font-size: 0.82rem; color: #8b92a5; }
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(4.5rem, 1fr));
  gap: 0.35rem;
  max-height: 10rem;
  overflow-y: auto;
  padding: 0.15rem;
  border: 1px solid #2e3648;
  border-radius: 6px;
  background: #12161f;
}
.pick {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.15rem;
  padding: 0.3rem 0.2rem;
  border: 1px solid transparent;
  border-radius: 6px;
  background: #1a2230;
  cursor: pointer;
  color: #a8b4c8;
}
.pick:hover:not(:disabled) { border-color: #5a7ab8; }
.pick.selected {
  border-color: #6f9fd4;
  box-shadow: 0 0 0 1px #3d5a8c;
  background: #243048;
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
  background: #10141b;
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
