use anyhow::Result;
use lowfat_core::config::RunfConfig;
use lowfat_core::lf::{self, Op};
use lowfat_plugin::discovery::discover_plugins;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn list() -> Result<()> {
    let config = RunfConfig::resolve();
    let plugins = discover_plugins(&config.plugin_dir);

    if plugins.is_empty() {
        println!("No community plugins installed.");
        println!("  Plugin dir: {}", config.plugin_dir.display());
        return Ok(());
    }

    println!("Community plugins:");
    println!();
    for plugin in &plugins {
        let m = &plugin.manifest;
        let name = &m.plugin.name;
        let version = m.plugin.version.as_deref().unwrap_or("?");
        let cmds = m.plugin.commands.join(", ");
        let category = &plugin.category;

        println!(
            "  {category}/{name} v{version} — commands: [{cmds}]"
        );
    }

    Ok(())
}

pub fn doctor() -> Result<()> {
    let config = RunfConfig::resolve();
    let plugins = discover_plugins(&config.plugin_dir);

    if plugins.is_empty() {
        println!("No community plugins to check.");
        return Ok(());
    }

    let uv_available = is_on_path("uv");
    let python_available = is_on_path("python3");

    let mut ready = 0;
    let mut total = 0;
    let mut needs_uv = false;
    let mut prewarmed = 0;

    for plugin in &plugins {
        total += 1;
        let name = &plugin.manifest.plugin.name;
        let entry_path = plugin
            .base_dir
            .join(plugin.manifest.runtime.resolve_entry(&plugin.base_dir));
        if !entry_path.exists() {
            println!("  {name:<24} x entry not found: {}", entry_path.display());
            continue;
        }

        let requires = &plugin.manifest.runtime.requires;
        if requires.contains_key("uv") {
            needs_uv = true;
        }

        let is_lf = entry_path
            .extension()
            .map(|e| e == "lf")
            .unwrap_or(false);
        if !is_lf {
            println!("  {name:<24} ok (shell)");
            ready += 1;
            continue;
        }

        // Parse .lf to verify syntactic validity
        let source = match std::fs::read_to_string(&entry_path) {
            Ok(s) => s,
            Err(e) => {
                println!("  {name:<24} x cannot read: {e}");
                continue;
            }
        };
        let rs = match lf::parse(&source) {
            Ok(r) => r,
            Err(e) => {
                println!("  {name:<24} x parse error: {e:#}");
                continue;
            }
        };

        // Collect python bodies with PEP 723 headers for prewarming
        let pep723_bodies = collect_pep723_bodies(&rs);
        if pep723_bodies.is_empty() {
            println!("  {name:<24} ok (.lf, {} rules)", rs.rules.len());
            ready += 1;
            continue;
        }
        if !uv_available {
            println!(
                "  {name:<24} ! needs uv to resolve {} PEP 723 body(ies)",
                pep723_bodies.len()
            );
            continue;
        }

        let mut all_ok = true;
        for (i, body) in pep723_bodies.iter().enumerate() {
            match prewarm_uv(body) {
                Ok(_) => prewarmed += 1,
                Err(e) => {
                    println!(
                        "  {name:<24} x PEP 723 body #{}: {e:#}",
                        i + 1
                    );
                    all_ok = false;
                    break;
                }
            }
        }
        if all_ok {
            println!(
                "  {name:<24} ok (.lf, {} rules, {} uv env(s) cached)",
                rs.rules.len(),
                pep723_bodies.len()
            );
            ready += 1;
        }
    }

    println!();
    println!("  {ready}/{total} plugins ready, {prewarmed} uv env(s) warmed.");
    if needs_uv && !uv_available {
        println!();
        println!("  ! uv not on PATH — required by at least one plugin.");
        println!("    install: curl -LsSf https://astral.sh/uv/install.sh | sh");
        println!("    or:      brew install uv");
    }
    if !python_available {
        println!();
        println!("  ! python3 not on PATH — `python:` blocks will fail.");
    }
    Ok(())
}

fn is_on_path(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Walk the ruleset and collect every `python:` body that declares
/// PEP 723 inline dependencies. Includes bodies inside macro definitions
/// and inside `split` sub-chains.
fn collect_pep723_bodies(rs: &lf::RuleSet) -> Vec<String> {
    let mut out = Vec::new();
    for d in &rs.defines {
        walk_ops(&d.ops, &mut out);
    }
    for r in &rs.rules {
        walk_ops(&r.ops, &mut out);
    }
    out
}

fn walk_ops(ops: &[Op], out: &mut Vec<String>) {
    for op in ops {
        match op {
            Op::Python(body) => {
                if body
                    .lines()
                    .any(|l| l.trim_start().starts_with("# /// script"))
                {
                    out.push(body.clone());
                }
            }
            Op::Split { pre, post, .. } => {
                walk_ops(pre, out);
                walk_ops(post, out);
            }
            _ => {}
        }
    }
}

/// Trigger uv dep resolution by running the script with empty stdin.
/// uv caches resolved envs at `~/.cache/uv/`, so the first real invocation
/// hits a warm cache.
fn prewarm_uv(body: &str) -> Result<()> {
    let mut script = tempfile::Builder::new()
        .prefix("lowfat-doctor-")
        .suffix(".py")
        .tempfile()?;
    script.write_all(body.as_bytes())?;
    script.flush().ok();
    let path = script.path().to_str().unwrap().to_string();

    let mut child = Command::new("uv")
        .args(["run", "--script", &path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    // Empty stdin: many scripts will exit immediately on stdin.read(); deps
    // are resolved during uv's startup regardless of script behavior.
    drop(child.stdin.take());
    let output = child.wait_with_output()?;
    if !output.status.success() {
        // Non-zero exit from the script body itself is fine — what we care
        // about is whether uv could resolve the env. Distinguish by checking
        // stderr for uv-level errors vs. script tracebacks.
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("error:") && !stderr.contains("Traceback") {
            anyhow::bail!("uv: {}", stderr.lines().next().unwrap_or("").trim());
        }
    }
    Ok(())
}

pub fn info(name: &str) -> Result<()> {
    let config = RunfConfig::resolve();
    let plugins = discover_plugins(&config.plugin_dir);

    let plugin = plugins
        .iter()
        .find(|p| p.manifest.plugin.name == name);

    match plugin {
        Some(p) => {
            let m = &p.manifest;
            println!("Plugin: {}", m.plugin.name);
            println!("  Version:     {}", m.plugin.version.as_deref().unwrap_or("?"));
            println!("  Description: {}", m.plugin.description.as_deref().unwrap_or("-"));
            println!("  Author:      {}", m.plugin.author.as_deref().unwrap_or("-"));
            println!("  Category:    {}", p.category);
            println!("  Entry:       {}", m.runtime.resolve_entry(&p.base_dir));
            println!("  Commands:    {}", m.plugin.commands.join(", "));
            println!("  Path:        {}", p.base_dir.display());
        }
        None => {
            eprintln!("lowfat: plugin not found: {name}");
        }
    }

    Ok(())
}

pub fn trust(name: &str) -> Result<()> {
    let config = RunfConfig::resolve();
    lowfat_plugin::security::trust_plugin(name, &config.home_dir)?;
    println!("lowfat: plugin '{name}' is now trusted");
    Ok(())
}

pub fn untrust(name: &str) -> Result<()> {
    let config = RunfConfig::resolve();
    lowfat_plugin::security::untrust_plugin(name, &config.home_dir)?;
    println!("lowfat: trust revoked for plugin '{name}'");
    Ok(())
}

pub fn new_plugin(name: &str, command: &str) -> Result<()> {
    let config = RunfConfig::resolve();

    // Create plugin directory: ~/.lowfat/plugins/<command>/<name>/
    let plugin_dir = config.plugin_dir.join(command).join(name);
    if plugin_dir.exists() {
        anyhow::bail!("plugin already exists: {}", plugin_dir.display());
    }
    std::fs::create_dir_all(&plugin_dir)?;

    // Write lowfat.toml manifest. No [runtime] needed — the entrypoint is
    // auto-detected (filter.lf wins over filter.sh).
    let manifest = format!(
        r#"[plugin]
name = "{name}"
commands = ["{command}"]
"#,
        name = name,
        command = command,
    );
    std::fs::write(plugin_dir.join("lowfat.toml"), manifest)?;

    // Write filter rules
    std::fs::write(plugin_dir.join("filter.lf"), scaffold_lf(name, command))?;

    // Scaffold samples/ directory
    let samples_dir = plugin_dir.join("samples");
    std::fs::create_dir_all(&samples_dir)?;
    std::fs::write(
        samples_dir.join(format!("{command}-output-full.txt")),
        "# Paste real command output here.\n# Filename convention: <command>-<subcommand>-<level>.txt\n# Run: lowfat plugin bench <name>\n",
    )?;

    // Auto-trust the plugin
    lowfat_plugin::security::trust_plugin(name, &config.home_dir)?;

    println!("lowfat: created plugin '{name}'");
    println!("  {}", plugin_dir.display());
    println!("  edit: {}", plugin_dir.join("filter.lf").display());
    println!("  bench: lowfat plugin bench {name}");
    println!("  test: lowfat {command} <args>");
    Ok(())
}

/// Scaffold a starter `filter.lf` — a level-scaled head, the safe default.
fn scaffold_lf(name: &str, command: &str) -> String {
    format!(
        r#"#!/usr/bin/env lowfat-filter
# {name} — compact {command} output for LLM contexts
#
# Rules match (subcommand, level) top-down; first match wins.
# Levels: ultra (~10 lines) · full (~30) · lite (~60).
# Ops: keep /re/ · drop /re/ · head N · tail N · else "text".
# Escape hatches: `shell: <cmd>` and `python: |` when ops aren't enough.

*, ultra:
    head 10

*, lite:
    head 60

*:
    head 30
"#
    )
}

pub fn bench(name: &str) -> Result<()> {
    let config = RunfConfig::resolve();
    let plugins = discover_plugins(&config.plugin_dir);

    let plugin = plugins
        .iter()
        .find(|p| p.manifest.plugin.name == name);

    let plugin = match plugin {
        Some(p) => p,
        None => {
            // Also check repo plugins/ directory
            anyhow::bail!("plugin not found: {name} (install it to ~/.lowfat/plugins/ first)");
        }
    };

    let samples_dir = plugin.base_dir.join("samples");
    if !samples_dir.is_dir() {
        anyhow::bail!("no samples/ directory in plugin '{name}' — add .txt files with sample command output");
    }

    let mut entries: Vec<_> = std::fs::read_dir(&samples_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "txt"))
        .collect();
    entries.sort_by_key(|e| e.path());

    if entries.is_empty() {
        anyhow::bail!("no .txt sample files in {}", samples_dir.display());
    }

    // Build the process filter
    let process_filter = lowfat_runner::process::ProcessFilter {
        info: lowfat_plugin::plugin::PluginInfo {
            name: plugin.manifest.plugin.name.clone(),
            version: plugin.manifest.plugin.version.clone().unwrap_or_default(),
            commands: plugin.manifest.plugin.commands.clone(),
            subcommands: plugin.manifest.plugin.subcommands.clone().unwrap_or_default(),
        },
        entry: plugin
            .base_dir
            .join(plugin.manifest.runtime.resolve_entry(&plugin.base_dir)),
        base_dir: plugin.base_dir.clone(),
    };

    println!("Benchmark: {name}");
    println!();

    let mut total_raw = 0usize;
    let mut total_filtered = 0usize;

    for entry in &entries {
        let path = entry.path();
        let sample_name = path.file_stem().unwrap_or_default().to_string_lossy();

        // Parse sample name: "git-status-full.txt" → command=git, subcommand=status, level=full
        let parts: Vec<&str> = sample_name.split('-').collect();
        let (command, subcommand, level_str) = match parts.len() {
            1 => (parts[0], "", "full"),
            2 => (parts[0], parts[1], "full"),
            _ => (parts[0], parts[1], parts[parts.len() - 1]),
        };

        let level = match level_str {
            "lite" => lowfat_core::level::Level::Lite,
            "ultra" => lowfat_core::level::Level::Ultra,
            _ => lowfat_core::level::Level::Full,
        };

        let raw = std::fs::read_to_string(&path)?;
        let raw_tokens = lowfat_core::tokens::estimate_tokens(&raw);

        let input = lowfat_plugin::plugin::FilterInput {
            raw: raw.clone(),
            command: command.to_string(),
            subcommand: subcommand.to_string(),
            args: vec![],
            level,
            head_limit: level.head_limit(40),
            exit_code: 0,
        };

        use lowfat_plugin::plugin::FilterPlugin;
        let result = process_filter.filter(&input)?;
        let filtered_tokens = lowfat_core::tokens::estimate_tokens(&result.text);
        let pct = if raw_tokens > 0 {
            (1.0 - filtered_tokens as f64 / raw_tokens as f64) * 100.0
        } else {
            0.0
        };

        total_raw += raw_tokens;
        total_filtered += filtered_tokens;

        println!(
            "  {:<30} {:>6} → {:>6} tokens  ({:>-3.0}%)",
            format!("{sample_name} ({level})"), raw_tokens, filtered_tokens, -pct
        );
    }

    if total_raw > 0 {
        let total_pct = (1.0 - total_filtered as f64 / total_raw as f64) * 100.0;
        println!();
        println!(
            "  {:<30} {:>6} → {:>6} tokens  ({:>-3.0}%)",
            "TOTAL", total_raw, total_filtered, -total_pct
        );
    }

    Ok(())
}