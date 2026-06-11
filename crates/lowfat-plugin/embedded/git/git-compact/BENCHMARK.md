# git-compact — Token Saving Benchmark

Run: `sh bench.sh`

| Sample | Level | Input | Output | Saved | % |
|---|---|---|---|---|---|
| git-diff-full | full | 2,782t | 1,743t | 1,039t | 37% |
| git-diff-full | ultra | 2,782t | 121t | 2,661t | **95%** |
| git-log-full | full | 910t | 178t | 732t | 80% |
| git-log-full | ultra | 910t | 86t | 824t | **90%** |
| git-show-full | full | 1,192t | 630t | 562t | 47% |
| git-show-full | ultra | 1,192t | 116t | 1,076t | **90%** |
| git-status-full | full | 127t | 49t | 78t | 61% |
| git-status-full | ultra | 127t | 33t | 94t | **74%** |

## Optimizations beyond static samples

These bench numbers reflect single representative payloads. Three optimizations
shipped in v0.3.12 target redundancy patterns that show up most in real usage,
where small invocations dominate:

1. **Drop redundant `--- a/X` / `+++ b/X` headers** — always duplicate the path
   already on `diff --git a/X b/Y`. Saves ~2 lines per file boundary; visible on
   multi-file diffs/shows.
2. **`--stat` / `--name-only` no-pattern fallback** — old filter returned empty
   for these (triggering raw passthrough, recorded as 0% savings); new filter
   runs a compact pass (drop blanks + 50-line cap), so large stat outputs stop
   showing up as zero-savings rows.
3. **Strip commit-message trailers and abbreviate the long hash** —
   `Signed-off-by:`, `Co-authored-by:`, `Change-Id:`, `Reviewed-by:`,
   `Acked-by:`, `Tested-by:`, `Reported-by:`, `Cc:` are dropped; the 40-hex
   commit hash is shortened to 12-hex (decoration like `(HEAD -> main)` is
   preserved). Helps every `git show` / `git log` row.
4. **Honest truncation + uncapped lite** (v0.6.10) — when the diff cap
   hits, a tail marker reports how many files/lines were cut instead of
   ending silently (silent truncation misleads the LLM reader). `lite` is
   now the escape hatch the marker points at: no line cap, every
   hunk/change/context line kept, with only redundant pre-hunk meta
   (`index`/mode/`---`/`+++`) and blank context lines dropped (~3% on an
   add-heavy diff, more with many files or renames). The marker costs
   ~30t on capped diffs; the old behaviour hid thousands of changed lines
   with no indication.
