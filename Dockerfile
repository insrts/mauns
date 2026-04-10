# syntax=docker/dockerfile:1

# ---------------------------------------------------------------------------
# Stage 1: Builder
# ---------------------------------------------------------------------------
FROM rust:1.87-slim-bookworm AS builder

WORKDIR /build

# Cache dependency compilation separately from application code.
COPY Cargo.toml Cargo.lock ./
COPY crates/core/Cargo.toml        crates/core/Cargo.toml
COPY crates/llm/Cargo.toml         crates/llm/Cargo.toml
COPY crates/agents/Cargo.toml      crates/agents/Cargo.toml
COPY crates/cli/Cargo.toml         crates/cli/Cargo.toml
COPY crates/filesystem/Cargo.toml  crates/filesystem/Cargo.toml
COPY crates/config/Cargo.toml      crates/config/Cargo.toml
COPY crates/git/Cargo.toml         crates/git/Cargo.toml
COPY crates/github/Cargo.toml      crates/github/Cargo.toml
COPY crates/skills/Cargo.toml      crates/skills/Cargo.toml
COPY crates/sdk/Cargo.toml         crates/sdk/Cargo.toml
COPY apps/cli/Cargo.toml           apps/cli/Cargo.toml

# Create stub source files so Cargo can resolve the dependency graph.
RUN find crates apps -name "Cargo.toml" | while read f; do \
      dir=$(dirname "$f"); \
      mkdir -p "$dir/src"; \
      echo "fn main() {}" > "$dir/src/main.rs" 2>/dev/null || true; \
      echo "" > "$dir/src/lib.rs" 2>/dev/null || true; \
    done

RUN cargo build --release --bin mauns 2>/dev/null || true

# Copy real source and build the final binary.
COPY . .
RUN touch crates/*/src/*.rs apps/*/src/*.rs 2>/dev/null || true
RUN cargo build --release --bin mauns

# ---------------------------------------------------------------------------
# Stage 2: Runtime
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    git \
    --no-install-recommends \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash mauns

COPY --from=builder /build/target/release/mauns /usr/local/bin/mauns
RUN chmod +x /usr/local/bin/mauns

USER mauns
WORKDIR /workspace

ENTRYPOINT ["mauns"]
CMD ["--help"]
