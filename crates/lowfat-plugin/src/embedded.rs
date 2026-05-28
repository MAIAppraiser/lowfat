//! Plugins bundled into the binary at compile time.
//!
//! These are the canonical replacements for the deleted native Rust filters
//! (git/docker/ls). They ship as **data** — DSL configuration, not Rust code —
//! so the lowfat binary itself only contains coreutils-equivalent logic + the
//! plugin protocol. A user can shadow any bundled plugin by dropping a file
//! at `~/.lowfat/plugins/<category>/<name>/filter.lf` — disk wins over bundled
//! in `discover_plugins`.
//!
//! Only the load-bearing files (`lowfat.toml` + `filter.lf`) are embedded.
//! Samples, BENCHMARK.md, bench.sh, and the legacy filter.sh are deliberately
//! left out of the binary — they're documentation, not runtime. The package
//! `exclude` in Cargo.toml also keeps them out of the published crate tarball.
//!
//! The bundled plugins live in this crate's `embedded/` dir (not the
//! workspace-root `plugins/`, which holds community plugins). They must stay
//! inside the crate: `include_str!` paths can't reach files outside the package
//! root, or `cargo publish` won't ship them.

pub struct EmbeddedPlugin {
    pub category: &'static str,
    pub name: &'static str,
    pub manifest: &'static str,
    pub filter_lf: &'static str,
}

pub const EMBEDDED: &[EmbeddedPlugin] = &[
    EmbeddedPlugin {
        category: "git",
        name: "git-compact",
        manifest: include_str!("../embedded/git/git-compact/lowfat.toml"),
        filter_lf: include_str!("../embedded/git/git-compact/filter.lf"),
    },
    EmbeddedPlugin {
        category: "docker",
        name: "docker-compact",
        manifest: include_str!("../embedded/docker/docker-compact/lowfat.toml"),
        filter_lf: include_str!("../embedded/docker/docker-compact/filter.lf"),
    },
    EmbeddedPlugin {
        category: "ls",
        name: "ls-compact",
        manifest: include_str!("../embedded/ls/ls-compact/lowfat.toml"),
        filter_lf: include_str!("../embedded/ls/ls-compact/filter.lf"),
    },
    EmbeddedPlugin {
        category: "find",
        name: "find-compact",
        manifest: include_str!("../embedded/find/find-compact/lowfat.toml"),
        filter_lf: include_str!("../embedded/find/find-compact/filter.lf"),
    },
    EmbeddedPlugin {
        category: "grep",
        name: "grep-compact",
        manifest: include_str!("../embedded/grep/grep-compact/lowfat.toml"),
        filter_lf: include_str!("../embedded/grep/grep-compact/filter.lf"),
    },
    EmbeddedPlugin {
        category: "tree",
        name: "tree-compact",
        manifest: include_str!("../embedded/tree/tree-compact/lowfat.toml"),
        filter_lf: include_str!("../embedded/tree/tree-compact/filter.lf"),
    },
];
