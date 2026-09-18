# Switchyard task runner. `just` lists everything; recipes run from the repo root.

set windows-shell := ["pwsh", "-NoLogo", "-Command"]

# List available recipes
default:
    @just --list --unsorted

# --- run ------------------------------------------------------------------------------------

# Install dependencies (JS packages + Rust crates)
setup:
    bun install
    cargo fetch

# Run the app with hot reload
dev:
    bun tauri dev

# Run the app forcing a terminal renderer: webgl or dom
dev-renderer $SWITCHYARD_RENDERER:
    bun tauri dev

# Run the frontend alone in a browser tab (no Rust core; terminals won't work)
web:
    bun run dev

# --- check ----------------------------------------------------------------------------------

# Everything CI checks: formatting, lints, types, all tests, stale bindings
check: fmt-check lint typecheck test bindings-check

# All tests, Rust and frontend
test: test-rust test-web

# Rust tests (also regenerates src/lib/bindings.ts)
test-rust *args:
    cargo test --workspace {{ args }}

# Frontend tests
test-web *args:
    bun run test {{ args }}

# Frontend tests in watch mode
test-watch:
    bun run test:watch

# Lint Rust (clippy, warnings are errors) and the frontend (eslint)
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    bun run lint

# Clippy for the Windows target, from any OS (needs: rustup target add x86_64-pc-windows-msvc)
lint-windows:
    bun run lint:windows

# Typecheck the frontend
typecheck:
    bun run typecheck

# Format everything
fmt:
    cargo fmt --all
    bun run format

# Check formatting without changing files
fmt-check:
    cargo fmt --all -- --check
    bun run format:check

# Regenerate TypeScript bindings from the Rust commands
bindings:
    bun run bindings

# Fail if the checked-in bindings are stale
bindings-check: bindings
    git diff --exit-code -- src/lib/bindings.ts

# --- build ----------------------------------------------------------------------------------

# Build installers for this OS (target/release/bundle/)
build:
    bun tauri build

# Build only the given bundle types, e.g. `just bundle deb` or `just bundle appimage,deb`
bundle types:
    bun tauri build --bundles {{ types }}

# Debug build of the app, no installers
build-debug:
    bun tauri build --debug --no-bundle

# --- measure --------------------------------------------------------------------------------

# Terminal rendering benchmarks (see docs/07-terminal-benchmarks.md)
[unix]
bench:
    scripts/bench/run.sh

# --- ci & housekeeping ----------------------------------------------------------------------

# Latest CI runs
ci:
    gh run list --limit 5

# Follow the most recent CI run until it finishes
ci-watch:
    gh run watch "$(gh run list --limit 1 --json databaseId -q '.[0].databaseId')" --exit-status

# Download installers from the latest successful CI run on main into ./artifacts
ci-artifacts:
    gh run download "$(gh run list --branch main --status success --limit 1 --json databaseId -q '.[0].databaseId')" --dir artifacts

# Remove build output (Rust target, frontend dist, downloaded artifacts)
[unix]
clean:
    cargo clean
    rm -rf dist artifacts

# Remove build output (Rust target, frontend dist, downloaded artifacts)
[windows]
clean:
    cargo clean
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue dist, artifacts
