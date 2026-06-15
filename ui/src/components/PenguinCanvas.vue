<script setup lang="ts">
import { computed } from "vue";
import type { RookeryPenguinSpec, RookerySessionStatus } from "../api";
import PenguinAvatar from "./PenguinAvatar.vue";
import { inferPenguinAvatarId } from "../penguins";

export type PenguinCard = {
  name: string;
  kind: string;
  displayName: string;
  description: string;
  output: string;
  toolIds: string[];
  chip: "draft" | "ready" | "missing-tool";
  avatar: string | null;
};

const props = defineProps<{
  pipeline: string[];
  penguins: RookeryPenguinSpec[] | null;
  sessionStatus: RookerySessionStatus;
  selectedName: string | null;
}>();

const emit = defineEmits<{
  (e: "select-penguin", name: string): void;
}>();

const cards = computed((): PenguinCard[] => {
  const byName = new Map((props.penguins ?? []).map((p) => [p.name, p]));
  return props.pipeline.map((name) => {
    const spec = byName.get(name);
    if (spec) {
      const toolIds = spec.tool_ids ?? [];
      const chip: PenguinCard["chip"] =
        props.sessionStatus === "proposed" || props.sessionStatus === "born"
          ? toolIds.length === 0 && spec.kind !== "ingest"
            ? "missing-tool"
            : "ready"
          : "draft";
      return {
        name: spec.name,
        kind: spec.kind,
        displayName: spec.display_name,
        description: spec.description,
        output: spec.output,
        toolIds,
        chip,
        avatar: spec.avatar ?? inferPenguinAvatarId(spec.kind, spec.name),
      };
    }
    return {
      name,
      kind: name,
      displayName: name,
      description: "Defined in conversation; propose to fill details.",
      output: "",
      toolIds: [],
      chip: "draft",
      avatar: inferPenguinAvatarId("step", name),
    };
  });
});

function chipLabel(chip: PenguinCard["chip"]): string {
  if (chip === "ready") return "ready";
  if (chip === "missing-tool") return "no tools";
  return "draft";
}
</script>

<template>
  <div class="penguin-canvas" :class="{ empty: !pipeline.length }">
    <p v-if="!pipeline.length" class="muted placeholder">
      Pipeline appears here after you propose a horde (or when the builder names steps in chat).
    </p>
    <div v-else class="track" role="list" aria-label="Pipeline penguins">
      <template v-for="(card, index) in cards" :key="card.name">
        <button
          type="button"
          class="penguin-card"
          :class="{ selected: selectedName === card.name, [`chip-${card.chip}`]: true }"
          role="listitem"
          @click="emit('select-penguin', card.name)"
        >
          <PenguinAvatar
            class="card-avatar"
            :avatar="card.avatar"
            :kind="card.kind"
            :name="card.name"
            variant="card"
            :alt="card.displayName"
          />
          <span class="card-head">
            <strong class="name">{{ card.displayName }}</strong>
            <span class="chip">{{ chipLabel(card.chip) }}</span>
          </span>
          <span class="kind mono">{{ card.kind }}</span>
          <span v-if="card.description" class="desc">{{ card.description }}</span>
          <span v-if="card.output" class="out mono">→ {{ card.output }}</span>
          <span v-if="card.toolIds.length" class="tools">
            <span v-for="t in card.toolIds" :key="t" class="tool-chip">{{ t }}</span>
          </span>
        </button>
        <span v-if="index < cards.length - 1" class="arrow" aria-hidden="true">→</span>
      </template>
    </div>
  </div>
</template>

<style scoped>
.penguin-canvas {
  border: 1px solid #2a2e38;
  border-radius: 8px;
  padding: 0.65rem;
  background: #10141b;
  min-height: 5.5rem;
}
.penguin-canvas.empty {
  display: flex;
  align-items: center;
  justify-content: center;
}
.placeholder {
  margin: 0;
  font-size: 0.85rem;
  text-align: center;
  padding: 0.5rem;
}
.track {
  display: flex;
  align-items: stretch;
  gap: 0.35rem;
  overflow-x: auto;
  padding-bottom: 0.25rem;
}
.penguin-card {
  flex: 0 0 auto;
  width: min(11rem, 42vw);
  text-align: left;
  background: #171b22;
  border: 1px solid #3d4658;
  border-radius: 8px;
  padding: 0.55rem 0.6rem;
  cursor: pointer;
  color: #d2d9e8;
  display: grid;
  gap: 0.25rem;
  transition: border-color 0.15s ease, transform 0.2s ease, box-shadow 0.2s ease;
  animation: slide-in 0.35s ease-out both;
}
.penguin-card:hover {
  border-color: #5a7ab8;
}
.penguin-card.selected {
  border-color: #6f9fd4;
  box-shadow: 0 0 0 1px #3d5a8c;
}
.penguin-card.chip-ready {
  border-left: 3px solid #3d7a58;
}
.penguin-card.chip-draft {
  border-left: 3px solid #5a606f;
  opacity: 0.92;
}
.penguin-card.chip-missing-tool {
  border-left: 3px solid #9a6b3d;
}
@keyframes slide-in {
  from {
    opacity: 0;
    transform: translateX(12px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}
.emoji {
  font-size: 1.1rem;
  line-height: 1;
}
.card-avatar {
  justify-self: start;
}
.card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.35rem;
}
.name {
  font-size: 0.88rem;
  font-weight: 600;
}
.chip {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  padding: 0.1rem 0.35rem;
  border-radius: 4px;
  background: #2a3142;
  color: #9aa8c0;
  flex-shrink: 0;
}
.chip-ready .chip {
  background: #2a4a3a;
  color: #b8e6c8;
}
.chip-missing-tool .chip {
  background: #4a3a2a;
  color: #e6d4b8;
}
.kind {
  font-size: 0.72rem;
  color: #8b92a5;
}
.desc {
  font-size: 0.78rem;
  color: #a8b4c8;
  line-height: 1.35;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.out {
  font-size: 0.7rem;
  color: #6a7285;
}
.tools {
  display: flex;
  flex-wrap: wrap;
  gap: 0.2rem;
}
.tool-chip {
  font-size: 0.65rem;
  padding: 0.05rem 0.3rem;
  border-radius: 4px;
  background: #252c3b;
  color: #c8cfdd;
}
.arrow {
  align-self: center;
  color: #5a7ab8;
  font-size: 1.1rem;
  flex-shrink: 0;
  padding: 0 0.1rem;
}
.mono {
  font-family: ui-monospace, monospace;
}
.muted {
  color: #6a7285;
}
@media (prefers-reduced-motion: reduce) {
  .penguin-card {
    animation: none;
  }
}
</style>
