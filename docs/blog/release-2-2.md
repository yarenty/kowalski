<!-- post: date=2026-09-26; slug=kowalski-2-2; summary=Kowalski 2.2 is out: a readable new look, five built-in hordes, download-and-run binaries for macOS and Linux, and defaults that ask before anything runs a command. -->
# Kowalski 2.2: download it, answer three questions, send in a horde

*September 2026*

Three releases went out this week, and together they finish the job we set ourselves in the
summer: kowalski should be something you download and use, not something you build.

## 2.0 — Out of the Box

One binary with the whole app inside. Start `kowalski`, a browser opens, and a Setup screen asks
three questions: which AI (a free local model through Ollama, or any OpenAI-compatible service with
your key), which folder the agents may work in, and, if you want spreadsheets, your
[tableski](https://tableski.io) account. It ships with the **Spreadsheet analyst**, which turns
plain questions into a report workbook without letting the AI compute a single number.

## 2.1 — Standing Orders

- **Download and run.** Pre-built binaries for macOS (Apple Silicon and Intel) and Linux (x86_64
  and ARM). The installer fetches the right one and checks it.
- **Two hordes that work while you do something else.** The **Morning brief** reads the pages you
  follow and writes one page, at 7:00 on weekdays if you switch it on. The **Folder watcher** turns
  each document you drop into its inbox into a note with the facts and the deadlines.
- **Safe by default.** Steps that run a command or change files stop and ask. Workflows imported
  from elsewhere run isolated, with their schedules off. A kowalski reachable from your network
  always asks for its token.

## 2.2 — Commando

A new look you can actually read: penguin black, snow white and one signal red, in light and dark.
The screens are named for what you do (**Hordes**, **Chat**, **Build**, **Setup**), and the
Hordes screen shows each run as a row of steps, then puts the answers on the page when it is done.
The docs got the same treatment, starting with a new [architecture page](../architecture.html).

![The Spreadsheet analyst answering three questions](../img/kowalski-demo.gif)

## Get it

```bash
curl -fsSL https://raw.githubusercontent.com/yarenty/kowalski/main/install.sh | bash
kowalski
```

Everything is open source under MIT at
[github.com/yarenty/kowalski](https://github.com/yarenty/kowalski); the full list of changes is
in the [changelog](https://github.com/yarenty/kowalski/blob/main/CHANGELOG.md). If you want the
long version of why the numbers can be trusted, read
[Ask your spreadsheets](ask-your-spreadsheets.md).
