# fs-blueprint

Makes a program from a blueprint (`scaffold.toml`, schema 2, ADR 0099) and checks that a repo is still what its blueprint says.

1. Install: `cargo install --git https://github.com/FreeSynergy/fs-release fs-blueprint` (needs the Rust toolchain of `RUST_VERSION`).
2. Call: `fs-blueprint new scaffold.toml --out fs-myprogram` (checks the blueprint and writes a data package, no Rust source (rule `code-in-program`): the check CI callers, the crate manifests, the desktop file, the frontend manifest and the package `layout`; a folder `forms/` next to the blueprint comes along; on an error nothing is written). `fs-blueprint check --repo . [--drift]` checks a repo; `--lang de` gives the sentences in German.
3. Next: `cd fs-myprogram`, commit the data package; the program runs through the modules and the forms host (`fs-forms-run`), code is only possible as an exception with a talk record.
