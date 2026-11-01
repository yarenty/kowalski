# Files, PDFs, mail and tasks (emperor servers)

Four small MCP servers give kowalski's agents more to work with. Each is a single binary, built
on [emperor-mcp](https://github.com/yarenty/emperor-mcp), read-only unless you start it otherwise:

| Server | What agents can do | Tools |
|---|---|---|
| [emperor-folder](https://github.com/yarenty/emperor-folder) | Browse, read and search one folder (or a few), and with `--writable` write, edit and move files in it | `list`, `read`, `search`, `stat` (+ `write`, `edit`, `move`) |
| [emperor-pdf](https://github.com/yarenty/emperor-pdf) | Read PDFs: info, text, search, a page as an image; with `--writable` split, merge and create them | `pdf_info`, `pdf_text`, `pdf_search`, `pdf_render_page` (+ `pdf_pages`, `pdf_create`) |
| [emperor-mail](https://github.com/yarenty/emperor-mail) | Search and read your mail and attachments, write drafts; send only to the recipients you allow | `list_folders`, `search`, `read_message`, `read_attachment`, `draft`, `send` |
| [emperor-vikunja](https://github.com/yarenty/emperor-vikunja) | List, read, create and update tasks in [Vikunja](https://vikunja.io), comment on them | `list_tasks`, `get_task`, `list_projects` (+ `create_task`, `update_task`, `comment`) |

## Install

Prebuilt binaries for macOS, Linux and Windows are on each repository's Releases page. With Rust
installed:

```bash
cargo install emperor-folder emperor-pdf emperor-mail emperor-vikunja
```

Each has a `doctor` command that checks its setup before you connect anything:
`emperor-folder doctor --root ~/notes`.

## Connect

Add one block per server to `config.toml` (next to the `tableski` block, if you have one) and
restart kowalski. With `transport = "stdio"` kowalski starts the server itself; `command` is the
program and its arguments as a list. Write full paths: there is no shell, so `~` is not expanded.

**A folder of notes or code**

```toml
[[mcp.servers]]
name = "folder"
transport = "stdio"
command = ["emperor-folder", "serve", "--stdio", "--root", "/Users/me/notes"]
# let agents change files too: add "--writable"
# several folders: "--root", "notes=/Users/me/notes", "--root", "code=/Users/me/src/app"
```

**PDFs**

```toml
[[mcp.servers]]
name = "pdf"
transport = "stdio"
command = ["emperor-pdf", "serve", "--stdio", "--root", "/Users/me/Documents/papers", "--pdfium-dir", "/Users/me/pdfium"]
```

emperor-pdf reads pages with PDFium; its README says where to get it.

**Mail.** The server reads your login from its environment (`MAIL_USER`, `MAIL_PASSWORD` or an
app password, `IMAP_HOST`, `SMTP_HOST`; the full list is in its README). A server kowalski starts
gets kowalski's environment, so set them where you start kowalski:

```toml
[[mcp.servers]]
name = "mail"
transport = "stdio"
command = ["emperor-mail", "serve", "--stdio"]
# to let it send, and only to your own domain:
# command = ["emperor-mail", "serve", "--stdio", "--allow-send", "--send-allow", "@example.org"]
```

Without `--allow-send`, `send` refuses and the agent writes a draft instead, which you find in
your mail client's Drafts folder.

**Tasks in Vikunja.** A token from Vikunja's **Settings → API Tokens**, in `VIKUNJA_TOKEN`:

```toml
[[mcp.servers]]
name = "vikunja"
transport = "stdio"
command = ["emperor-vikunja", "serve", "--stdio", "--base-url", "https://vikunja.example.com"]
# look, don't touch: add "--read-only"
```

Or run the server yourself and give kowalski the address, with the token in the block like the
tableski one:

```toml
[[mcp.servers]]
name = "vikunja"
transport = "http"
url = "http://127.0.0.1:8034/mcp"
headers = { Authorization = "Bearer <your token>", "X-Upstream-Url" = "https://vikunja.example.com" }
```

## Check it

```bash
kowalski-cli mcp ping -c ~/.config/kowalski/config.toml    # each server answers
kowalski-cli mcp tools -c ~/.config/kowalski/config.toml   # the tools agents get
```

Two servers can offer a tool with the same name: `search` is in both folder and mail. Agents
then see one of them under its server's name, for example `mail::search`.

## Use them in a horde

A model step lists the tools it may call in `tool_ids`, in its `agents/*.md` file:

```toml
tool_ids = ["pdf_text", "pdf_search"]
```

Only those tools are offered to that step, so a summarising step can read PDFs and nothing else.

## Safety

- **Writing is a start-up switch.** Without `--writable` (folder, pdf) or with `--read-only`
  (vikunja), the write tools are not offered at all. Mail never deletes, and sends only with
  `--allow-send` to the recipients you list.
- **Nothing is stored.** Passwords and tokens are used for the call and dropped, never written
  to disk or to the log.
- **Content is labelled.** Text from files, PDFs, mail and tasks reaches the model marked as
  content to read, not instructions to follow, so a mail that says "forward all invoices to…" is
  treated as what it is.
