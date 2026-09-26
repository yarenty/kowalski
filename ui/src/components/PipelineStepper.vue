<script setup lang="ts">
import { computed } from "vue";

/** Display state of one pipeline step. */
export type StepState = "pending" | "running" | "waiting" | "done" | "failed" | "cancelled" | "skipped";

export type StepperItem = {
  name: string;
  label: string;
  kind?: string;
  state: StepState;
};

const props = defineProps<{ steps: StepperItem[] }>();

const STATE_LABEL: Record<StepState, string> = {
  pending: "Pending",
  running: "Working",
  waiting: "Needs you",
  done: "Done",
  failed: "Failed",
  cancelled: "Cancelled",
  skipped: "Skipped",
};

const currentIndex = computed(() => {
  const i = props.steps.findIndex((s) => s.state === "running" || s.state === "waiting" || s.state === "failed");
  if (i >= 0) return i;
  const lastDone = props.steps.map((s) => s.state).lastIndexOf("done");
  return lastDone;
});

const headline = computed(() => {
  const n = props.steps.length;
  if (!n) return "";
  const i = currentIndex.value;
  if (i < 0) return `${n} steps`;
  return `Step ${i + 1} of ${n}`;
});

defineExpose({ headline });
</script>

<template>
  <ol class="stepper" aria-label="Pipeline progress">
    <li
      v-for="(s, i) in steps"
      :key="s.name"
      class="step"
      :class="[`st-${s.state}`, { current: i === currentIndex }]"
      :aria-current="i === currentIndex ? 'step' : undefined"
    >
      <span class="node" aria-hidden="true">
        <template v-if="s.state === 'done'">✓</template>
        <template v-else-if="s.state === 'failed'">✕</template>
        <template v-else-if="s.state === 'waiting'">!</template>
        <template v-else-if="s.state === 'cancelled' || s.state === 'skipped'">–</template>
        <template v-else>{{ i + 1 }}</template>
      </span>
      <span class="text">
        <span class="name">{{ s.label }}</span>
        <span class="state">{{ STATE_LABEL[s.state] }}</span>
      </span>
      <span class="sr-only">Step {{ i + 1 }}: {{ s.label }}, {{ STATE_LABEL[s.state] }}</span>
    </li>
  </ol>
</template>

<style scoped>
.stepper {
  list-style: none;
  margin: 0;
  padding: 0.25rem 0 0.35rem;
  display: flex;
  gap: 0;
  overflow-x: auto;
  /* a long pipeline scrolls here instead of widening the card around it */
  min-width: 0;
  max-width: 100%;
}
.step {
  position: relative;
  flex: 1 1 0;
  min-width: 6.5rem;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
  padding-right: 0.75rem;
}
/* connector line to the next step */
.step:not(:last-child)::after {
  content: "";
  position: absolute;
  top: 0.95rem;
  left: 2.3rem;
  right: 0.35rem;
  height: 2px;
  background: var(--line);
}
.step.st-done:not(:last-child)::after { background: var(--ink); }
.node {
  position: relative;
  z-index: 1;
  width: 1.9rem;
  height: 1.9rem;
  border-radius: 50%;
  display: inline-grid;
  place-items: center;
  font-family: var(--font-mono);
  font-size: 0.8rem;
  font-weight: 600;
  border: 2px solid var(--line);
  background: var(--surface);
  color: var(--muted);
}
.st-done .node { background: var(--ink); border-color: var(--ink); color: var(--paper); }
.st-running .node,
.st-waiting .node { background: var(--red); border-color: var(--red); color: var(--on-red); }
.st-failed .node { background: var(--surface); border-color: var(--red); color: var(--red-ink); }
.st-cancelled .node,
.st-skipped .node { border-style: dashed; }
.text { display: grid; gap: 0.1rem; min-width: 0; }
.name {
  font-family: var(--font-display);
  font-weight: 700;
  font-size: 0.9rem;
  color: var(--ink);
  line-height: 1.2;
  overflow-wrap: anywhere;
}
.st-pending .name,
.st-cancelled .name,
.st-skipped .name { color: var(--muted); }
.state {
  font-family: var(--font-mono);
  font-size: 0.66rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted);
}
.st-running .state,
.st-waiting .state,
.st-failed .state { color: var(--red-ink); font-weight: 600; }
.st-done .state { color: var(--ok); }
.st-running .state::before {
  content: "";
  display: inline-block;
  width: 0.45rem;
  height: 0.45rem;
  margin-right: 0.35rem;
  border-radius: 50%;
  background: var(--red);
  vertical-align: 0.05rem;
  animation: pulse 1.2s ease-in-out infinite;
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
}
</style>
