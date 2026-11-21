# fs-blueprint

Makes a program from a blueprint (`scaffold.toml`, schema 2, ADR 0099) and checks that a repo is still what its blueprint says.

1. Install: `cargo install --git https://github.com/FreeSynergy/fs-release fs-blueprint` (needs the Rust toolchain of `RUST_VERSION`).
2. Call: `fs-blueprint new scaffold.toml --out fs-myprogram` (checks the blueprint and writes a data package, no Rust source and no crate manifest (rule `code-in-program`): the check CI callers, the desktop file, the frontend manifest, the package `layout`, and for the program's own types the contract text (`.capnp`) and the tables (`.sql`); a folder `forms/` next to the blueprint comes along; without the file named by `[frontend] layout`, `new` makes a minimal forms description there (head, contract range, title in both languages, no page yet) and `check` refuses a description whose head the forms host cannot read (`layout-unreadable`); on an error nothing is written). `fs-blueprint check --repo . [--drift]` checks a repo; `--lang de` gives the sentences in German.
3. Next: `cd fs-myprogram`, commit the data package; the program runs through the modules and the host `fs-starter` (it opens the forms description; `fs-forms-run` is never started by hand), code is only possible as an exception with a talk record.
