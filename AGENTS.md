# Repository Guidelines

## Project Overview

**Sticks & Stones** is a Bevy-based game written in Rust. It uses a custom deferred rendering pipeline (rasterize + raymarch + lighting passes), a JSON-driven asset/library system, and SpacetimeDB for multiplayer networking.

---

## Project Structure

```
sticks_and_stones/
├── src/                    # Main binary crate
│   ├── main.rs
│   ├── window.rs
│   ├── ambassadors/        # World-state bridges (environment, baseplates, brickhub)
│   ├── net/                # Networking & SpacetimeDB bindings
│   ├── player/             # Player logic
│   ├── renderer/           # Custom deferred pipeline (raymarch, raster, lighting, post)
│   └── ui/                 # All UI screens (menu, HUD, inventory, chat, etc.)
├── crates/
│   ├── sns_assets/         # JSON asset loading, library manifests, registry
│   ├── sns_core/           # Shared math, structs, and shader path constants
│   ├── sns_macros/         # Procedural macros for the asset/registry system
│   └── sns_world/          # World data: baseplates, brickhub, environment, worldgen
├── assets/
│   ├── data/               # JSON asset libraries (lib.json, schemas, worldgen)
│   ├── shaders/            # WGSL shaders (raymarch, raster, lighting, post, shared)
│   ├── art/                # Aseprite sprites and artwork
│   ├── fonts/
│   └── images/
└── Cargo.toml              # Workspace root
```

---

## Build, Test & Development Commands

```bash
# Run in development (optimised deps, debug binary)
cargo run

# Release build
cargo build --release

# Check all workspace crates for errors without producing a binary
cargo check --workspace

# Run all tests across the workspace
cargo test --workspace

# Lint with Clippy
cargo clippy --workspace -- -D warnings

# Format all code
cargo fmt --all
```

> **SpacetimeDB note:** The server is a separate WASM binary. Copy `spacetime.wasm` from the SpacetimeDB target into `target/debug/` and ensure the SpacetimeDB CLI is up to date before running multiplayer. See `README.md` for full setup steps.

---

## Coding Style & Naming Conventions

- **Formatter:** `rustfmt` — run `cargo fmt --all` before committing. No unformatted code.
- **Linter:** `cargo clippy` — address all warnings; treat `-D warnings` as the bar.
- **Naming:** follow standard Rust conventions — `snake_case` for modules, functions, and variables; `PascalCase` for types and traits; `SCREAMING_SNAKE_CASE` for constants.
- **Modules:** each logical subsystem lives in its own module directory with a `mod.rs`. Keep modules focused; avoid monolithic files.
- **Asset references** use the `namespace:name` string format (e.g. `"lib:stone_block"`).
- **Shader files** are `.wgsl` and live under `assets/shaders/`. Shader path constants are defined in `sns_core`.

---

## Testing Guidelines

- Tests use the built-in Rust test framework (`#[test]`, `#[cfg(test)]`).
- Place unit tests in an inline `mod tests` block at the bottom of the relevant file.
- Integration tests go in a `tests/` directory inside the relevant crate.
- Run the full suite with `cargo test --workspace`.
- There is no minimum coverage requirement enforced by CI at this time, but all new logic in `sns_assets`, `sns_core`, and `sns_world` should have accompanying unit tests.

---

## Commit & Pull Request Guidelines

**Commit messages** observed in this project follow an imperative, sentence-style format:

```
Added <feature or system>
Implemented <thing>
Reworked <subsystem>
Fixed <bug description>
Converted <thing> to use <approach>
```

- Start with a capital letter; no trailing period required.
- Be specific: name the system or file affected (e.g. `Reworked brickmap, deltas, and more`).
- Keep the subject line under 72 characters; add a body for non-obvious changes.

**Pull requests should:**
- Include a short description of what changed and why.
- Reference any related issue or design discussion.
- Pass `cargo check --workspace`, `cargo fmt --all -- --check`, and `cargo clippy --workspace` before review.
- Keep scope focused — one feature or fix per PR.

---

## Architecture Notes

- **Rendering** is a fully custom deferred pipeline. Passes are defined under `src/renderer/` and driven by Bevy's render graph. Do not replace this with Bevy's default PBR pipeline without discussion.
- **Asset system** (`sns_assets`) loads JSON files validated against JSON Schema. Every asset type implements the `StaticAsset` trait. Schemas live alongside the data in `assets/data/`.
- **Workspace crates** (`crates/*`) are pure-Rust libraries with no Bevy dependency where possible. Keep Bevy imports in the main crate or behind a feature flag.
