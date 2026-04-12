//! `kowalski-cli run` — interactive orchestrator REPL (chat + federation hints).

use kowalski_core::agent::Agent;
use kowalski_core::template::agent::TemplateAgent;
use rustyline::DefaultEditor;
use std::io::{self, Write};

/// Multi-line aware REPL: loads config, one `TemplateAgent`, then `chat_with_tools` per input.
pub async fn run_orchestrator(config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let path = crate::ops::mcp_config_path(config_path);
    let cfg = crate::ops::load_kowalski_config_for_serve(&path)?;
    kowalski_core::db::run_memory_migrations_if_configured(&cfg).await?;

    let mut agent = TemplateAgent::new(cfg.clone()).await?;
    let model = cfg.ollama.model.clone();
    let mut conv_id = agent.start_conversation(&model);

    println!(
        "Kowalski orchestrator REPL — model `{}` · session `{}`",
        model, conv_id
    );
    println!("Commands: /bye exit · /new new session · lines ending with \\ continue");
    println!("Lines are prefixed with [agent] (LLM) and [tool] (tool round) for readability.");
    println!("Federation: use `serve` + Vue or `curl` to /api/federation/* (HTTP + optional Postgres NOTIFY).");

    let mut rl = DefaultEditor::new()?;
    let mut pending = String::new();

    loop {
        let prompt = if pending.is_empty() {
            "kowalski-run> ".to_string()
        } else {
            "...> ".to_string()
        };
        let line = match rl.readline(&prompt) {
            Ok(l) => l,
            Err(_) => break,
        };
        let _ = rl.add_history_entry(line.as_str());

        let chunk = line.trim_end();
        if chunk.ends_with('\\') {
            pending.push_str(chunk.trim_end_matches('\\'));
            pending.push('\n');
            continue;
        }
        pending.push_str(chunk);
        let input = std::mem::take(&mut pending);
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input.eq_ignore_ascii_case("/bye") || input.eq_ignore_ascii_case("/exit") {
            break;
        }
        if input.eq_ignore_ascii_case("/new") {
            conv_id = agent.start_conversation(&model);
            println!("New session: {}", conv_id);
            continue;
        }

        {
            let _repl = kowalski_core::agent::repl_trace::ReplTraceGuard::enable();
            if let Err(e) = agent.chat_with_tools(&conv_id, input).await {
                eprintln!("error: {}", e);
            }
        }
        // `chat_with_tools` prints each LLM turn (and `[agent]`/`[tool]` when trace is on); no extra println.
        let _ = io::stdout().flush();
    }

    println!("Goodbye.");
    Ok(())
}
