FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

# Install additional tools useful for development
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    gdb \
    lldb \
    python3 \
    python3-pip \
    nodejs \
    npm \
    wget \
    curl \
    jq \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Install Claude Code CLI
RUN curl -fsSL https://claude.ai/install.sh | bash

# Install just (command runner)
RUN cargo install just --locked

# Install Rust tools via cargo (using --locked to avoid dependency resolution issues)
RUN cargo install cargo-watch --locked && \
    cargo install cargo-tree --locked || true
# Note: cargo-edit and cargo-audit skipped due to MSRV issues, install manually if needed

# Set up git to trust the workspace directory for safe.directory
RUN git config --global --add safe.directory /workspace

# Set default shell to zsh
SHELL ["/bin/zsh", "-c"]

# Set working directory
WORKDIR /workspace
