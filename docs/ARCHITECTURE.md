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
  plugins are embedded via `include_str!`; user plugins live under
  `~/.lowfat/plugins/`. Disk wins on name collision.
- **Builtins** — in-process processors (`strip-ansi`, `head`, `grep`,
  `dedup-blank`, `normalize`, …) used as pipeline stages.
- **SQLite metrics** — `~/.lowfat/data/` tracks token savings and invocation
  history (powers `lowfat stats` and `lowfat history`).
