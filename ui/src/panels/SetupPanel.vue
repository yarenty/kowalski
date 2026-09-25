<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, type ModelChoice, type SetupStatus } from "../api";

const emit = defineEmits<{ (e: "done"): void }>();

const status = ref<SetupStatus | null>(null);
const err = ref<string | null>(null);
const notice = ref<string | null>(null);

const provider = ref<"ollama" | "openai">("ollama");
const ollamaModel = ref("llama3.2");
const preset = ref("openai");
const baseUrl = ref("https://api.openai.com/v1");
const hostedModel = ref("gpt-4o-mini");
const apiKey = ref("");
const filesDir = ref("");

const check = ref<{ ok: boolean; message: string } | null>(null);
const busy = ref<string | null>(null);

const PRESETS: Record<string, { base: string; model: string; label: string }> = {
  openai: { base: "https://api.openai.com/v1", model: "gpt-4o-mini", label: "OpenAI" },
  groq: { base: "https://api.groq.com/openai/v1", model: "llama-3.3-70b-versatile", label: "Groq" },
  openrouter: { base: "https://openrouter.ai/api/v1", model: "openai/gpt-4o-mini", label: "OpenRouter" },
  lmstudio: { base: "http://127.0.0.1:1234/v1", model: "local-model", label: "LM Studio (local)" },
  custom: { base: "", model: "", label: "Other OpenAI-compatible" },
};

function applyPreset() {
  const p = PRESETS[preset.value];
  if (p && preset.value !== "custom") {
    baseUrl.value = p.base;
    hostedModel.value = p.model;
  }
}

const choice = computed<ModelChoice>(() =>
  provider.value === "ollama"
    ? { provider: "ollama", model: ollamaModel.value.trim(), files_dir: filesDir.value.trim() || undefined }
    : {
        provider: "openai",
        model: hostedModel.value.trim(),
        openai_api_base: baseUrl.value.trim(),
        api_key: apiKey.value.trim() || undefined,
        files_dir: filesDir.value.trim() || undefined,
      },
);

async function load() {
  err.value = null;
  try {
    const s = await api.setupStatus();
    status.value = s;
    provider.value = s.provider === "openai" ? "openai" : s.ollama.reachable || s.provider === "ollama" ? "ollama" : "openai";
    if (s.provider === "openai") {
      hostedModel.value = s.model;
      if (s.openai_api_base) {
        baseUrl.value = s.openai_api_base;
        preset.value = Object.entries(PRESETS).find(([, p]) => p.base === s.openai_api_base)?.[0] ?? "custom";
      }
    } else {
      const same = (m: string) => m === s.model || m.split(":")[0] === s.model;
      ollamaModel.value = s.ollama.models.find(same) ?? s.ollama.models[0] ?? s.model ?? "llama3.2";
    }
    filesDir.value = s.files_dir ?? "";
  } catch (e) {
    err.value = e instanceof Error ? e.message : String(e);
  }
}

async function testModel() {
  busy.value = "check";
  check.value = null;
  try {
    check.value = await api.setupTestModel(choice.value);
  } catch (e) {
    check.value = { ok: false, message: e instanceof Error ? e.message : String(e) };
  } finally {
    busy.value = null;
  }
}

async function waitForServer() {
  const until = Date.now() + 30_000;
  await new Promise((r) => setTimeout(r, 900));
  while (Date.now() < until) {
    try {
      await api.health();
      return true;
    } catch {
      await new Promise((r) => setTimeout(r, 500));
    }
  }
  return false;
}

async function restart(message: string) {
  busy.value = "restart";
  notice.value = message;
  try {
    await api.setupRestart();
  } catch {
    /* the server may already be going down */
  }
  const back = await waitForServer();
  busy.value = null;
  if (back) {
    await load();
    notice.value = "Done. kowalski restarted with your settings.";
    emit("done");
  } else {
    err.value = "kowalski did not come back within 30 s. Start it again from the terminal.";
  }
}

async function saveAll() {
  err.value = null;
  busy.value = "save";
  try {
    await api.setupSave(choice.value);
    await restart("Saved. Restarting kowalski so every agent uses the new settings…");
  } catch (e) {
    err.value = e instanceof Error ? e.message : String(e);
    busy.value = null;
  }
}

async function connectTableski() {
  err.value = null;
  busy.value = "tableski";
  try {
    const { authorize_url } = await api.setupTableskiStart();
    window.location.href = authorize_url;
  } catch (e) {
    err.value = e instanceof Error ? e.message : String(e);
    busy.value = null;
  }
}

async function disconnectTableski() {
  if (!window.confirm("Disconnect tableski? Agents lose its SQL tools until you connect again.")) return;
  busy.value = "tableski";
  try {
    await api.setupTableskiDisconnect();
    await restart("Disconnected. Restarting kowalski…");
  } catch (e) {
    err.value = e instanceof Error ? e.message : String(e);
    busy.value = null;
  }
}

onMounted(async () => {
  const q = new URLSearchParams(window.location.search).get("setup");
  if (q) {
    window.history.replaceState({}, "", window.location.pathname);
    const messages: Record<string, string> = {
      "tableski-connected": "tableski connected.",
      "tableski-cancelled": "tableski sign-in was cancelled.",
      "tableski-expired": "That sign-in link expired. Press Connect again.",
      "tableski-failed": "tableski sign-in failed. Try again; the server log has the reason.",
    };
    if (q === "tableski-connected") {
      await restart("tableski connected. Restarting kowalski so agents get its tools…");
      return;
    }
    notice.value = messages[q] ?? null;
  }
  await load();
});
</script>

<template>
  <section class="setup">
    <header>
      <h2>Setup</h2>
      <p class="muted">Three answers and the rookery is ready. Everything lands in <code>{{ status?.write_path ?? "config.toml" }}</code>; edit it by hand any time.</p>
    </header>

    <p v-if="notice" class="notice">{{ notice }}</p>
    <p v-if="err" class="err">{{ err }}</p>

    <ol class="steps" v-if="status">
      <li>
        <h3><span class="n">1</span> Which brain?</h3>
        <div class="choices">
          <label :class="{ on: provider === 'ollama' }">
            <input v-model="provider" type="radio" value="ollama" />
            <b>Local, free</b> — Ollama on this machine
            <small>{{ status.ollama.reachable ? `running at ${status.ollama.url}, ${status.ollama.models.length} model(s)` : "not running (install from ollama.com)" }}</small>
          </label>
          <label :class="{ on: provider === 'openai' }">
            <input v-model="provider" type="radio" value="openai" />
            <b>Hosted</b> — any OpenAI-compatible endpoint, your own key
            <small>stronger answers; the provider bills you per use</small>
          </label>
        </div>

        <div v-if="provider === 'ollama'" class="fields">
          <label>Model
            <select v-if="status.ollama.models.length" v-model="ollamaModel">
              <option v-for="m in status.ollama.models" :key="m" :value="m">{{ m }}</option>
            </select>
            <input v-else v-model="ollamaModel" placeholder="llama3.2" spellcheck="false" />
          </label>
          <p v-if="!status.ollama.models.length" class="muted small">No models yet: <code>ollama pull {{ ollamaModel || "llama3.2" }}</code></p>
        </div>

        <div v-else class="fields">
          <label>Provider
            <select v-model="preset" @change="applyPreset">
              <option v-for="(p, k) in PRESETS" :key="k" :value="k">{{ p.label }}</option>
            </select>
          </label>
          <label>Endpoint <input v-model="baseUrl" placeholder="https://…/v1" spellcheck="false" /></label>
          <label>Model <input v-model="hostedModel" spellcheck="false" /></label>
          <label>API key
            <input v-model="apiKey" type="password" autocomplete="off" :placeholder="status.has_api_key ? 'saved; leave empty to keep it' : 'sk-…'" />
          </label>
          <p class="muted small">The key is stored in your config file with owner-only permissions (or set <code>OPENAI_API_KEY</code> and leave this empty).</p>
        </div>

        <div class="row">
          <button class="ghost" :disabled="busy !== null" @click="testModel">{{ busy === "check" ? "Checking…" : "Check" }}</button>
          <span v-if="check" :class="check.ok ? 'ok' : 'err'">{{ check.message }}</span>
        </div>
      </li>

      <li>
        <h3><span class="n">2</span> Where do your files live?</h3>
        <div class="fields">
          <label>Folder <input v-model="filesDir" placeholder="~/Documents/kowalski" spellcheck="false" /></label>
        </div>
        <p class="muted small">Chat's file tool reads and writes only inside this folder. Leave empty to decide per chat.</p>
      </li>

      <li>
        <h3><span class="n">3</span> Spreadsheets as SQL (tableski)</h3>
        <p v-if="status.tableski.connected" class="ok">Connected{{ status.tableski.signed_in ? " (signed in)" : "" }}: {{ status.tableski.url }}</p>
        <p v-else class="muted">Optional. Sign in to tableski.io and your agents can query your uploaded spreadsheets with SQL. Free plan, no card.</p>
        <div class="row">
          <button v-if="!status.tableski.connected" class="ghost" :disabled="busy !== null" @click="connectTableski">{{ busy === "tableski" ? "Opening tableski…" : "Connect tableski" }}</button>
          <button v-else class="ghost" :disabled="busy !== null" @click="disconnectTableski">Disconnect</button>
        </div>
      </li>
    </ol>

    <div class="save" v-if="status">
      <button :disabled="busy !== null" @click="saveAll">{{ busy === "save" || busy === "restart" ? "Working…" : "Save and restart" }}</button>
    </div>
  </section>
</template>

<style scoped>
.setup { max-width: 46rem; margin: 0 auto; }
header h2 { margin: 0 0 0.3rem; font-size: 1.2rem; }
.muted { color: #9aa3b2; }
.small { font-size: 0.85rem; }
code { background: #1c1f27; border: 1px solid #2a2e38; border-radius: 4px; padding: 0 0.3rem; }
.notice { border-left: 3px solid #7aa2f7; background: #1a1d25; padding: 0.6rem 0.8rem; border-radius: 4px; }
.err { color: #f7768e; }
.ok { color: #9ece6a; }
.steps { list-style: none; padding: 0; margin: 1.2rem 0; display: grid; gap: 1rem; }
.steps > li { border: 1px solid #2a2e38; border-radius: 10px; padding: 1rem 1.1rem; background: #15171e; }
h3 { margin: 0 0 0.7rem; font-size: 1rem; display: flex; align-items: center; gap: 0.5rem; }
.n { display: inline-grid; place-items: center; width: 1.5rem; height: 1.5rem; border-radius: 50%; background: #7aa2f7; color: #0f1117; font-size: 0.8rem; font-weight: 700; }
.choices { display: grid; gap: 0.5rem; margin-bottom: 0.8rem; }
.choices label { border: 1px solid #2a2e38; border-radius: 8px; padding: 0.6rem 0.8rem; cursor: pointer; display: block; }
.choices label.on { border-color: #7aa2f7; background: #1a1f2e; }
.choices small { display: block; color: #9aa3b2; margin: 0.2rem 0 0 1.4rem; }
.fields { display: grid; gap: 0.6rem; }
.fields label { display: grid; gap: 0.25rem; font-size: 0.9rem; }
.fields input, .fields select { background: #0f1117; color: inherit; border: 1px solid #2a2e38; border-radius: 6px; padding: 0.5rem 0.6rem; font: inherit; }
.row { display: flex; gap: 0.8rem; align-items: center; flex-wrap: wrap; margin-top: 0.7rem; }
button { background: #7aa2f7; color: #0f1117; border: 0; border-radius: 6px; padding: 0.55rem 1rem; font-weight: 600; cursor: pointer; }
button.ghost { background: transparent; color: #c0caf5; border: 1px solid #3b4252; }
button:disabled { opacity: 0.5; cursor: default; }
.save { display: flex; justify-content: flex-end; }
</style>
