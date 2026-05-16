use anyhow::Result;

use crate::commands::{audit, gain};

/// `lowfat stats [--audit]` — one place to see what's actually happened.
///
///   bare           → lifetime token savings (replaces `gain`)
///   stats --audit  → recent plugin executions (replaces `audit`)
///
/// `gain` and `audit` still work as hidden aliases for backward compatibility.
pub fn run(audit_flag: bool, audit_limit: usize) -> Result<()> {
    if audit_flag {
        audit::run(audit_limit)
    } else {
        gain::run()
    }
}
