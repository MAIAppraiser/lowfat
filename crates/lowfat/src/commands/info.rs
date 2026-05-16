use anyhow::Result;

use crate::commands::{config, filters, pipeline, status};

/// `lowfat info [cmd] [--config]` — one place to see what's currently set up.
///
///   bare           → status badge + active filter list
///   info <cmd>     → pipeline applied to that command
///   info --config  → full resolved config (paths, level, env overrides)
///
/// Replaces the older `status`, `filters`, `pipeline`, and `config` commands,
/// all of which still work as hidden aliases for backward compatibility.
pub fn run(cmd: Option<&str>, config_flag: bool) -> Result<()> {
    if config_flag {
        return config::run();
    }
    if let Some(c) = cmd {
        return pipeline::run(c);
    }
    // Default view: badge first (skips silently when there's no data),
    // then the enabled filter inventory.
    status::run()?;
    filters::run(false)
}
