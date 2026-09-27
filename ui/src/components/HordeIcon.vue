<script setup lang="ts">
import { computed } from "vue";
import { categoryOf, iconPath } from "../hordeIcons";

const props = withDefaults(
  defineProps<{
    category?: string | null;
    icon?: string | null;
    /** Square edge in px. */
    size?: number;
  }>(),
  { category: null, icon: null, size: 52 },
);

const style = computed(() => ({
  width: `${props.size}px`,
  height: `${props.size}px`,
  borderRadius: `${Math.round(props.size * 0.26)}px`,
  background: `var(${categoryOf(props.category).colorVar})`,
}));
const glyph = computed(() => `${Math.round(props.size * 0.5)}px`);
const d = computed(() => iconPath(props.icon, props.category));
</script>

<template>
  <span class="horde-icon" :style="style" aria-hidden="true">
    <svg viewBox="0 0 24 24" :width="glyph" :height="glyph"><path :d="d" /></svg>
  </span>
</template>

<style scoped>
.horde-icon {
  display: inline-grid;
  place-items: center;
  flex: 0 0 auto;
  color: var(--on-cat);
}
svg {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}
</style>
