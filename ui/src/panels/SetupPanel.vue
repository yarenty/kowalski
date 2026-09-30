<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, type ModelChoice, type SetupStatus } from "../api";
import SecretInput from "../components/SecretInput.vue";
import { takeSetupOutcome } from "../nav";

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
const searchKey = ref("");
const searchProvider = ref("duckduckgo");
const searchEngineId = ref("");

/** Search providers Setup offers (`[search] provider`); SearXNG is set in the config file. */
const SEARCH_CHOICES: Record<string, { label: string; key?: string; link?: string; note: string }> = {
  duckduckgo: { label: "DuckDuckGo (no key)", note: "Free and works right away. It is not an official API, so it may slow down or refuse now and then; a key below is steadier." },
  brave: { label: "Brave Search", key: "Brave Search API key", link: "https://brave.com/search/api/", note: "Free monthly allowance, then paid." },
  staan: { label: "Staan (European index)", key: "Staan API key", link: "https://staan.ai/", note: "Free monthly allowance, then paid." },
  tavily: { label: "Tavily (built for agents)", key: "Tavily API key", link: "https://tavily.com/", note: "About 1,000 free searches a month, then paid." },
  serper: { label: "Google, via Serper", key: "Serper API key", link: "https://serper.dev/", note: "About 2,500 free searches to start, then paid per search." },
  google: { label: "Google Programmable Search", key: "Google API key", link: "https://developers.google.com/custom-search/v1/overview", note: "100 free searches a day, then paid. Needs a key and a search engine ID." },
  off: { label: "Off", note: "Agents only read pages you give them." },
};
const searchChoice = computed(() => SEARCH_CHOICES[searchProvider.value]);

const check = ref<{ ok: boolean; message: string } | null>(null);
/** The form as last loaded from the server: Save is offered only when something differs. */
const saved = ref("");
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
    ? { provider: "ollama", model: ollamaModel.value.trim(), files_dir: filesDir.value.trim() || undefined, search_api_key: searchKey.value.trim() || undefined, search_provider: searchProvider.value, search_engine_id: searchEngineId.value.trim() || undefined }
    : {
        provider: "openai",
        model: hostedModel.value.trim(),
        openai_api_base: baseUrl.value.trim(),
        api_key: apiKey.value.trim() || undefined,
        files_dir: filesDir.value.trim() || undefined,
        search_api_key: searchKey.value.trim() || undefined,
        search_provider: searchProvider.value,
        search_engine_id: searchEngineId.value.trim() || undefined,
      },
);

/** Everything Save would write except the secrets (those count as changes when typed). */
function snapshot() {
  const { api_key: _k, search_api_key: _s, ...rest } = choice.value as ModelChoice & Record<string, unknown>;
  return JSON.stringify(rest);
}
const dirty = computed(() => snapshot() !== saved.value || apiKey.value.trim() !== "" || searchKey.value.trim() !== "");

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
    searchProvider.value = s.search.provider in SEARCH_CHOICES || s.search.provider === "searxng" ? s.search.provider : "duckduckgo";
    searchEngineId.value = s.search.engine_id ?? "";
    apiKey.value = "";
    searchKey.value = "";
    saved.value = snapshot();
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

/** Wait until a NEW server process answers: the old one can still reply for a moment after
 *  the restart request, and reading settings from it shows the state from before the change. */
async function waitForServer(previousBoot: string | undefined) {
  const until = Date.now() + 30_000;
  await new Promise((r) => setTimeout(r, 700));
  while (Date.now() < until) {
    try {
      const h = await api.health();
      if (!previousBoot || (h.boot_id && h.boot_id !== previousBoot)) return true;
    } catch {
      /* down while it restarts */
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  return false;
}

async function currentBoot() {
  try {
    return (await api.health()).boot_id;
  } catch {
    return undefined;
  }
}

async function restart(message: string) {
  busy.value = "restart";
  notice.value = message;
  const before = await currentBoot();
  try {
    await api.setupRestart();
  } catch {
    /* the server may already be going down */
  }
  const back = await waitForServer(before);
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
  const q = takeSetupOutcome();
  if (q) {
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
  <section class="page-narrow setup">
    <header class="page-head">
      <div>
        <p class="eyebrow">Setup // 3 steps</p>
        <h1>Get the rookery ready</h1>
        <p class="lead">
          Three answers and your penguins can work. Everything lands in
          <code>{{ status?.write_path ?? "config.toml" }}</code>; you can edit it by hand any time.
        </p>
      </div>
    </header>

    <p v-if="notice" class="note note-info" role="status">{{ notice }}</p>
    <p v-if="err" class="note note-err" role="alert">{{ err }}</p>

    <ol v-if="status" class="steps">
      <li class="step card">
        <div class="step-head">
          <span class="n" aria-hidden="true">1</span>
          <div>
            <p class="eyebrow plain">Step 1 of 3 · Model</p>
            <h2>Which brain?</h2>
          </div>
        </div>
        <div class="choices" role="radiogroup" aria-label="Model provider">
          <label class="choice" :class="{ on: provider === 'ollama' }">
            <input v-model="provider" type="radio" value="ollama" />
            <span class="choice-text">
              <b>Local, free</b> — Ollama on this machine
              <small>
                <span class="status-dot" :class="status.ollama.reachable ? 'dot-ok' : 'dot-off'" aria-hidden="true"></span>
                {{ status.ollama.reachable ? `running at ${status.ollama.url}, ${status.ollama.models.length} model(s)` : "not running (install from ollama.com)" }}
              </small>
            </span>
          </label>
          <label class="choice" :class="{ on: provider === 'openai' }">
            <input v-model="provider" type="radio" value="openai" />
            <span class="choice-text">
              <b>Hosted</b> — any OpenAI-compatible endpoint, your own key
              <small>stronger answers; the provider bills you per use</small>
            </span>
          </label>
        </div>

        <div v-if="provider === 'ollama'" class="fields">
          <label class="field">
            <span>Model</span>
            <select v-if="status.ollama.models.length" v-model="ollamaModel">
              <option v-for="m in status.ollama.models" :key="m" :value="m">{{ m }}</option>
            </select>
            <input v-else v-model="ollamaModel" placeholder="llama3.2" spellcheck="false" />
          </label>
          <p v-if="!status.ollama.models.length" class="muted small">No models yet: <code>ollama pull {{ ollamaModel || "llama3.2" }}</code></p>
        </div>

        <div v-else class="fields">
          <label class="field">
            <span>Provider</span>
            <select v-model="preset" @change="applyPreset">
              <option v-for="(p, k) in PRESETS" :key="k" :value="k">{{ p.label }}</option>
            </select>
          </label>
          <label class="field"><span>Endpoint</span><input v-model="baseUrl" placeholder="https://…/v1" spellcheck="false" /></label>
          <label class="field"><span>Model</span><input v-model="hostedModel" spellcheck="false" /></label>
          <label class="field">
            <span>API key</span>
            <SecretInput v-model="apiKey" label="API key" :placeholder="status.has_api_key ? 'saved; leave empty to keep it' : 'sk-…'" />
          </label>
          <p class="muted small">The key is stored in your config file with owner-only permissions (or set <code>OPENAI_API_KEY</code> and leave this empty).</p>
        </div>

        <div class="row">
          <button type="button" :disabled="busy !== null" @click="testModel">{{ busy === "check" ? "Checking…" : "Check the model" }}</button>
          <span v-if="check" class="check-result" :class="check.ok ? 'check-ok' : 'check-err'" role="status">
            <span aria-hidden="true">{{ check.ok ? "✓" : "✕" }}</span> {{ check.message }}
          </span>
        </div>
      </li>

      <li class="step card">
        <div class="step-head">
          <span class="n" aria-hidden="true">2</span>
          <div>
            <p class="eyebrow plain">Step 2 of 3 · Files folder</p>
            <h2>Files and the web</h2>
          </div>
        </div>
        <div class="fields">
          <label class="field"><span>Folder</span><input v-model="filesDir" placeholder="~/Documents/kowalski" spellcheck="false" /></label>
        </div>
        <p class="muted small">Chat's file tool reads and writes only inside this folder. Leave empty to decide per chat.</p>
        <div class="fields two">
          <label class="field">
            <span>Web search</span>
            <select v-model="searchProvider">
              <option v-for="(c, k) in SEARCH_CHOICES" :key="k" :value="k">{{ c.label }}</option>
              <option v-if="searchProvider === 'searxng'" value="searxng">SearXNG (from the config file)</option>
            </select>
          </label>
          <label v-if="searchChoice?.key" class="field">
            <span>{{ searchChoice.key }}</span>
            <SecretInput
              v-model="searchKey"
              :label="searchChoice.key"
              :placeholder="status.search.has_key && status.search.provider === searchProvider ? 'saved; leave empty to keep it' : 'paste the key'"
            />
          </label>
        </div>
        <div v-if="searchProvider === 'google'" class="fields">
          <label class="field"><span>Search engine ID (cx)</span><input v-model="searchEngineId" placeholder="from programmablesearchengine.google.com" spellcheck="false" /></label>
        </div>
        <p class="muted small">
          Agents can always read a web page you give them; this is what they search with.
          <template v-if="searchChoice">
            {{ searchChoice.note }}
            <a v-if="searchChoice.link" :href="searchChoice.link" target="_blank" rel="noopener">Get a key</a>
          </template>
        </p>
      </li>

      <li class="step card">
        <div class="step-head">
          <span class="n" :class="{ done: status.tableski.connected }" aria-hidden="true">{{ status.tableski.connected ? "✓" : "3" }}</span>
          <div>
            <p class="eyebrow plain">Step 3 of 3 · tableski (optional)</p>
            <h2>Spreadsheets as SQL</h2>
          </div>
        </div>
        <p v-if="status.tableski.connected" class="note note-ok">
          <strong>Connected</strong>{{ status.tableski.signed_in ? " (signed in)" : "" }}: {{ status.tableski.url }}.
          Saved already; nothing else to press.
        </p>
        <p v-else>Sign in to tableski.io and your agents can query your uploaded spreadsheets with SQL. Free plan, no card.</p>
        <div class="row">
          <button v-if="!status.tableski.connected" type="button" :disabled="busy !== null" @click="connectTableski">{{ busy === "tableski" ? "Opening tableski…" : "Connect tableski" }}</button>
          <button v-else type="button" class="danger" :disabled="busy !== null" @click="disconnectTableski">Disconnect</button>
        </div>
      </li>
    </ol>
    <p v-else-if="!err" class="muted">Loading your current settings…</p>

    <div v-if="status" class="save">
      <p class="muted small">
        {{ dirty ? "Saving restarts kowalski so every agent picks up the new settings." : "No changes to save. Connecting tableski saves by itself." }}
      </p>
      <p v-if="err" class="note note-err save-err" role="alert">{{ err }}</p>
      <button type="button" class="primary" :disabled="busy !== null || !dirty" @click="saveAll">{{ busy === "save" || busy === "restart" ? "Working…" : dirty ? "Save changes and restart" : "Saved" }}</button>
    </div>
  </section>
</template>

<style scoped>
.steps { list-style: none; padding: 0; margin: 0 0 1.25rem; display: grid; gap: 1rem; }
.step { padding: 1.25rem 1.35rem; }
.step-head { display: flex; align-items: center; gap: 0.9rem; margin-bottom: 1rem; }
.step-head h2 { margin: 0; font-size: 1.25rem; }
.step-head .eyebrow { margin: 0 0 0.15rem; }
.n {
  width: 2.4rem;
  height: 2.4rem;
  flex: 0 0 auto;
  display: inline-grid;
  place-items: center;
  border-radius: var(--radius-sm);
  background: var(--ink);
  color: var(--paper);
  font-family: var(--font-display);
  font-weight: 800;
  font-size: 1.15rem;
}
.n.done { background: var(--ok); color: var(--surface); }
.choices { display: grid; gap: 0.5rem; margin-bottom: 1rem; }
.choice {
  display: flex;
  gap: 0.7rem;
  align-items: flex-start;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 0.75rem 0.9rem;
  cursor: pointer;
  background: var(--surface);
}
.choice:hover { border-color: var(--muted); }
.choice.on { border-color: var(--ink); box-shadow: inset 4px 0 0 var(--red); }
.choice input { margin-top: 0.25rem; }
.choice-text small { display: flex; align-items: center; gap: 0.4rem; color: var(--muted); margin-top: 0.2rem; font-size: 0.85rem; }
.status-dot { width: 0.5rem; height: 0.5rem; border-radius: 50%; flex: 0 0 auto; }
.dot-ok { background: var(--ok); }
.dot-off { background: var(--muted); }
.fields { display: grid; gap: 0.8rem; margin-bottom: 0.4rem; }
.fields.two { grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr); margin-top: 1rem; }
.row { display: flex; gap: 0.8rem; align-items: center; flex-wrap: wrap; margin-top: 0.8rem; }
.check-result { font-weight: 500; padding: 0.35rem 0.6rem; border-radius: var(--radius-sm); }
.check-ok { color: var(--ok); background: var(--surface); border: 1px solid var(--ok); }
.check-err { color: var(--red-ink); background: var(--red-soft); border: 1px solid var(--red); }
.save {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
  padding-top: 1rem;
  border-top: 2px solid var(--ink);
}
.save p { margin: 0; }
.save .save-err { flex-basis: 100%; }
.save .primary { padding: 0.7rem 1.5rem; font-size: 1rem; }
@media (max-width: 720px) {
  .fields.two { grid-template-columns: 1fr; }
}
</style>
