# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MPSQL is a cross-platform desktop app (Tauri v2) for importing GIS data into PostgreSQL via `ogr2ogr`. It uses micromamba to manage a conda environment containing GDAL, libpq, and related tools — so no system-level GDAL install is required.

## Commands

```bash
# Install frontend dependencies
pnpm install

# Run in dev mode (starts both Vite and Rust backend)
pnpm tauri dev

# Build production app
pnpm tauri build

# Frontend-only (no Tauri)
pnpm dev

# Type-check frontend
pnpm build
```

Rust backend is in `src-tauri/`. To work on it in isolation:
```bash
cd src-tauri
cargo check
cargo build
```

There are no automated tests in this codebase.

## Architecture

### Frontend–Backend Bridge

The frontend calls Rust via `invoke()` from `@tauri-apps/api/core`. All Tauri commands are registered in `src-tauri/src/lib.rs` in the `run()` function's `invoke_handler`. Progress events are emitted from Rust to the frontend via `app.emit("gdal-progress", ...)` and consumed with Tauri's event listeners.

### Tauri Commands (lib.rs)

| Command | Purpose |
|---|---|
| `check_env_status` | Returns path + package list of the conda env |
| `create_env` | Creates/recreates the conda env with GDAL packages via micromamba |
| `check_gdal` | Verifies ogr2ogr/gdalinfo is executable in the env |
| `ogr_convert` | Main import: runs ogr2ogr with options, emits progress events |
| `optimize_postgres` | Runs ANALYZE, GIST index creation, VACUUM via psql |
| `load_connections` / `save_connection` / `delete_connection` | Persist DB connections to `config.json` in app data dir |
| `test_connection` | TCP connectivity check to PostgreSQL host:port |

### Environment Management

The app bundles `micromamba` binaries in `src-tauri/binaries/` for each platform. On first use the user creates a conda env at `<AppData>/gis_env` containing `gdal`, `libpq`, and `libgdal-pg` from conda-forge. All GDAL/psql commands are run via `micromamba run -p <env_path> <program>`, falling back to direct executable lookup within the env dirs.

### Frontend Structure

- `src/App.tsx` — all page components (`ImportPage`, `OptimizePage`, `EnvPage`, `HelpPage`) and routing via react-router-dom
- `src/contexts/ConnectionContext.tsx` — manages DB connection list and active selection; persisted via Rust commands
- `src/contexts/LogContext.tsx` — in-memory log entries displayed in `LogPanel`
- `src/contexts/AppStateContext.tsx` — env loading state and cross-component refresh signals
- `src/components/` — UI components; `ui/` subdirectory contains shadcn/ui primitives (do not hand-edit)

### DB Connection Storage

Connections are stored as JSON at the Tauri app data directory (`config.json`). The `DbConnection` struct in Rust mirrors the `DbConnection` interface in `ConnectionContext.tsx` — both must stay in sync when fields are added.

### Release

Releases are triggered by pushing a `v*` tag. GitHub Actions builds for Windows (x64), macOS (x64 + ARM) in parallel via `tauri-apps/tauri-action`. No Linux builds are included.
