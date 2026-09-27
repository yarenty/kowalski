<script setup lang="ts">
import { relativeTime } from "../runs";

/** Side list of saved threads (Chat conversations, Build sessions) beside the work area. */
defineProps<{
  label: string;
  newLabel: string;
  emptyText: string;
  items: Array<{ id: string; title: string; updatedAt: number }>;
  activeId: string | null;
  deletable?: boolean;
}>();
const emit = defineEmits<{
  (e: "select", id: string): void;
  (e: "new"): void;
  (e: "delete", id: string): void;
}>();
</script>

<template>
  <aside class="threads" :aria-label="label">
    <div class="head">
      <p class="eyebrow plain">{{ label }}</p>
      <button type="button" class="sm" :title="newLabel" @click="emit('new')">+ New</button>
    </div>
    <ul v-if="items.length" class="list">
      <li v-for="t in items" :key="t.id" class="row">
        <button
          type="button"
          class="item"
          :class="{ active: t.id === activeId }"
          :aria-current="t.id === activeId ? 'true' : undefined"
          @click="emit('select', t.id)"
        >
          <span class="title">{{ t.title }}</span>
          <span class="time">{{ relativeTime(new Date(t.updatedAt).toISOString()) }}</span>
        </button>
        <button
          v-if="deletable"
          type="button"
          class="del"
          :aria-label="`Delete ${t.title}`"
          title="Delete"
          @click.stop="emit('delete', t.id)"
        >
          ×
        </button>
      </li>
    </ul>
    <p v-else class="muted small empty">{{ emptyText }}</p>
  </aside>
</template>

<style scoped>
.threads {
  position: sticky;
  top: calc(56px + 3px + 1.5rem);
  align-self: start;
  max-height: calc(100vh - 56px - 3px - 3rem);
  overflow-y: auto;
  border-right: 1px solid var(--hair);
  padding-right: 0.75rem;
}
.head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.4rem; }
.head .eyebrow { margin: 0; }
.list { list-style: none; margin: 0; padding: 0; display: grid; gap: 0.1rem; }
.row { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; }
.item {
  display: grid;
  justify-items: start;
  justify-content: stretch;
  gap: 0.05rem;
  width: 100%;
  min-width: 0;
  text-align: left;
  padding: 0.42rem 0.55rem;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--body);
  font-weight: 400;
  font-size: 0.88rem;
}
.item:hover:not(:disabled) { background: var(--sunk); }
.item.active { background: var(--sunk); box-shadow: inset 3px 0 0 var(--red); color: var(--ink); }
.title { max-width: 100%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.time { font-family: var(--font-mono); font-size: 0.66rem; color: var(--muted); }
.del {
  width: 1.6rem;
  height: 1.6rem;
  padding: 0;
  border: 0;
  color: var(--muted);
  opacity: 0;
}
.row:hover .del,
.row:focus-within .del { opacity: 1; }
.del:hover:not(:disabled) { color: var(--red-ink); background: var(--sunk); }
.empty { padding: 0.2rem 0.3rem; }

@media (max-width: 900px) {
  .threads { position: static; max-height: 12rem; border-right: 0; border-bottom: 1px solid var(--hair); padding: 0 0 0.75rem; }
}
</style>
