# Streamable HTTP MCP (JSON + optional SSE) for DataFusion.
# Build from the repository root:
#   docker build -f kowalski-mcp-datafusion/Dockerfile -t kowalski-mcp-datafusion .
# Run:
#   docker run --rm -p 8080:8080 kowalski-mcp-datafusion
# Point Kowalski at: url = "http://127.0.0.1:8080/" , transport = "http" or "sse"

FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml ./
COPY kowalski ./kowalski
COPY kowalski-core ./kowalski-core
COPY kowalski-cli ./kowalski-cli
COPY kowalski-mcp-datafusion ./kowalski-mcp-datafusion
RUN cargo build -p kowalski-mcp-datafusion --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/kowalski-mcp-datafusion /usr/local/bin/
COPY kowalski-mcp-datafusion/fixtures/sample.csv /data/sample.csv
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/kowalski-mcp-datafusion"]
CMD ["--bind", "0.0.0.0:8080", "--csv", "/data/sample.csv", "--table", "data"]
