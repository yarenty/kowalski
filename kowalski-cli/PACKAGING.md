# Packaging and deployment (operators)

The default binary is intentionally self-contained. Heavier TLS (rustls via `axum-server`) is linked for optional **`serve --tls-cert` / `--tls-key`** HTTPS.

## Static binary

```bash
cargo build -p kowalski-cli --release --features postgres   # optional DB/federation
```

Copy `target/release/kowalski-cli` (or workspace package name if renamed) plus a `config.toml` and optional TLS PEM files.

## systemd (sketch)

- `User=` and `WorkingDirectory=` pointing at config.
- `ExecStart=/usr/local/bin/kowalski-cli serve --bind 127.0.0.1:3456 -c /etc/kowalski/config.toml`
- For TLS: add `--tls-cert` and `--tls-key` paths; or terminate TLS at **nginx** / **Caddy** and proxy to `localhost:3456`.

## Container

- Multi-stage: `cargo build --release` then `FROM debian:bookworm-slim` (or `distroless`) with the binary and `config.toml`.
- Expose the `serve` port; set `memory.database_url` via env or mounted secret if using Postgres.

## UI

Build the Vue app in `../ui` and point **`VITE_API_BASE`** at the public URL of `serve` (see `ui/DEPLOY.md`).
