# Demo Slides — copy/paste each block into PowerPoint

> One block = one slide. Each block has:
> - **Title**
> - **Body** (what goes on the slide; keep it sparse)
> - **Speaker notes** (what you say)
> - **Visual suggestion** (what picture / mockup / screenshot to drop in)
>
> Suggested deck length: **11 slides + 1 backup**. Aim for ≤10 words per bullet.
>
> Recommended visual identity: dark background, big type, ONE accent color (Kowalski purple or your brand color), monospace for code/CLI snippets, screenshots boxed with a subtle drop shadow.

---

## SLIDE 1 — Title

**Title:**
**Kowalski**
*From “AI is too expensive to use” to apps built in plain language.*

**Subtitle / footer:**
Your name · Date · For: Yves (RM) + leadership

**Visual suggestion:**
- Full-bleed Kowalski logo (`docs/img/kowalski_logo.png` or `kowalski_batman.png` if you want personality).
- Big, bold; resist the urge to add bullets.

---

## SLIDE 2 — The Real Problem

**Body:**
- Customers won’t renew. *Why?*
- It costs **€20,000 – €50,000** to ship **one** useful AI app.
- Data prep · Connectors · Workflows · UI · Prompts · Test · Rebuild
- *“It’s just not worth it.”* — Yves

**Speaker notes:**
Quote Yves directly. Pause for 3 seconds after the price tag. Don’t soften it.

**Visual suggestion:**
- Big number **€20k–€50k** centered, accent color.
- Underneath: small icons in a row (database, plug, flowchart, monitor, brain, bug) all greyed out → suggests effort piling up.
- Optionally a small photo / silhouette of a frustrated knowledge worker on the right.

---

## SLIDE 3 — Background (about you)

**Body:** *(you fill in)*
- Past project 1 — one line, one image
- Past project 2 — one line, one image
- Past project 3 — one line, one image
- → **Kowalski**: ~1 year of focused work in Rust on multi-agent infra.

**Speaker notes:**
Pick projects that prove **distributed systems** + **data** + **shipping**. Not a CV dump.

**Visual suggestion:**
- 4-tile grid, each tile: small thumbnail + 1 line caption.
- Last tile = Kowalski logo, slightly larger, accented.

---

## SLIDE 4 — What Kowalski Is Today

**Body:**
- Rust workspace · on-prem-friendly · Ollama + OpenAI APIs
- **Engine:** `kowalski-core` (agents, memory, MCP client)
- **API:** `kowalski` (`/api/*` HTTP server)
- **CLI:** `kowalski-cli` (operators, agent-app)
- **UI:** Vue 3 operator shell (chat · horde · federation · graph)
- **Tools:** MCP (in-repo + external catalogs)

**Speaker notes:**
Don’t read it. Say: *“It’s a complete stack. Backend, API, CLI, UI, pluggable tools. All Rust, all on-prem-friendly.”*

**Visual suggestion:**
- Drop in `docs/img/architecture_v_1_2_0.png` directly — it already shows the layered architecture.
- If too busy, use `docs/img/kowalski_modules_med.jpeg`.
- Add one line on top in your accent color: **“Already runs in production-grade form today.”**

---

## SLIDE 5 — A Horde = a Team of Agents

**Body:**
- A **horde** is a small team that ships one outcome.
- Each agent has: **role** · **tools** · **prompt** · **output**.
- An **orchestrator** knows the dependencies and dispatches the work.
- Workers can run on different machines → **federation**.

**Visual suggestion (build this in PPT, takes 5 min):**
- 4 “person” icons in a row, each with a label: **Ingest · Compile · Ask · Lint**.
- Arrows left → right between them.
- Above them: a single “team lead” icon labeled **Orchestrator**, with thin lines down to each worker.
- Below them: a stack icon labeled **Tools (MCP)**.
- Caption: *“Like a small project team — but it never sleeps.”*

---

## SLIDE 6 — Federation in One Picture

**Body:**
- Operator → API → Orchestrator → many Workers
- Workers can be **on different boxes**, **different tenants**, even **different orgs**.
- Single event stream over SSE — operator sees every step live.

**Visual suggestion:**
- Diagram: laptop (Operator UI) → cloud/box labeled **`kowalski` API** → branching arrows to 3–4 boxes labeled **Worker (ingest)**, **Worker (compile)**, etc.
- Tag two of the workers with a lock icon and a different color to imply **on-prem isolation**.
- Inspiration to crib: **Temporal.io**’s worker diagrams, **Apache Airflow** worker pool diagrams.

---

## SLIDE 7 — LIVE DEMO marker

**Body (huge, centered):**
**▶ Live demo: Knowledge Sucking Swarm**

Sub-line: *“4 agents · 1 URL in · 1 paste-ready note out.”*

**Speaker notes:**
This is just a marker so you stop talking and *show*. Switch to the browser **immediately**.

**Visual suggestion:**
- Black slide. One play-button glyph. Nothing else.
- (Trick: this slide stops you from over-talking.)

---

## SLIDE 8 — What You Just Saw, Under the Hood

**Body:**
- The whole app = **8 markdown files**.
- 1× `horde.md` (the team contract)
- 4× `agents/*.md` (job descriptions)
- 3× `prompts/*.md` (the brains)
- **Zero Python. Zero workflow YAML soup.**

**Visual suggestion:**
- Left half: the file tree (monospace, syntax-highlighted screenshot from your editor).
- Right half: a single quote in big type:
  *“If a power user can edit a Notion page, they can edit a horde.”*

---

## SLIDE 9 — From Hand-Authored to Conversational

**Body (two columns):**

| **Today** | **Tomorrow** |
|---|---|
| Engineer writes 8 markdown files | Operator types one paragraph |
| 1 day of expert time | 5 minutes of conversation |
| ~€2,000 in labor | ~€10 in tokens |
| Locked to people who know the format | Anyone in the org can do it |

**Speaker notes:**
This is the slide that makes the ROI math obvious without needing to spell it out.

**Visual suggestion:**
- Left column greyed-out (as if archived). Right column in accent color.
- Animated arrow `→` between the two columns if you want a small flourish.

---

## SLIDE 10 — Three Editor Surfaces (the product)

**Body:**
1. **Horde Overview** — see the team on a canvas. *(like n8n, minus the spaghetti)*
2. **Agent Editor** — fill in a job description. *(like a Notion side panel)*
3. **Conversational Builder** — describe the app in chat, watch it appear. *(like Lovable, for agents)*

**Speaker notes:**
List them in order of ambition. Each one is independently shippable. The third is the moat.

**Visual suggestion (this is the slide to invest the most design time in):**
- Three mockup tiles side-by-side, each a screenshot of a Figma frame.
- See the **Mockup specs** section at the bottom of this file for exactly what to draw.

---

## SLIDE 11 — Why We Can Actually Ship This

**Body:**
- ✅ Runtime exists — **you just saw it run.**
- ✅ Output format = **markdown** (LLMs are great at markdown).
- ✅ Tools come from the **MCP ecosystem** — we don’t write every connector.
- ✅ **On-prem / Rust / Ollama** = no SaaS dependency = right fit for self-hosted enterprise buyers.
- ✅ Not generating a full app — just **filling in 8 short files from a conversation**.

**Visual suggestion:**
- 5 green checkmark rows, big type.
- Last row in your accent color, slightly larger.

---

## SLIDE 12 — The Ask

**Body:**
- **Fund:** N weeks of focused build time. *(you fill the number)*
- **Deliver:**
  1. Figma click-dummy of the Conversational Builder
  2. Working prototype on top of Kowalski (live, on a real customer use case)
- **Need from RM:** one real use case, one operator to co-design with.
- **Decision by:** *(date)*

**Visual suggestion:**
- Single column, generous spacing.
- Footer: your contact + repo link.

---

## SLIDE 13 — Backup: Architecture v0.3 (future)

*(only show if they ask “where does this go after the builder?”)*

**Body:**
- Orchestration service · Federation registry · Policy + RBAC · Telemetry
- Reusable workflow engine · Tool bus + MCP adapters
- Episodic + semantic + graph memory
- Managed deployment targets

**Visual suggestion:**
- Drop in `docs/img/architecture_v03-future.excalidraw` (export to PNG).
- Or render the Mermaid diagram from `docs/architecture_v03_future.md`.

---

# 🎨 Mockup specs — for Slide 10 (Figma click-dummy)

This is what Yves will pay attention to most. These three screens **are the click-dummy** he asked for. Each is a single Figma frame; together they tell the story.

> Design rule of thumb for all three: **one screen, one job.** Resist the urge to put 12 panels on one screen. Lovable, v0.dev, and ChatGPT custom-GPT builder all win because they look almost empty.

---

## Mockup A — Horde Overview Editor

**One-line job:** “See the whole app at a glance.”

**Layout (top → bottom):**
- **Top bar:** horde name (editable inline), Run / Pause / Share buttons.
- **Main canvas (~80% of screen):** 4–6 large agent **cards** placed left-to-right, connected by simple arrows. No webby spaghetti.
  - Each card: icon · agent name · role one-liner · tiny “tools used” chip row · status dot (idle/running/done).
  - Drag arrows between cards = dependencies.
- **Right rail (collapsed by default):** properties of selected agent.
- **Bottom strip:** event stream (auto-scrolling timeline of `task_started` / `task_finished` events from `/api/federation/stream`).

**Feel:** **Linear**-clean, **Miro**-spatial, **n8n**-without-the-noise.

**Reference apps to imitate:**
- [LangFlow](https://www.langflow.org/) — agent-flow canvas, but cleaner.
- [Flowise](https://flowiseai.com/) — same space.
- [Dify](https://dify.ai/) — node-based orchestration, decent visuals.
- [Retool Workflows](https://retool.com/products/workflows) — for the side-by-side panel + canvas.
- [Temporal Web UI](https://docs.temporal.io/web-ui) — for the run timeline.

---

## Mockup B — Agent Editor (the “job description” form)

**One-line job:** “Configure one agent without writing code.”

**Layout (left → right, two columns):**
- **Left (form):**
  - Name
  - Role description (textarea, “What does this agent do?”)
  - Tools: multi-select chip picker, sourced from the MCP catalog
  - Prompt: markdown editor with a **“Improve with AI”** button
  - Inputs / Outputs (file paths, with `@artifact@` autocomplete)
- **Right (preview):**
  - Live-rendered prompt
  - Last 3 runs of this agent: input → output diff
  - **“Test this agent”** button → runs against a sample input

**Feel:** **Notion** database side-panel + **Postman** request builder + **OpenAI GPT Builder**.

**Reference apps to imitate:**
- [OpenAI’s Custom GPT Builder](https://chat.openai.com/gpts/editor) — left = chat, right = preview. Steal this layout almost verbatim.
- [Anthropic Console](https://console.anthropic.com/) prompt workbench.
- [PromptLayer](https://www.promptlayer.com/) / [Humanloop](https://humanloop.com/) — for the version-and-test side.
- [Retool](https://retool.com/) form components.

---

## Mockup C — Conversational Horde Builder ⭐ (the killer)

**One-line job:** “Describe your app, watch it build itself.”

**Layout (split 40 / 60):**
- **Left 40% — Chat:**
  - System greeting: *“Hi! Tell me what your app should do. Examples: ‘summarize new invoices in our shared mailbox’ or ‘every Monday compile last week’s competitor news.’”*
  - User types → assistant asks 1–2 clarifying questions, then proposes a horde.
  - Each assistant turn includes inline **action chips** like *“Add a Slack-notification step”*, *“Use SharePoint instead of S3”*.
- **Right 60% — Live Horde Canvas:**
  - Same canvas as Mockup A — but **agent cards animate in** as the assistant speaks (use `[ghost card] → [solid card]` transition).
  - When operator clicks any card, the chat focuses on that agent: *“Want me to change anything about the Compile step?”*
  - Top-right: **▶ Run with sample data** button, always visible.
- **Top bar status:** small progress indicator: `Drafting → Reviewing → Ready to run`.

**Killer micro-interactions to specify in Figma:**
1. Agent cards **slide in** from the right as the assistant proposes them. Don’t pop — slide.
2. Tools attach to a card with a small **magnetic snap** animation.
3. The “Run with sample data” button **pulses gently** once the horde is valid.
4. When something is missing (e.g. *“you didn’t pick a destination for the summary”*), the relevant card gets a subtle red dot — not a modal, not an alert.

**Feel:** **Lovable.dev** + **OpenAI GPT Builder** + a tiny dash of **Linear**.

**Reference apps to imitate (these are gold — open all of them, do screen recordings, study them):**
- [**Lovable.dev**](https://lovable.dev) — *the* reference for chat-builds-the-thing-live UX. Watch their landing-page demo video frame by frame.
- [**v0.dev**](https://v0.dev) — chat → React; pay attention to the iterative refinement loop.
- [**bolt.new**](https://bolt.new) — chat → full-stack app; shows preview + file tree + chat in three panes.
- [**Replit Agent**](https://replit.com/ai) — the “build alongside me” narration is exactly the tone you want.
- [**Cursor Composer**](https://cursor.com) and **Claude Artifacts** — for streaming a complex artifact next to the chat.
- [**OpenAI GPT Builder**](https://chat.openai.com/gpts/editor) — the “interview the user, then propose a config” flow.
- [**Vercel AI SDK demos**](https://chat.vercel.ai/) — for the streaming UI patterns (use them as React inspiration).
- [**Pika.style**](https://pika.style/) / [**Tiptap**](https://tiptap.dev/) — for inline editable cards / chips.

---

# Designer brief (give this to whoever does the Figma)

Single paragraph the designer can paste into their tool of choice:

> *“Build three Figma frames for an internal product called Kowalski. Mood: Linear + Notion + Lovable. Dark mode preferred. Frame A: a horde-overview canvas with 4–5 agent cards connected by arrows, a top bar with horde name and Run button, and a collapsible right side panel. Frame B: an agent-editor with a left form (name, role, tools chips, markdown prompt) and a right live-preview with a Test button. Frame C: the hero — a 40/60 split with a chat on the left and the same horde canvas on the right; cards animate in as the assistant proposes them. Use placeholder copy like ‘Summarize invoices arriving in the shared mailbox and notify finance on Teams.’ All frames must look ‘almost empty’ — sparse, generous whitespace, one accent color. Reference: Lovable.dev, v0.dev, OpenAI Custom GPT builder.”*

---

# Pictures already in the repo you can reuse on slides

| File | Use on slide |
|---|---|
| `docs/img/kowalski_logo.png` | Slide 1 (title) |
| `docs/img/architecture_v_1_2_0.png` | Slide 4 (engine today) |
| `docs/img/kowalski_modules_med.jpeg` | Slide 4 alt |
| `docs/img/architecture_v03-future.excalidraw` (export PNG) | Slide 13 backup |
| `docs/img/high_performance_agentic_ai.png` | Optional intro / cover |
| `docs/img/pluggable.png` | Could illustrate MCP / tools |
| `docs/img/memory_3tiers.png` | Optional Q&A backup on memory architecture |

Take screenshots fresh on demo day for:
- The UI **Horde** tab with “Knowledge Sucking Swarm” selected.
- The federation event stream mid-run (lots of green ticks looks great).
- A `PASTE_ME.md` opened in your editor with the rendered markdown preview.
- The file tree of `examples/knowledge-compiler/`.

---

# Final coaching notes

1. **Open with their pain, not your solution.** Slide 2 is the most important slide in the deck.
2. **Demo before architecture.** They’ll forgive a confusing diagram if they’ve already seen the thing work.
3. **The Figma click-dummy is the deliverable Yves is buying.** Treat slide 10 + the three mockups as the centerpiece of the whole deck.
4. **Silence after the ask.** Don’t fill it. Whoever speaks first loses.
