<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { setTheme, themeChoice, type ThemeChoice } from "../theme";

type TabId =
  | "home"
  | "mcp"
  | "chat"
  | "rookery"
  | "federation-management"
  | "federation-run"
  | "graph"
  | "about"
  | "setup";
type ConversationItem = {
  id: string;
  title: string;
  updatedAt: number;
};
type HordeInteractionItem = {
  id: string;
  title: string;
  updatedAt: number;
};
type RookerySessionItem = {
  id: string;
  title: string;
  updatedAt: number;
};

const props = defineProps<{
  activeTab: TabId;
  collapsed: boolean;
  conversations: ConversationItem[];
  activeConversationId: string | null;
  hordeInteractions: HordeInteractionItem[];
  activeHordeInteractionId: string | null;
  rookerySessions: RookerySessionItem[];
  activeRookerySessionId: string | null;
  appVersion: string;
}>();

const emit = defineEmits<{
  (e: "select-tab", tab: TabId): void;
  (e: "toggle-collapse"): void;
  (e: "select-conversation", id: string): void;
  (e: "new-conversation"): void;
  (e: "select-horde-interaction", id: string): void;
  (e: "new-horde-interaction"): void;
  (e: "delete-horde-interaction", id: string): void;
  (e: "select-rookery-session", id: string): void;
  (e: "new-rookery-session"): void;
  (e: "delete-rookery-session", id: string): void;
}>();

type NavItem = { id: TabId; label: string; icon: string };

/** Everyday work, in the order a non-technical operator needs it. */
const primaryItems: NavItem[] = [
  { id: "federation-run", label: "Hordes", icon: "hordes" },
  { id: "chat", label: "Chat", icon: "chat" },
  { id: "rookery", label: "Build", icon: "build" },
  { id: "setup", label: "Setup", icon: "setup" },
];

/** Technical screens, folded under Admin. */
const adminItems: NavItem[] = [
  { id: "federation-management", label: "Federation", icon: "federation" },
  { id: "mcp", label: "MCP servers", icon: "mcp" },
  { id: "graph", label: "Graph", icon: "graph" },
  { id: "home", label: "Diagnostics", icon: "diagnostics" },
  { id: "about", label: "About", icon: "about" },
];

const isAdminTab = computed(() => adminItems.some((i) => i.id === props.activeTab));
const adminOpen = ref(isAdminTab.value);
watch(isAdminTab, (v) => {
  if (v) adminOpen.value = true;
});

const themeOptions: Array<{ id: ThemeChoice; label: string }> = [
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
  { id: "system", label: "Auto" },
];

/** 24×24 stroke icons (currentColor). */
const ICONS: Record<string, string> = {
  hordes:
    "M8 7.5a2.5 2.5 0 1 0 0-.01M16 7.5a2.5 2.5 0 1 0 0-.01M3.5 19c.4-3 2.2-5 4.5-5s4.1 2 4.5 5M11.5 19c.4-3 2.2-5 4.5-5s4.1 2 4.5 5",
  chat: "M4 5h16v11H9l-5 4z",
  build: "M4 20h16M6 20V10l6-5 6 5v10M10 20v-5h4v5",
  setup: "M5 6h14M5 12h14M5 18h14M9 4v4M15 10v4M8 16v4",
  admin:
    "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1",
  federation: "M6 6h4v4H6zM14 6h4v4h-4zM10 14h4v4h-4zM8 10v2h8v-2M12 12v2",
  mcp: "M9 4v5M15 4v5M7 9h10v3a5 5 0 0 1-10 0zM12 17v3",
  graph: "M6 18a2 2 0 1 0 0-.01M18 6a2 2 0 1 0 0-.01M18 18a2 2 0 1 0 0-.01M7.5 16.5l9-9M8 18h8",
  diagnostics: "M3 12h4l2-5 4 10 2-5h6",
  about: "M12 8h.01M11 11h1v6h1M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z",
};

function timeAgo(ts: number): string {
  const diff = Math.max(0, Date.now() - ts);
  const sec = Math.floor(diff / 1000);
  if (sec < 60) return "now";
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  return `${day}d ago`;
}
</script>

<template>
  <aside class="rail" :class="{ collapsed }" aria-label="Main navigation">
    <div class="brand">
      <img class="mark" src="/favicon.svg" alt="" width="34" height="34" />
      <div v-if="!collapsed" class="wordmark-wrap">
        <span class="wordmark">KOWALSKI</span>
        <span class="tagline">Field ops // penguins</span>
      </div>
    </div>

    <nav class="nav" aria-label="Primary">
      <button
        v-for="item in primaryItems"
        :key="item.id"
        type="button"
        class="nav-item"
        :class="{ active: activeTab === item.id }"
        :aria-current="activeTab === item.id ? 'page' : undefined"
        :title="collapsed ? item.label : undefined"
        @click="emit('select-tab', item.id)"
      >
        <svg class="ico" viewBox="0 0 24 24" aria-hidden="true"><path :d="ICONS[item.icon]" /></svg>
        <span v-if="!collapsed">{{ item.label }}</span>
      </button>
    </nav>

    <template v-if="!collapsed">
      <section v-if="activeTab === 'federation-run'" class="threads">
        <div class="threads-head">
          <span class="rail-label">Run threads</span>
          <button type="button" class="new-btn" title="New run thread" aria-label="New run thread" @click="emit('new-horde-interaction')">+</button>
        </div>
        <div class="threads-scroll">
          <div v-for="h in hordeInteractions" :key="h.id" class="thread-row">
            <button
              type="button"
              class="thread-btn"
              :class="{ active: h.id === activeHordeInteractionId }"
              @click="emit('select-horde-interaction', h.id)"
            >
              <span class="title">{{ h.title }}</span>
              <span class="time">{{ timeAgo(h.updatedAt) }}</span>
            </button>
            <button
              type="button"
              class="delete-btn"
              title="Delete thread"
              aria-label="Delete thread"
              @click.stop="emit('delete-horde-interaction', h.id)"
            >
              ×
            </button>
          </div>
          <p v-if="!hordeInteractions.length" class="rail-empty">Finished runs are kept here.</p>
        </div>
      </section>

      <section v-if="activeTab === 'chat'" class="threads">
        <div class="threads-head">
          <span class="rail-label">Conversations</span>
          <button type="button" class="new-btn" title="New conversation" aria-label="New conversation" @click="emit('new-conversation')">+</button>
        </div>
        <div class="threads-scroll">
          <button
            v-for="c in conversations"
            :key="c.id"
            type="button"
            class="thread-btn"
            :class="{ active: c.id === activeConversationId }"
            @click="emit('select-conversation', c.id)"
          >
            <span class="title">{{ c.title }}</span>
          </button>
          <p v-if="!conversations.length" class="rail-empty">No conversations yet.</p>
        </div>
      </section>

      <section v-if="activeTab === 'rookery'" class="threads">
        <div class="threads-head">
          <span class="rail-label">Build sessions</span>
          <button type="button" class="new-btn" title="New build session" aria-label="New build session" @click="emit('new-rookery-session')">+</button>
        </div>
        <div class="threads-scroll">
          <div v-for="r in rookerySessions" :key="r.id" class="thread-row">
            <button
              type="button"
              class="thread-btn"
              :class="{ active: r.id === activeRookerySessionId }"
              @click="emit('select-rookery-session', r.id)"
            >
              <span class="title">{{ r.title }}</span>
              <span class="time">{{ timeAgo(r.updatedAt) }}</span>
            </button>
            <button
              type="button"
              class="delete-btn"
              title="Delete session"
              aria-label="Delete session"
              @click.stop="emit('delete-rookery-session', r.id)"
            >
              ×
            </button>
          </div>
          <p v-if="!rookerySessions.length" class="rail-empty">No build sessions yet.</p>
        </div>
      </section>
    </template>

    <div class="rail-foot">
      <section class="admin">
        <button
          type="button"
          class="nav-item admin-toggle"
          :class="{ 'has-active': isAdminTab && !adminOpen }"
          :aria-expanded="adminOpen"
          :title="collapsed ? 'Admin' : undefined"
          @click="collapsed ? emit('toggle-collapse') : (adminOpen = !adminOpen)"
        >
          <svg class="ico" viewBox="0 0 24 24" aria-hidden="true"><path :d="ICONS.admin" /></svg>
          <template v-if="!collapsed">
            <span>Admin</span>
            <span class="caret" aria-hidden="true">{{ adminOpen ? "▾" : "▸" }}</span>
          </template>
        </button>
        <div v-if="adminOpen && !collapsed" class="admin-nav">
          <button
            v-for="item in adminItems"
            :key="item.id"
            type="button"
            class="nav-item sub"
            :class="{ active: activeTab === item.id }"
            :aria-current="activeTab === item.id ? 'page' : undefined"
            @click="emit('select-tab', item.id)"
          >
            <svg class="ico" viewBox="0 0 24 24" aria-hidden="true"><path :d="ICONS[item.icon]" /></svg>
            <span>{{ item.label }}</span>
          </button>
        </div>
      </section>

      <div v-if="!collapsed" class="theme-toggle" role="radiogroup" aria-label="Colour theme">
        <button
          v-for="opt in themeOptions"
          :key="opt.id"
          type="button"
          role="radio"
          :aria-checked="themeChoice === opt.id"
          :class="{ on: themeChoice === opt.id }"
          @click="setTheme(opt.id)"
        >
          {{ opt.label }}
        </button>
      </div>
      <div class="foot-row">
        <span v-if="!collapsed" class="version">v{{ appVersion }}</span>
        <button
          type="button"
          class="collapse-btn"
          :title="collapsed ? 'Expand sidebar' : 'Collapse sidebar'"
          :aria-label="collapsed ? 'Expand sidebar' : 'Collapse sidebar'"
          @click="emit('toggle-collapse')"
        >
          <span aria-hidden="true">{{ collapsed ? "»" : "«" }}</span>
        </button>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.rail {
  width: 248px;
  flex-shrink: 0;
  position: sticky;
  top: 0;
  height: 100vh;
  overflow-y: auto;
  background: var(--rail);
  color: var(--rail-text);
  display: flex;
  flex-direction: column;
  padding: 0.9rem 0.75rem 0.75rem;
  border-right: 3px solid var(--red);
}
.rail.collapsed { width: 64px; padding: 0.9rem 0.5rem 0.75rem; align-items: center; }

.brand { display: flex; align-items: center; gap: 0.65rem; padding: 0 0.25rem; }
.mark { width: 34px; height: 34px; border-radius: 7px; flex: 0 0 auto; }
.wordmark-wrap { display: grid; line-height: 1.05; }
.wordmark {
  font-family: var(--font-display);
  font-weight: 900;
  font-size: 1.28rem;
  letter-spacing: 0.02em;
  color: var(--rail-text);
}
.tagline {
  font-family: var(--font-mono);
  font-size: 0.62rem;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--rail-muted);
  margin-top: 0.2rem;
}

.foot-row { display: flex; align-items: center; justify-content: space-between; margin-top: 0.6rem; gap: 0.5rem; }
.rail.collapsed .foot-row { justify-content: center; }
.collapse-btn {
  padding: 0.15rem 0.45rem;
  font-size: 0.85rem;
  border: 1px solid var(--rail-line);
  color: var(--rail-muted);
  background: transparent;
}
.collapse-btn:hover:not(:disabled) { background: var(--rail-raised); color: var(--rail-text); }
.collapse-btn:focus-visible { outline-color: var(--rail-text); }

.nav { display: grid; gap: 0.2rem; width: 100%; margin-top: 1.4rem; }

.nav-item {
  justify-content: flex-start;
  width: 100%;
  gap: 0.7rem;
  padding: 0.6rem 0.7rem;
  border: 0;
  border-left: 3px solid transparent;
  border-radius: 0 var(--radius) var(--radius) 0;
  background: transparent;
  color: var(--rail-muted);
  font-family: var(--font-display);
  font-size: 1rem;
  font-weight: 700;
  letter-spacing: 0.01em;
  transition: none;
}
.rail.collapsed .nav-item { justify-content: center; padding: 0.6rem 0; border-radius: var(--radius); }
.nav-item:hover:not(:disabled) { background: var(--rail-raised); color: var(--rail-text); }
.nav-item.active {
  color: var(--rail-text);
  background: var(--rail-raised);
  border-left-color: var(--red);
}
.nav-item:focus-visible { outline: 2px solid var(--red); outline-offset: -2px; }
.nav-item.sub { font-family: var(--font-body); font-size: 0.9rem; font-weight: 500; padding: 0.42rem 0.7rem; }
.ico {
  width: 20px;
  height: 20px;
  flex: 0 0 auto;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}
.nav-item.sub .ico { width: 16px; height: 16px; }
.nav-item.active .ico { stroke: var(--red); }

.threads {
  margin-top: 1rem;
  padding-top: 0.8rem;
  border-top: 1px solid var(--rail-line);
  min-height: 0;
}
.threads-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.4rem; padding: 0 0.25rem; }
.rail-label {
  font-family: var(--font-mono);
  font-size: 0.66rem;
  font-weight: 600;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--rail-muted);
}
.new-btn {
  width: 26px;
  height: 26px;
  padding: 0;
  font-size: 1.05rem;
  border: 1px solid var(--rail-line);
  color: var(--rail-text);
  background: transparent;
}
.new-btn:hover:not(:disabled) { border-color: var(--red); background: var(--rail-raised); }
.threads-scroll { max-height: clamp(8rem, 34vh, 20rem); overflow-y: auto; display: grid; gap: 0.15rem; }
.thread-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 0.2rem; }
.thread-btn {
  display: block;
  width: 100%;
  min-width: 0;
  text-align: left;
  padding: 0.4rem 0.55rem;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--rail-text);
  font-weight: 400;
  font-size: 0.86rem;
}
.thread-btn:hover:not(:disabled) { background: var(--rail-raised); }
.thread-btn.active { background: var(--rail-raised); box-shadow: inset 2px 0 0 var(--red); }
.thread-btn:focus-visible { outline-offset: -2px; }
.title { display: block; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.time { display: block; font-family: var(--font-mono); font-size: 0.66rem; color: var(--rail-muted); margin-top: 0.1rem; }
.delete-btn {
  width: 24px;
  height: 24px;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--rail-muted);
  font-size: 1.05rem;
  opacity: 0;
}
.thread-row:hover .delete-btn,
.thread-row:focus-within .delete-btn { opacity: 1; }
.delete-btn:hover:not(:disabled) { color: var(--red); background: var(--rail-raised); }
.rail-empty { color: var(--rail-muted); font-size: 0.82rem; padding: 0.2rem 0.3rem; margin: 0; }

.rail-foot { margin-top: auto; padding-top: 0.8rem; width: 100%; }
.admin { border-top: 1px solid var(--rail-line); padding-top: 0.5rem; }
.admin-toggle { font-size: 0.9rem; }
.admin-toggle.has-active { border-left-color: var(--red); color: var(--rail-text); }
.caret { margin-left: auto; font-size: 0.8rem; }
.admin-nav { display: grid; gap: 0.1rem; margin: 0.15rem 0 0 0.6rem; }

.theme-toggle {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  margin-top: 0.75rem;
  border: 1px solid var(--rail-line);
  border-radius: var(--radius);
  overflow: hidden;
}
.theme-toggle button {
  border: 0;
  border-radius: 0;
  padding: 0.35rem 0;
  font-family: var(--font-mono);
  font-size: 0.68rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--rail-muted);
  background: transparent;
}
.theme-toggle button + button { border-left: 1px solid var(--rail-line); }
.theme-toggle button:hover:not(:disabled) { background: var(--rail-raised); color: var(--rail-text); }
.theme-toggle button.on { background: var(--rail-text); color: var(--rail); }
.theme-toggle button:focus-visible { outline-offset: -3px; }
.version {
  margin: 0;
  font-family: var(--font-mono);
  font-size: 0.66rem;
  letter-spacing: 0.1em;
  color: var(--rail-muted);
  text-align: center;
}
</style>
