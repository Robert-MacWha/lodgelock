# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Lodgelock** is a modular wallet framework built on an entity-domain-plugin architecture. Plugins run as sandboxed WASM modules and communicate with the host via JSON-RPC over STDIO. The workspace has two frontends (a Dioxus web app and a Ratatui TUI), a host runtime, shared API/SDK crates, and several first-party plugins.

## Commands

### Development

```bash
# Enter the nix dev environment first
nix-shell

# Build a single plugin (debug, fast)
PLUGIN=rpc-provider make plugin

# Build all plugins (debug)
make plugins

# Build all plugins (release, optimized for size) and copy to frontend/public/plugins/ and tui-plugins/
make plugins-release

# Run the web frontend (after building plugins)
cd frontend && dx serve --platform web

# Launch Chrome with security disabled (required for SharedArrayBuffer in local dev)
chrome-unsafe

# Run the TUI
cargo run -p tui

# Format
make fmt

# Lint (errors on warnings)
make lint
```

### Testing

```bash
cargo test -p <crate-name>
cargo test --workspace
```

### Web Deployment (Cloudflare Pages)

```bash
make wrangler-dev    # local preview
make wrangler-deploy # deploy to CF Pages
```

## Architecture

### Workspace Structure

| Path                 | Purpose                                                                        |
| -------------------- | ------------------------------------------------------------------------------ |
| `crates/host`        | Host runtime: loads plugins, routes RPC, manages entities/state/storage        |
| `crates/tlock-api`   | Shared types and domain trait definitions (used by both host and plugins)      |
| `crates/tlock-hdk`   | Host Development Kit: macros for wiring host-side RPC handlers                 |
| `crates/tlock-pdk`   | Plugin Development Kit: `PluginRunner`, typed `StateExt` helpers               |
| `crates/tlock-alloy` | Alloy transport adapter that routes through the host (for plugins using alloy) |
| `crates/erc20s`      | ERC-20 contract bindings                                                       |
| `frontend/`          | Dioxus (WASM) web frontend                                                     |
| `tui/`               | Ratatui terminal UI                                                            |
| `plugins/`           | First-party plugin implementations                                             |

### Core Concepts

**Domains** are abstract interfaces any entity can implement (`Vault`, `EthProvider`, `Coordinator`, `Page`). Defined in `crates/tlock-api/src/domains/`.

**Entities** are domain implementations registered by plugins. A single plugin can register multiple entities across different domains.

**Plugins** are `wasm32-wasip1` binaries. They communicate with the host exclusively via JSON-RPC over STDIO — no direct network or filesystem access. The `tlock-pdk` crate provides the plugin-side runtime.

**Host** (`crates/host`) is the trusted kernel. It manages plugin lifetimes, entity registries, persistent key-value storage, and user-approval flows. It exposes services to plugins via host calls defined in `tlock-api`.

### Plugin Pattern

Every plugin binary follows this pattern using `PluginRunner` from `tlock-pdk`:

```rust
fn main() {
    PluginRunner::new()
        .with_method(global::Ping, ping)
        .with_method(host::Init, init)          // registers entities with the host
        .with_method(eth::BlockNumber, block_number)
        // ... domain methods
        .run();
}
```

- `init` calls `host::RegisterEntity` to declare which domain the plugin implements.
- State is persisted via `transport.state().lock_or(|| default)` / `.read()` / `.write()`.
- See `plugins/rpc-provider` for the canonical complete example.

### Adding a New Plugin

1. Create `plugins/<name>/` as a new workspace member with `[lib]` crate-type `["cdylib"]` or a `[[bin]]`.
2. Implement `main()` with `PluginRunner` — see `plugins/plugin-template` for the skeleton.
3. Implement `init` to call `host::RegisterEntity` with the appropriate `Domain` variant.
4. Implement domain-specific RPC methods matching the signatures in `tlock-api`.

### Build Targets

- **Native** (`cargo build`): used for the host, TUI, and tests.
- **WASM** (`--target wasm32-wasip1`): plugins only, via `make plugin` / `make plugins`.
- **WASM web** (`--target wasm32-unknown-unknown`): Dioxus frontend, built by `dx`.

### Key Constraints

- Plugins cannot access the network or filesystem directly — all I/O goes through host calls.
- `wasm32-wasip1` plugins must be single-threaded; use `?Send` async bounds inside plugins.
- The `--profile release-wasm` profile (size-optimized with `opt-level = "z"`, LTO, `panic = abort`) is required for production plugin builds.
- `wasmi-plugin-hdk` and `wasmi-plugin-pdk` are patched to a local sibling repo (`../wasmi-plugin-framework`).
