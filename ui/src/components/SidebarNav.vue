<script setup lang="ts">
type TabId = "home" | "mcp" | "chat" | "federation" | "graph";
type ConversationItem = {
  id: string;
  title: string;
  updatedAt: number;
};

defineProps<{
  activeTab: TabId;
  collapsed: boolean;
  conversations: ConversationItem[];
  activeConversationId: string | null;
}>();

const emit = defineEmits<{
  (e: "select-tab", tab: TabId): void;
  (e: "toggle-collapse"): void;
  (e: "select-conversation", id: string): void;
  (e: "new-conversation"): void;
}>();
</script>

<template>
  <aside class="sidebar" :class="{ collapsed }">
    <button class="collapse-btn" @click="emit('toggle-collapse')">
      {{ collapsed ? "»" : "«" }}
    </button>
    <div v-if="!collapsed">
      <h1>Kowalski</h1>
      <p class="tagline">Operator UI</p>
      <nav class="nav">
        <button :class="{ active: activeTab === 'chat' }" @click="emit('select-tab', 'chat')">Chat</button>
        <button :class="{ active: activeTab === 'home' }" @click="emit('select-tab', 'home')">Home</button>
        <button :class="{ active: activeTab === 'mcp' }" @click="emit('select-tab', 'mcp')">MCP</button>
        <button :class="{ active: activeTab === 'federation' }" @click="emit('select-tab', 'federation')">Federation</button>
        <button :class="{ active: activeTab === 'graph' }" @click="emit('select-tab', 'graph')">Graph</button>
      </nav>

      <section v-if="activeTab === 'chat'" class="chat-list">
        <div class="chat-list-head">
          <strong>Conversations</strong>
          <button class="new-btn" @click="emit('new-conversation')">+</button>
        </div>
        <div class="chat-list-scroll">
          <button
            v-for="c in conversations"
            :key="c.id"
            class="conv-btn"
            :class="{ active: c.id === activeConversationId }"
            @click="emit('select-conversation', c.id)"
          >
            <span class="title">{{ c.title }}</span>
          </button>
          <p v-if="!conversations.length" class="muted">No conversations yet.</p>
        </div>
      </section>
    </div>
  </aside>
</template>

<style scoped>
.sidebar { width: 260px; border-right: 1px solid #2a2e38; background: #171b22; padding: 0.75rem; box-sizing: border-box; }
.sidebar.collapsed { width: 42px; padding: 0.5rem 0.35rem; }
.collapse-btn { width: 100%; margin-bottom: 0.5rem; background: #2a3142; border: 1px solid #3d4658; color: #c8cfdd; border-radius: 6px; cursor: pointer; }
h1 { margin: 0; font-size: 1.05rem; }
.tagline { margin: 0.3rem 0 0.6rem; color: #8b92a5; font-size: 0.8rem; }
.nav { display: grid; gap: 0.35rem; }
.nav button, .conv-btn, .new-btn {
  background: #2a3142; border: 1px solid #3d4658; color: #c8cfdd; padding: 0.35rem 0.55rem; border-radius: 6px; cursor: pointer; text-align: left;
}
.nav button.active, .conv-btn.active { background: #3d5a8c; border-color: #5a7ab8; color: #fff; }
.chat-list { margin-top: 0.8rem; border-top: 1px solid #2a2e38; padding-top: 0.6rem; }
.chat-list-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.4rem; }
.new-btn { width: 28px; height: 28px; padding: 0; text-align: center; }
.chat-list-scroll { max-height: calc(100vh - 260px); overflow: auto; display: grid; gap: 0.35rem; }
.title { display: block; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.muted { color: #6a7285; font-size: 0.85rem; }
</style>
