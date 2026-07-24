# Demo Plan — Kowalski as the Answer to “App Building Is Too Expensive”

> Audience: your boss + Yves (RM). Goal: get the green light (and budget) to build the **plain-language app/agent builder** on top of the Kowalski engine you already have.
>
> Format: ~25–30 minutes, live + slides + code peek. Heavy on **showing**, light on slides.
>
> Core narrative arc you want them to feel:
> **Problem** (apps cost €20–50k each) → **Insight** (apps = orchestrated agents) → **Proof** (Kowalski already runs a real multi-agent “horde” end-to-end) → **Vision** (operators describe an app in plain language, Kowalski builds the horde) → **Ask** (fund the builder layer).

---

## 0. Pre-flight checklist (do this 30 min before the meeting)

- [ ] `cargo run -p kowalski -c config.toml` running on `127.0.0.1:3456`
- [ ] `cd ui && bun run dev` running on `5173`
- [ ] Ollama up, default model warm (run one chat to pre-load)
- [ ] Knowledge Compiler workers started (UI → Federation → **Start All**) and showing READY
- [ ] One **clean dry run** of the horde with a known-good URL (e.g. `https://yarenty.com`) so you’re sure the live demo will finish in <90 s
- [ ] Editor open at `examples/knowledge-compiler/` with `horde.md` + `agents/ingest.md` already in tabs
- [ ] Browser tabs pre-opened: Lovable, v0.dev, Retool, n8n editor, LangFlow, Figma file with mockups, your slides
- [ ] Close Slack, mail, notifications. One desktop, one browser window.
- [ ] Have `PASTE_ME.md` from the previous run open in a second window as a fallback if live run hangs

---

## 1. Opening — frame the pain in their language (2 min)  *(slide-driven)*

Show **Slide 1 — Title** then **Slide 2 — The Real Problem**.

Say (out loud, roughly):

> “Yves told you the single biggest blocker isn’t the model, isn’t the context window, isn’t the hardware. It’s that **building one useful AI app costs €20–50k of human effort**: data prep, connectors, workflow design, UI, prompt engineering, testing. Customers do that math and don’t renew.
>
> What I want to show you today is that **we already have the engine** to make that cost collapse — and a concrete path to the builder layer Yves asked for. Then I’ll show you what the Figma click-dummy would look like.”

**Impact trick:** put a single number on slide 2: **€20,000–50,000 per use case → €0 in operator hours**. Don’t over-explain it; it lands harder if you let it sit for 3 seconds.

---

## 2. Who I am / why I can build this (2–3 min)  *(slide-driven, this is your part)*

Show **Slide 3 — Background**.

- 1–2 short bullets per relevant project. Pictures, not prose.
- End with: “Kowalski is what I’ve been building in Rust for the last ~year as the substrate for exactly this problem.”
- Keep it under 3 minutes. They’re here for the demo, not your CV.

> 👉 **You fill this slide in** — I’ve left the structure and one-line guidance for each bullet in `demo_slides.md`.

---

## 3. Meet Kowalski — the engine (3 min)  *(slide + UI tour)*

Show **Slide 4 — What Kowalski Is Today**.

Then switch to the **live UI** at `http://localhost:5173`:

1. Click through **Home → Chat → MCP → Federation → Horde → Graph** sidebar tabs. ~10 seconds per tab. You’re showing **breadth**, not depth.
2. Land on the **Horde** tab and pause on **“Knowledge Sucking Swarm”**.

Say:
> “This is a Rust workspace: a core agent runtime, an HTTP API, a CLI, a Vue operator UI, and pluggable MCP tools. **Ollama-friendly, on-prem-friendly, no SaaS lock-in.** This matters for on-prem enterprise customers.”

---

## 4. What is a “Horde”? Orchestration & Federation (3 min)  *(slide-driven, then UI)*

Show **Slide 5 — Horde = Orchestrated Team of Agents** and **Slide 6 — Federation Model**.

Use the company-team analogy you already pitched in your reply:

> “A horde is a **team**. Each agent has a **job description** (capability), **tools**, and a **prompt**. The orchestrator is the **team lead**: it knows the dependency order between tasks and dispatches them. Workers can run on different machines — that’s the federation part.”

Visual: pipeline diagram on slide showing
**ingest → compile → ask → lint** with arrows + worker icons.

Key sentence to plant for later:
> “Today the **team** is defined by hand in markdown. Tomorrow we let the **operator describe the team in plain language** and Kowalski writes the markdown.”

---

## 5. **LIVE DEMO** — Knowledge Compiler horde end-to-end (5–6 min)  *(this is the hero moment)*

Switch to the UI **Horde** tab.

1. Confirm horde **Knowledge Sucking Swarm** is selected.
2. Paste a URL. Use one Yves cares about — pick a current AI/RM-relevant article or a docs page. Backup: `https://yarenty.com`.
3. Question field: leave the default, or type: *“What are the key ideas and how would I apply them?”*
4. Click **Run Horde**.

While it runs, **narrate every step** as the federation event stream updates:

- “**Ingest** agent just fetched and normalized the page.”
- “**Compile** agent is digesting it — extracting concepts, claims, sources.”
- “**Ask** agent is now answering the operator question against the digest.”
- “**Lint** agent is producing the final paste-ready markdown.”

When it finishes, click **Open output folder** and open `PASTE_ME.md`. Read 2–3 lines aloud.

Then say:
> “Four specialized agents collaborated, each in their own process, coordinated by the orchestrator, and produced something **a human would have spent an hour writing**. Now hold this thought — we’re going to look under the hood.”

> 🛟 **Fallback:** if the live run stalls, switch instantly to the pre-baked `PASTE_ME.md` window: *“Here’s the same pipeline from earlier — same artifact.”* Don’t apologize, don’t debug live.

---

## 6. How it’s structured today — the “code share” reveal (4 min)  *(IDE on screen)*

Open the editor on `examples/knowledge-compiler/`. Walk the tree top-down:

```
examples/knowledge-compiler/
├── horde.md          ← the team definition (1 file!)
├── agents/
│   ├── ingest.md     ← job description + capability
│   ├── compile.md
│   ├── ask.md
│   └── lint.md
└── prompts/
    ├── compiler.md   ← the actual prompt body
    ├── query.md
    ├── lint.md
    └── output.md
```

Open files in this exact order, ~30 s each:

1. **`horde.md`** — point at `pipeline = ["ingest", "compile", "ask", "lint"]`.
   > “This is the entire team contract. Order, IDs, where output goes.”
2. **`agents/ingest.md`** — point at `capability`, `default_agent_id`, `output`.
   > “One agent = one markdown file. No code.”
3. **`prompts/compiler.md`** — show it’s ~10 lines of plain English.
   > “This is the brain of one agent. Plain English. A power user can edit this.”

Land the punchline:
> “What you just saw running in production is **8 markdown files**. No Python. No n8n spaghetti. No 200-step workflow editor. **This is the format we want operators to author — but we don’t want them to author it as files. We want them to describe it in chat.**”

---

## 7. Where this is going — the Builder Vision (5 min)  *(slides + Figma click-dummy)*

Show **Slide 7 — From Hand-Authored to Conversational**, **Slide 8 — The Builder Loop**, **Slide 9 — Three Editor Surfaces**.

Walk through the three surfaces of the future builder, in this order (simple → ambitious):

### 7a. Horde Overview Editor (the “org chart”)
A canvas where each agent is a card with: name, capability, tools, prompt preview, output. Drag arrows to define dependencies. Live-bound to `horde.md` + `agents/*.md` underneath.

> **Reference apps to mention/show:** **n8n** (for the canvas metaphor — but say *“without the spaghetti”*), **LangFlow** / **Flowise**, **Dify**.

### 7b. Agent Editor (the “job description” form)
Side panel: name, role description (textarea), tools (multi-select MCP catalog), prompt (markdown editor with live preview), output path. Save → writes `agents/<name>.md`.

> **Reference apps:** **Retool** form builders, **Notion** database side-panels, **OpenAI’s Custom GPT builder** (the chat-to-spec UX is gold).

### 7c. **The killer one — Conversational Horde Builder** (Lovable-style)
Split screen:
- **Left:** chat input. Operator types: *“I need an app that watches our shared mailbox, summarizes invoices, files them in SharePoint, and pings finance on Teams when something is over €10k.”*
- **Right:** Kowalski **streams the horde being built** — agent cards appear, tools get attached, prompts get drafted, dependencies get drawn. Operator can click any card to refine.

> **Reference apps to study and steal from:**
> - **[Lovable.dev](https://lovable.dev)** — chat → web app, with live preview. Closest UX to what you want.
> - **[v0.dev](https://v0.dev)** by Vercel — chat → React UI, iterative refinement.
> - **[bolt.new](https://bolt.new)** by StackBlitz — chat → full-stack app in browser.
> - **[Replit Agent](https://replit.com/ai)** — chat → working app, “build alongside me” model.
> - **[Cursor Composer](https://cursor.com)** / **Claude Artifacts** — for the “stream the thing being built next to the chat” pattern.
> - **OpenAI GPT Builder** — for the “interview the user to extract a spec” pattern.
>
> Land it: *“This UX already exists. Lovable does it for websites. v0 does it for React. **Nobody does it for multi-agent business apps.** That’s the wedge.”*

---

## 8. Why this is realistic — connect vision back to engine (2 min)

Show **Slide 10 — Why We Can Actually Ship This**.

The honest, technical version of “trust me”:

- The **runtime exists** (you just saw it run).
- The **artifact format is already plain markdown** — so the LLM only has to emit markdown, not generate Rust. That’s a 10× easier code-gen problem than Lovable solved.
- **MCP** gives us a growing tool catalog without us writing every connector.
- **On-prem / Ollama** story = no SaaS dependency = fits the self-hosted enterprise buyer profile.

One sentence to leave hanging:
> “We are **not** asking ‘can an LLM write a multi-agent system from scratch?’ We are asking ‘can an LLM fill in 8 short markdown files from a conversation?’ — and the answer is obviously yes.”

---

## 9. The Ask (1 min)  *(slide-driven)*

Show **Slide 11 — What I Need**.

Be specific:
- Funding for **N weeks** to deliver a Figma click-dummy + working prototype of the **Conversational Horde Builder** on top of Kowalski.
- A target customer / use case from RM to **co-design** against (so we don’t build in a vacuum).
- A decision date.

Then stop talking. Let them respond.

---

## 10. Backup material (if they ask…)

Have these one-liner answers ready; do **not** bring them up unprompted:

| If they ask… | Snap answer |
|---|---|
| *“Why Rust?”* | Single-binary, on-prem-friendly, no GC pauses, MCP/HTTP/SSE built in. Matches self-hosted deployment models. |
| *“What about security / RBAC?”* | Federation already isolates workers per process. RBAC + policy is on the v03 architecture (`docs/architecture_v03_future.md`). |
| *“How is this not just n8n + LLM nodes?”* | n8n forces operators to **think in nodes**. We let them think in **goals**. n8n is the spaghetti they already complained about. |
| *“What about LangChain / CrewAI / AutoGen?”* | Those are libraries for developers. We’re building the **operator surface** on top of an equivalent runtime — and our runtime is on-prem-Rust, not Python-as-a-service. |
| *“What if Lovable adds agents?”* | Then the market just validated the thesis. Our moat is on-prem + MCP + the markdown spec format we control. |
| *“ROI numbers?”* | Today: 1 horde took ~1 day of my time to author. Target with builder: **<1 hour by a power user**. That’s the €20–50k → ~€500 collapse Yves described. |

---

## 11. Timing summary

| Section | Min |
|---|---|
| 1. Open / pain | 2 |
| 2. About me | 3 |
| 3. Kowalski tour | 3 |
| 4. Horde concept | 3 |
| 5. **LIVE horde run** | 6 |
| 6. Code-share reveal | 4 |
| 7. Builder vision + references | 5 |
| 8. Why realistic | 2 |
| 9. Ask | 1 |
| Buffer / Q&A | 5+ |
| **Total** | **~30 + Q&A** |

---

## 12. Three rules during the demo

1. **Show, then tell.** Click first, narrate while it runs, summarize after. Never explain a feature without showing it.
2. **One thought per slide.** If a slide needs explanation, it’s too busy.
3. **The hero is the operator, not the tech.** Every time you’re tempted to talk about Rust internals, swap in a sentence about *what an operator can now do that they couldn’t before*.
