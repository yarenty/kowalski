<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { setTheme, themeChoice, type ThemeChoice } from "../theme";
import type { TabId } from "../nav";

const props = defineProps<{ activeTab: TabId; appVersion: string; needsYou: number }>();
const emit = defineEmits<{
  (e: "select-tab", tab: TabId): void;
  (e: "open-picker"): void;
}>();

type NavItem = { id: TabId; label: string };

/** Everyday work, in the order a non-technical operator needs it. */
const primaryItems: NavItem[] = [
  { id: "federation-run", label: "Hordes" },
  { id: "runs", label: "Runs" },
  { id: "chat", label: "Chat" },
  { id: "rookery", label: "Build" },
  { id: "setup", label: "Setup" },
];

/** Technical screens, folded under Admin. */
const adminItems: NavItem[] = [
  { id: "federation-management", label: "Federation" },
  { id: "mcp", label: "MCP servers" },
  { id: "graph", label: "Graph" },
  { id: "home", label: "Diagnostics" },
  { id: "about", label: "About" },
];

const isAdminTab = computed(() => adminItems.some((i) => i.id === props.activeTab));
const activeLabel = computed(
  () => [...primaryItems, ...adminItems].find((i) => i.id === props.activeTab)?.label ?? "Menu",
);

const themeOptions: Array<{ id: ThemeChoice; label: string; d: string }> = [
  { id: "light", label: "Light", d: "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4" },
  { id: "dark", label: "Dark", d: "M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z" },
  { id: "system", label: "Auto", d: "M12 3a9 9 0 1 0 0 18zM12 3a9 9 0 0 1 0 18" },
];

/** One open menu at a time: the Admin dropdown or (narrow windows) the whole nav. */
const openMenu = ref<"admin" | "nav" | null>(null);
const adminBtn = ref<HTMLButtonElement | null>(null);
const navBtn = ref<HTMLButtonElement | null>(null);
const adminMenu = ref<HTMLElement | null>(null);
const navMenu = ref<HTMLElement | null>(null);
const menuEl = computed(() => (openMenu.value === "nav" ? navMenu.value : adminMenu.value));

async function toggleMenu(which: "admin" | "nav") {
  openMenu.value = openMenu.value === which ? null : which;
  if (openMenu.value) {
    await nextTick();
    (menuEl.value?.querySelector('[role="menuitem"]') as HTMLElement | null)?.focus();
  }
}

function closeMenu(returnFocus = false) {
  const which = openMenu.value;
  openMenu.value = null;
  if (returnFocus) (which === "nav" ? navBtn.value : adminBtn.value)?.focus();
}

function pick(id: TabId) {
  closeMenu();
  emit("select-tab", id);
}

function onMenuKey(e: KeyboardEvent) {
  const items = [...(menuEl.value?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])];
  const i = items.indexOf(document.activeElement as HTMLElement);
  if (e.key === "Escape") {
    e.preventDefault();
    closeMenu(true);
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    items[(i + 1) % items.length]?.focus();
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    items[(i - 1 + items.length) % items.length]?.focus();
  } else if (e.key === "Home") {
    e.preventDefault();
    items[0]?.focus();
  } else if (e.key === "End") {
    e.preventDefault();
    items[items.length - 1]?.focus();
  } else if (e.key === "Tab") {
    closeMenu();
  }
}

function onDocClick(e: MouseEvent) {
  if (!openMenu.value) return;
  const t = e.target as Node;
  if (adminMenu.value?.contains(t) || navMenu.value?.contains(t) || adminBtn.value?.contains(t) || navBtn.value?.contains(t)) return;
  closeMenu();
}

const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
const shortcut = isMac ? "⌘K" : "Ctrl K";

onMounted(() => document.addEventListener("mousedown", onDocClick));
onUnmounted(() => document.removeEventListener("mousedown", onDocClick));
</script>

<template>
  <header class="topbar">
    <div class="bar">
      <a class="brand" href="?" aria-label="Kowalski — Hordes home" @click.prevent="emit('select-tab', 'federation-run')">
        <img class="mark" src="/favicon.svg" alt="" width="30" height="30" />
        <span class="wordmark">KOWALSKI</span>
      </a>

      <nav class="nav" aria-label="Primary">
        <button
          v-for="item in primaryItems"
          :key="item.id"
          type="button"
          class="nav-item"
          :class="{ active: activeTab === item.id }"
          :aria-current="activeTab === item.id ? 'page' : undefined"
          @click="emit('select-tab', item.id)"
        >
          {{ item.label }}
          <span v-if="item.id === 'runs' && needsYou > 0" class="count" :aria-label="`${needsYou} need you`">{{ needsYou }}</span>
        </button>
        <div class="menu-wrap">
          <button
            ref="adminBtn"
            type="button"
            class="nav-item"
            :class="{ active: isAdminTab }"
            aria-haspopup="menu"
            :aria-expanded="openMenu === 'admin'"
            @click="toggleMenu('admin')"
          >
            Admin <span class="caret" aria-hidden="true">▾</span>
          </button>
          <div v-if="openMenu === 'admin'" ref="adminMenu" class="menu" role="menu" aria-label="Admin" @keydown="onMenuKey">
            <button
              v-for="item in adminItems"
              :key="item.id"
              type="button"
              role="menuitem"
              class="menu-item"
              :class="{ active: activeTab === item.id }"
              @click="pick(item.id)"
            >
              {{ item.label }}
            </button>
            <p class="menu-foot">v{{ appVersion }}</p>
          </div>
        </div>
      </nav>

      <div class="menu-wrap nav-compact">
        <button
          ref="navBtn"
          type="button"
          class="nav-item active"
          aria-haspopup="menu"
          :aria-expanded="openMenu === 'nav'"
          @click="toggleMenu('nav')"
        >
          {{ activeLabel }} <span class="caret" aria-hidden="true">▾</span>
        </button>
        <div v-if="openMenu === 'nav'" ref="navMenu" class="menu" role="menu" aria-label="Navigation" @keydown="onMenuKey">
          <button
            v-for="item in primaryItems"
            :key="item.id"
            type="button"
            role="menuitem"
            class="menu-item"
            :class="{ active: activeTab === item.id }"
            @click="pick(item.id)"
          >
            {{ item.label }}
          </button>
          <p class="menu-label">Admin</p>
          <button
            v-for="item in adminItems"
            :key="item.id"
            type="button"
            role="menuitem"
            class="menu-item"
            :class="{ active: activeTab === item.id }"
            @click="pick(item.id)"
          >
            {{ item.label }}
          </button>
          <p class="menu-foot">v{{ appVersion }}</p>
        </div>
      </div>

      <div class="right">
        <button type="button" class="search" aria-label="Find a horde or a run" :aria-keyshortcuts="isMac ? 'Meta+K' : 'Control+K'" @click="emit('open-picker')">
          <svg class="ico" viewBox="0 0 24 24" aria-hidden="true"><path d="M10.5 4a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13zM20 20l-4.8-4.8" /></svg>
          <span class="search-text">Find a horde or a run…</span>
          <kbd>{{ shortcut }}</kbd>
        </button>
        <div class="theme-toggle" role="radiogroup" aria-label="Colour theme">
          <button
            v-for="opt in themeOptions"
            :key="opt.id"
            type="button"
            role="radio"
            :aria-checked="themeChoice === opt.id"
            :aria-label="opt.label"
            :title="`${opt.label} theme`"
            :class="{ on: themeChoice === opt.id }"
            @click="setTheme(opt.id)"
          >
            <svg class="ico" viewBox="0 0 24 24" aria-hidden="true"><path :d="opt.d" /></svg>
          </button>
        </div>
      </div>
    </div>
  </header>
</template>

<style scoped>
.topbar {
  position: sticky;
  top: 0;
  z-index: 30;
  background: var(--rail);
  color: var(--rail-text);
  border-bottom: 3px solid var(--red);
}
.bar {
  height: 56px;
  display: flex;
  align-items: center;
  gap: 1.5rem;
  padding: 0 1.25rem;
  max-width: 100%;
}
.brand { display: flex; align-items: center; gap: 0.6rem; text-decoration: none; flex: 0 0 auto; }
.brand:focus-visible { outline: 2px solid var(--rail-text); outline-offset: 3px; border-radius: var(--radius-sm); }
.mark { width: 30px; height: 30px; border-radius: 6px; }
.wordmark {
  font-family: var(--font-display);
  font-weight: 900;
  font-size: 1.22rem;
  letter-spacing: 0.03em;
  color: var(--rail-text);
}

.nav { display: flex; align-items: stretch; height: 100%; gap: 0.1rem; min-width: 0; }
.nav-item {
  height: 100%;
  border: 0;
  border-radius: 0;
  padding: 0 0.8rem;
  background: transparent;
  color: var(--rail-muted);
  font-family: var(--font-mono);
  font-size: 0.76rem;
  font-weight: 600;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  box-shadow: inset 0 -3px 0 transparent;
  transition: none;
}
.nav-item:hover:not(:disabled) { background: transparent; color: var(--rail-text); }
.nav-item:active:not(:disabled) { transform: none; }
.nav-item.active { color: var(--rail-text); box-shadow: inset 0 -3px 0 var(--red); }
.nav-item:focus-visible { outline: 2px solid var(--rail-text); outline-offset: -6px; }
.caret { font-size: 0.7rem; margin-left: 0.15rem; }
.count {
  min-width: 1.15rem;
  height: 1.15rem;
  padding: 0 0.3rem;
  border-radius: 999px;
  background: var(--red);
  color: var(--on-red);
  font-size: 0.66rem;
  letter-spacing: 0;
  display: inline-grid;
  place-items: center;
}

.menu-wrap { position: relative; height: 100%; display: flex; }
.menu {
  position: absolute;
  top: calc(100% + 3px);
  left: 0;
  min-width: 13rem;
  background: var(--surface);
  border: 1px solid var(--line);
  border-top: 3px solid var(--ink);
  border-radius: 0 0 var(--radius) var(--radius);
  box-shadow: var(--pop-shadow);
  padding: 0.35rem;
  display: grid;
  gap: 0.1rem;
  z-index: 40;
}
.menu-item {
  justify-content: flex-start;
  border: 0;
  padding: 0.5rem 0.65rem;
  border-radius: var(--radius-sm);
  font-weight: 500;
  color: var(--body);
}
.menu-item:hover:not(:disabled),
.menu-item:focus-visible { background: var(--sunk); color: var(--ink); outline: none; }
.menu-item.active { color: var(--ink); box-shadow: inset 3px 0 0 var(--red); }
.menu-label,
.menu-foot {
  margin: 0.4rem 0.65rem 0.2rem;
  font-family: var(--font-mono);
  font-size: 0.66rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted);
}
.menu-foot { border-top: 1px solid var(--hair); padding-top: 0.45rem; margin-top: 0.3rem; }

.nav-compact { display: none; }

.right { margin-left: auto; display: flex; align-items: center; gap: 0.75rem; flex: 0 0 auto; }
.search {
  width: 17.5rem;
  justify-content: flex-start;
  gap: 0.55rem;
  padding: 0.42rem 0.6rem;
  border: 1px solid var(--rail-line);
  background: var(--rail-raised);
  color: var(--rail-muted);
  font-weight: 400;
  font-size: 0.86rem;
}
.search:hover:not(:disabled) { background: var(--rail-raised); border-color: var(--rail-muted); color: var(--rail-text); }
.search:focus-visible { outline: 2px solid var(--rail-text); outline-offset: 2px; }
.search-text { flex: 1; text-align: left; overflow: hidden; text-overflow: ellipsis; }
kbd {
  font-size: 0.68rem;
  padding: 0.1rem 0.35rem;
  border: 1px solid var(--rail-line);
  border-radius: var(--radius-sm);
  color: var(--rail-muted);
}
.ico {
  width: 16px;
  height: 16px;
  flex: 0 0 auto;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.theme-toggle {
  display: flex;
  border: 1px solid var(--rail-line);
  border-radius: var(--radius);
  overflow: hidden;
}
.theme-toggle button {
  border: 0;
  border-radius: 0;
  width: 1.9rem;
  height: 1.8rem;
  padding: 0;
  color: var(--rail-muted);
  background: transparent;
}
.theme-toggle button + button { border-left: 1px solid var(--rail-line); }
.theme-toggle button:hover:not(:disabled) { background: var(--rail-raised); color: var(--rail-text); }
.theme-toggle button.on { background: var(--rail-text); color: var(--rail); }
.theme-toggle button:focus-visible { outline: 2px solid var(--red); outline-offset: -2px; }

@media (max-width: 1240px) {
  .search { width: auto; }
  .search-text { display: none; }
  .bar { gap: 1rem; }
}
@media (max-width: 1000px) {
  .nav { display: none; }
  .nav-compact { display: flex; }
}
@media (max-width: 520px) {
  .wordmark { display: none; }
  .bar { padding: 0 0.75rem; }
}
</style>
