# AGENTS.md

## Project Overview

`roll` is a CLI dice roller written in Rust (edition 2024). It parses dice expressions like `2d6+5`, rolls them with a PRNG, and outputs results in a formatted table with color highlighting (crits in green, fumbles in red).

## Commands

```bash
cargo build                # build
cargo test                 # run all tests (unit tests inline in expression.rs)
cargo clippy               # lint
cargo run -- 2d6+3         # run with expression
cargo run -- -a 2d6        # average mode (no actual roll)
cargo run -- add fireball 8d6  # save alias
cargo run -- rm fireball       # remove alias
cargo run -- list              # list saved aliases
```

No CI config, Makefile, or integration test harness exists. Tests are `#[cfg(test)]` unit tests in `src/expression.rs` only.

## Architecture

### Control Flow

1. `main()` → `run(args)` → dispatches on `Mode` (Norm / Average / Add / Rem / List)
2. **Norm mode**: `execute_expressions` — parses expressions, realizes (rolls) them, builds a `comfy_table::Table`, prints it, then appends roll history to `~/.roll.history`
3. **Average mode**: `print_averages` — computes expected value per expression without rolling

### Key Modules

| Module | Responsibility |
|---|---|
| `args` | CLI parsing via `clap` derive. Defines `Args`, `SubCommand`, `Mode`, `PathConfig` |
| `expression` | `ExpressionParser` (regex-based), `Expression` model, `Realizer` trait, `RealizedExpression` output |
| `token` | `TokenExtractor` trait + impls for reroll (`r`/`rN`) and explode (`!`/`!N`/`e`/`eN`) suffixes |
| `realize` | `RandomRealizer<R>` — concrete `Realizer` impl using `rand`; `LogWrapper` for roll history tracking |
| `history` | Appends roll logs to `~/.roll.history` in format `timestamp|version|die_size:val1,val2,...` |
| `error` | `thiserror`-based error types: `Error` (top-level) and `ExpressionError` (parsing) |
| `default_iter` | `DefaultIfEmpty` iterator adapter — yields a default value if the iterator is empty |

### Data Flow

```
CLI input → expand_expressions (handles N*expr / Nxexpr count syntax)
         → ExpressionParser::parse (regex) → Expression
         → Realizer::realize → RealizedExpression
         → Into<comfy_table::Row> (color-coded output)
         → History::append_log → write to ~/.roll.history
```

## Conventions & Patterns

- **No tests for anything outside `expression.rs`** — all unit tests live in `mod tests` at the bottom of `expression.rs`. The test helper `parse()` calls `ExpressionParser::new().parse(expr).unwrap()`.
- **`cow` everywhere** — the table rows use `Cow<str>` and `Either` to avoid allocations for mixed static/dynamic content.
- **`SmallVec<[i32; 4]>`** — dice result arrays use smallvec to avoid heap allocation for typical rolls (≤4 dice).
- **SquirrelRng** — the default PRNG is `squirrel-rng` (deterministic hash-based), not `rand::thread_rng()`.
- **Serde on Expression** — `Expression` derives `Serialize`/`Deserialize` so aliases (stored as JSON in `~/.roll`) can embed parsed expressions.
- **No `mod.rs` files** — flat module structure, all in `src/`.
- **Rust 2024 edition** — uses latest edition features.

## Expression Syntax

- `NdM` — N dice with M sides (`2d6`)
- `N` — shorthand for `1dN` (`20` = `1d20`)
- `dM` — also `1dM`
- `+N` / `-N` — modifier
- `a`/`A` prefix — advantage (roll twice, take higher; only on first die)
- `s`/`S` prefix — disadvantage (roll twice, take lower; only on first die)
- `r` / `rN` — reroll values ≤ 1 (or ≤ N)
- `!` / `!N` / `e` / `eN` — explode on max (or ≥ N)
- `Nx` / `N*` / `NX` — repeat expression N times (`2d6*3` = three instances of `2d6`)

## Gotchas

- **Linux shell globbing**: `2d6*2` must be quoted (`'2d6*2'`) or use `2d6x2` — the `*` is interpreted by the shell.
- **Config paths**: Aliases are stored in `~/.roll` (JSON). History goes to `~/.roll.history`. The `-c`/`--config` flag selects alternate profiles (`~/.roll.profilename`).
- **Reroll logic is `self.0 >= value`** (reroll if threshold ≥ value), while **explode logic is `value >= self.0`** (explode if value ≥ threshold). These are inverted — this is intentional but looks like a bug at first glance.
- **Advantage/disadvantage only applies to the first die** in a multi-die expression — the `advantage` is `.take()`d after the first iteration.
- **No subcommand = roll mode**. The `add`, `rm`, and `list` subcommands manage aliases. `-a`/`--show-average` switches to average mode.
- **`expand_expressions`** handles the count syntax (`N*expr`) and is called before any parsing — it duplicates the raw string, not the parsed expression.
