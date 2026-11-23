//! Pattern: none (test). Every tool the release chain installs in CI is pinned (ADR-free rule of Bahn 1, G6 no. 5), with counter-checks.

use fs_release::ci_tools::{scan, violations};
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
}

#[test]
fn this_repository_installs_only_pinned_tools() {
    let bad = scan(root());
    assert!(bad.is_empty(), "unpinned installs:\n{}", bad.join("\n"));
}

#[test]
fn unpinned_installs_are_red() {
    let red = [
        "run: cargo install cargo-deb --locked",
        "run: cargo install cargo-deb@3.8.0",
        "run: cargo binstall --no-confirm --locked cargo-deb",
        "run: cargo binstall --no-confirm cargo-deb@3.8.0",
        "run: cargo binstall --locked cargo-deb@3.8 ",
        "run: cargo install --git https://x/y --locked fs-blueprint",
        "          tools: cargo-wix",
        "          tools: cargo-deb@3.8.0 cargo-generate-rpm",
        "        tool: cargo-binstall",
        "        TOOLS: cargo-deny@latest",
        "      - uses: actions/checkout@v4",
        "      - uses: actions/checkout@main # v4",
        "      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
        "      - uses: actions/checkout",
        "    uses: FreeSynergy/fs-release/.github/workflows/release.yml@v0.0.17",
        "      - uses: FreeSynergy/fs-release/actions/sbom@e596ed6",
    ];
    for l in red {
        assert_eq!(violations("f.yml", l).len(), 1, "should be red: {l}");
    }
}

#[test]
fn pinned_installs_and_non_installs_are_green() {
    let green = [
        "run: cargo install cargo-deb@3.8.0 --locked",
        "run: cargo install --version 3.8.0 --locked cargo-deb",
        "run: cargo install --git https://x/y --rev \"$REV\" --locked fs-blueprint",
        "run: cargo binstall --no-confirm --locked cargo-deb@3.8.0 cargo-wix@0.3.9",
        "run: cargo binstall --no-confirm --locked $TOOLS",
        "          tools: cargo-deb@3.8.0",
        "        tool: cargo-binstall@1.28.0",
        "  tools: { description: \"each as name@x.y.z\", required: true }",
        "        TOOLS: ${{ inputs.tools }}",
        "# cargo install cargo-deb (a comment)",
        "run: cargo build --locked",
        "      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1",
        "      - uses: FreeSynergy/fs-release/actions/sbom@e596ed65e56e5cdca1a9a453d0f12f7bc4b2edf7",
        "      - uses: ./release/actions/setup-rust",
    ];
    for l in green {
        assert!(violations("f.yml", l).is_empty(), "should be green: {l}");
    }
}
