# fs-blueprint

Makes the data package of a program from a blueprint (`scaffold.toml`, schema 2, ADR 0099) and checks that a repo is still what its blueprint says. The package holds data and CI, no code.

1. Install: `cargo install --git https://github.com/FreeSynergy-Net/fs-release fs-blueprint` (needs the Rust toolchain of `RUST_VERSION`).
2. Call: `fs-blueprint new scaffold.toml --out fs-myprogram`. It checks the blueprint (and the folder `forms/` next to it, which comes along) and writes the data package only when the check is clean; on an error nothing is written. `new` writes:
   - the CI: `.github/workflows/check.yml`, `.github/workflows/windows.yml`, and `.gitattributes`;
   - `packaging/`: the desktop file, the frontend manifest, the package layout;
   - the layout seed `forms/<layout>.toml`, only when the author's `forms/` does not have that file (an existing file is never overwritten; a minimal description: head, contract range, title in en and de, no page yet);
   - for the program's own types: `schema/<name>.capnp` (the contract text) and `schema/<name>.sql` (the tables);
   - `packaging/settings.toml` when the blueprint has `[[setting]]`, `packaging/help.toml` when it has `[[help]]`.

   It writes NO `Cargo.toml`, no Rust source and no `crates/` folder (finding `code-in-program`). The example `examples/leer` in the fs-release checkout (one file, `scaffold.toml`) gives the line `made leer (written 7, kept 0)`; the tree then holds 8 files, because the blueprint itself is copied in as well.

   `fs-blueprint check --repo . [--drift]` checks a repo; `--talks DIR` names the folder of the talk records, `--lang de` gives the messages in German, `--catalog FILE` adds your own capabilities.
3. Next: `cd fs-myprogram` and commit the data package. The program runs through the modules and the host `fs-starter` (a separate repository, see its README), which opens the data package; no `-bin` crate is generated (ADR 0105). Code is only possible as an exception with a talk record (`code-in-program`).

## The parts of a blueprint

- `[contract] provides` is a list of `{ contract = "dependency.evaluate", version = "0.1" }`. It only NAMES contracts of fs-interfaces (checked against the catalog rows `[[contract]]`: `contract-unknown`, `contract-version-invalid`). No `interface` is generated; a program without own types has no contract text.
- Every field of an own type names exactly one of `kit`, `basic`, `nested` or `reference` (`field-kind-both`, `field-kind-missing`).
- `nested = "<OwnType>"`: a child table `<parent>_<field>` with the columns `id`, `owner`, `pos`. A single value gets `UNIQUE (owner)`, a list `UNIQUE (owner, pos)`. In the contract text the child is a struct inside the parent, `List(Child)` when the field is `multiple`. Cycles and a type nested twice are findings (`nested-cycle`, `nested-twice`).
- `reference = "<OwnType>|IRI"`: a column when single; when `multiple` a link table with `owner`, `pos` and `target` (`INTEGER` referencing the own table, or `TEXT` for an IRI). In the contract text `List(Int64)` for an own type, `List(Text)` for an IRI. A reference to a type that is only nested is refused (`reference-to-nested`); two types or fields that would make the same table are refused (`table-collides`); `column-collides` refuses a field `id` in a top-level type, and in a nested type a field `id`, `owner` or `pos`; only fields that are neither `multiple` nor `nested` are checked.
- A type that is only nested needs no standard, envelope or media type.
- Tables are typed (`INTEGER`, `TEXT`, snake_case columns); there is no JSON or blob column. Every table and column name is quoted in the SQL.
- The contract text is append-only against the one already in the target: a field, method or type may only be added at the end. Inserting, reordering, deleting or changing a type is `contract-append-only`; nothing is written then. Nested structs count as well.
- `[[setting]]` (key, type, default, label in en and de) is written to `packaging/settings.toml` as data; there is no widget. A missing label is `text-pair`.
- `[[help]]` (topic, text in en and de) is written to `packaging/help.toml`. A text missing in one language is a visible finding (`text-pair`), never a silent gap; the variables of en and de must match (`text-vars`).

## The generated CI

The generated `check.yml` has three jobs. `kind` looks whether the repo has a `Cargo.toml`. `package` (no `Cargo.toml`) checks out fs-documentation for the talk records, installs `fs-blueprint` from the tag `v0.0.17` and runs `fs-blueprint check --repo "$PWD" --talks "$PWD/.fs-documentation" --drift`. `code` (a repo with a `Cargo.toml`) calls the fs-release `check.yml` at the same tag with `blueprint: warn`. The install tag is the one constant `skeleton::RELEASE_TAG` (today `v0.0.17`); a test fails when a pin differs.

## Findings

Every finding is a code with named parameters; the sentence comes from the message catalog (`src/messages.rs`, en and de). A finding is an error, except `skeleton-part-pending` (a warning) and a finding of a rule set to `Warn` (a warning as well); the generated CI runs the check with `blueprint: warn`.

Schema, names and versions:

- `schema-unsupported`: the blueprint schema is not one this tool reads.
- `schema-newer`: the blueprint schema is newer than the one this tool knows.
- `parse-error`: the blueprint is not readable TOML (line and column are given).
- `blueprint-unserializable`: the blueprint cannot be written in its canonical form.
- `unknown-key`: a key the schema does not know.
- `field-missing`: a mandatory key is missing in a table.
- `field-empty`: a key that needs at least one entry has none.
- `duplicate`: the same name appears twice in one table.
- `name-rule`: the program name breaks the naming rule (lower case, digits and hyphen, no `fs-` prefix).
- `name-case`: a value has the wrong letter case for its key.
- `semver-invalid`: a version is not `major.minor.patch`.
- `skeleton-unknown`: the skeleton version does not exist.
- `skeleton-part-pending`: a skeleton part (session, update, mcp-switch in this version) does not exist yet; a warning.
- `capability-unknown`: the capability is not in the list of capabilities.
- `category-main-missing`: at least one main category of the desktop menu is missing.

Catalog (modules, bundles, contracts):

- `contract-version-invalid`: a contract version is not `major.minor`.
- `contract-id-invalid`: a file id is not `0x` and 16 hexadecimal digits.
- `contract-unknown`: a provided contract at that level is not in the catalog (the contract lives in fs-interfaces).
- `module-unknown`: the module is not in the catalog.
- `bundle-unknown`: the bundle is not in the catalog.
- `version-unknown`: the module has no such tag.
- `kind-unknown`: the kind of the module is unknown.
- `kind-mismatch`: the module is declared as one kind, the catalog says another.
- `kind-wrong-role`: the module has the wrong kind for this use.
- `bundle-part-unknown`: the bundle has no such part.
- `module-in-bundle`: a part of a bundle may not also be a module.

Types, fields and standards:

- `type-unknown`: the type is not declared.
- `owned-by-module`: the type belongs to a module and carries no own key of its own.
- `standard-missing`: a field has no standard reference.
- `standard-invalid`: the standard address has no section.
- `media-type-invalid`: the value is not a media type.
- `field-kind-both`: a field names more than one of kit, basic type, nested type and reference.
- `field-kind-missing`: a field names none of kit, basic type, nested type and reference.
- `kit-unknown`: the data kit does not exist.
- `basic-unknown`: the basic type does not exist.
- `words-unknown`: the word list does not exist or is not allowed here.
- `ruleset-source-unknown`: the source of a rule set is no module or bundle of this blueprint.
- `ruleset-default-missing`: the default of a rule set is not one of its presets.
- `preset-unknown`: a rule set names an unknown preset.
- `envelope-invalid`: the value is neither a type of the ActivityStreams 2.0 vocabulary nor a full IRI.

Nested types, references and tables:

- `nested-cycle`: a type is nested inside itself, directly or through other types.
- `nested-twice`: a type is nested by more than one field; a nested type has exactly one parent.
- `reference-to-nested`: a reference names a type that is only nested and has no table of its own.
- `column-collides`: a field `id` would be a second id column next to the key column.
- `table-collides`: two types or fields would make the same table.
- `contract-append-only`: the contract text would break the earlier one (numbers and types are append-only); nothing is written.

Forms description and talk records:

- `layout-not-found`: the layout file does not exist.
- `layout-outside-forms`: the forms description must lie in `forms/` in the root of the repo, next to `scaffold.toml`.
- `layout-unreadable`: the forms host cannot read the description (the cause is given).
- `covers-invalid`: the value is neither `all` nor an entry `Interface.method` or `Type.field`.
- `talk-not-found`: the talk record of an exception does not exist.

Paths, exceptions and code:

- `path-invalid`: not a valid repo path (relative, with slash, no `..`, no wildcard).
- `path-not-found`: the path of an exception does not exist.
- `path-generated`: the path covers a generated file.
- `path-overlap`: a path is covered by two exceptions.
- `file-unowned`: a file has no generator head and no exception.
- `main-reserved`: the path covers a `main.rs`; the entry point is generated and exchangeable.
- `exception-undeclared`: a path is marked as exception but no exception of the blueprint names it.
- `code-in-program`: a hand-written `.rs` file is code in the program; only an exception with a talk record may keep it.
- `path-symlink`: a part of the path is a symbolic link; nothing is written.
- `header-unsupported`: the file kind has no comment form for the head line.
- `artifact-duplicate`: two emitters write the same path.

Generated files and writing:

- `generated-changed`: a generated file differs from what the blueprint generates (changed by hand, or the blueprint changed).
- `generated-missing`: a generated file is missing.
- `seed-missing`: a seed file is missing.
- `slot-unknown`: a template names an unknown slot.
- `slot-open`: a template has an open or unfilled slot.
- `write-failed`: a file could not be written.
- `file-unreadable`: a file is there but cannot be read as text; nothing is written.
- `line-ending`: the output has a carriage return; generated files have LF only.
- `fmt-changed`: the file is not in the canonical form (`fmt --check`).
- `fmt-unsafe`: the canonical form would change the blueprint; nothing is written.

Texts:

- `text-pair`: a text lacks a language (the parameter `missing` names it).
- `text-vars`: a text has different variables in en and de.
- `text-syntax`: a text is not a simple MF2 message (plain text and variable placeholders only).

## Packages (rpm)

`fs-blueprint package rpm BAUPLAN` makes the noarch RPM `fs-<name>` of a program; `fs-blueprint package release --key F --fingerprint HEX40 --pages-url https://...` makes `freesynergy-release` (dnf source with ONE channel `[freesynergy]` that holds pre-releases too, plus the public key). Nothing is signed here: signing is the CI. The version of `freesynergy-release` follows the fs-blueprint version; when the key or the URL changes, `--pkg-version` MUST be raised, otherwise dnf sees no update. The rpm tests run on Linux only and need `rpm` and `cpio` (a missing tool is a red test).
