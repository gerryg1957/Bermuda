# MoyoDB core and command-line tool

This Rust project implements the first working MoyoDB pipeline:

- SGF FF[4]-style collection and tree parsing
- first-child main-variation extraction
- setup stones (`AB`, `AW`, and `AE`)
- captures, suicide checking, passes, and simple ko
- compact, versioned `.moves` files
- a `moyodb` command-line executable

## Build and test

```bash
cargo fmt
cargo test
cargo clippy --all-targets --all-features
cargo build --release
```

The release executable is written to:

```text
target/release/moyodb
```

## Commands

Show help:

```bash
cargo run -- --help
```

Convert an SGF game to a compact move file:

```bash
cargo run -- import game.sgf game.moves
```

Inspect a compact move file:

```bash
cargo run -- inspect game.moves
```

Replay the entire game:

```bash
cargo run -- replay game.moves
```

Replay through move 100:

```bash
cargo run -- replay game.moves --move-number 100
```

After a release build, replace `cargo run --` with `target/release/moyodb`.

## Deliberate current limits

- board sizes are limited to 19 or smaller;
- the first game in an SGF collection is selected;
- the first child at each branch is treated as the main variation;
- simple ko is implemented, not positional or situational superko;
- compressed SGF point ranges such as `AB[aa:cc]` are not yet expanded;
- character-set conversion from legacy SGF encodings is not yet implemented.

The next milestone is directory import with SQLite metadata and duplicate detection.
