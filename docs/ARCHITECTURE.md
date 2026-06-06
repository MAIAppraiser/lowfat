# Architecture

High-level view of how `lowfat` filters command output between you and your AI agent.

```
       ┌──────────────────────────────────────────┐
       │  User / AI Agent                         │
       │  $ lowfat <cmd>   |   Claude hook stdin  │
       └────────────────────┬─────────────────────┘
                            │
                            ▼
       ┌──────────────────────────────────────────┐
       │              lowfat CLI                  │
       │     parse args → dispatch command        │
       └────────────────────┬─────────────────────┘
                            │ run <cmd>
                            ▼
   ┌───────────────────────────────────────────────────┐
   │                 lowfat Runner                     │
   │                                                   │
   │   exec cmd  ─▶  resolve pipeline  ─▶  filter      │
   │     (real)        (config+plugin)     (chain)     │
   └──────┬───────────────┬───────────────────┬────────┘
          │               │                   │
          ▼               ▼                   ▼
     ┌────────┐     ┌──────────┐       ┌──────────────┐
     │ Config │     │ Plugins  │       │  Builtins    │
     │ .lowfat│     │ embedded │       │  strip-ansi  │
     │  env   │     │ + ~/.lf  │       │  head/grep…  │
     └────────┘     └──────────┘       └──────────────┘
                            │
                            ▼
                  ┌─────────────────────┐
                  │  filtered output    │ ──▶ Agent
                  │  + SQLite metrics   │
                  └─────────────────────┘
```

## Components

- **lowfat CLI** (`crates/lowfat`) — clap entry point, dispatches subcommands.
- **lowfat Runner** (`crates/lowfat-runner`) — executes the real command, loads
  plugins via `HybridRunner`, and walks the pipeline stages.
- **Config** (`crates/lowfat-core`) — resolves `.lowfat` TOML + env vars into a
  `RunfConfig` (level, plugin dir, conditional pipelines).
- **Plugins** (`crates/lowfat-plugin`) — manifest + `.lf` DSL files. Bundled
  plugins live in the crate's `embedded/` dir and are baked in via
  `include_str!`; community plugins live at the repo-root `plugins/`; user
  plugins live under `~/.lowfat/plugins/`. A same-named user plugin overrides
  the bundled one.
- **Builtins** — in-process processors (`strip-ansi`, `head`, `grep`,
  `dedup-blank`, `normalize`, …) used as pipeline stages.
- **SQLite metrics** — `$XDG_DATA_HOME/lowfat` (default `~/.local/share/lowfat`,
  override with `$LOWFAT_DATA`) holds `history.db`, tracking token savings and
  invocation history (powers `lowfat stats` and `lowfat history`).
