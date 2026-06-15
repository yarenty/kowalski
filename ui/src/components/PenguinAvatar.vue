<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  PENGUIN_DISPLAY,
  type PenguinDisplayVariant,
  penguinAvatarUrl,
  resolvePenguinAvatar,
} from "../penguins";

const props = withDefaults(
  defineProps<{
    avatar?: string | null;
    kind?: string;
    name?: string;
    /** Named preset scaled from 256px source (`inline` 32px, `card` 48px, `editor` 64px). */
    variant?: PenguinDisplayVariant;
    /** Explicit px override when `variant` is omitted. */
    size?: number;
    alt?: string;
  }>(),
  {
    avatar: null,
    kind: "",
    name: "",
    variant: undefined,
    size: PENGUIN_DISPLAY.inline,
    alt: "Penguin agent",
  },
);

const resolvedId = computed(() =>
  resolvePenguinAvatar(props.avatar, props.kind ?? "", props.name ?? ""),
);

const src = computed(() => penguinAvatarUrl(resolvedId.value));

const displayPx = computed(() =>
  props.variant ? PENGUIN_DISPLAY[props.variant] : props.size,
);

const imgSrc = ref(src.value);
watch(src, (url) => {
  imgSrc.value = url;
});

function onImgError() {
  imgSrc.value = penguinAvatarUrl(null);
}
</script>

<template>
  <img
    class="penguin-avatar"
    :src="imgSrc"
    :alt="alt"
    :style="{
      '--penguin-avatar-size': `${displayPx}px`,
      width: `${displayPx}px`,
      height: `${displayPx}px`,
    }"
    loading="lazy"
    decoding="async"
    @error="onImgError"
  />
</template>

<style scoped>
.penguin-avatar {
  display: block;
  max-width: var(--penguin-avatar-size);
  max-height: var(--penguin-avatar-size);
  object-fit: contain;
  border-radius: 6px;
  flex-shrink: 0;
  background: #12161f;
}
</style>
