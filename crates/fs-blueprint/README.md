# fs-blueprint

Makes a program from a blueprint (`scaffold.toml`, schema 2, ADR 0099) and checks that a repo is still what its blueprint says.

1. Install: `cargo install --git https://github.com/FreeSynergy/fs-release fs-blueprint` (needs the Rust toolchain of `RUST_VERSION`).
2. Call: `fs-blueprint new scaffold.toml --out fs-myprogram` (checks the blueprint, makes the program; a folder `forms/` next to the blueprint comes along; on an error nothing is written). `fs-blueprint check --repo . [--drift]` checks a repo; `--lang de` gives the sentences in German.
3. Build: `cd fs-myprogram && cargo build && target/debug/fs-<name> --version`.
