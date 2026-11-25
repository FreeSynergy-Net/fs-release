//! Pattern: none (test). The chain gate of the lift step `pins`: red gate = red step, nothing written.

mod support;

use fs_release::lift;
use fs_release_chain::sys::SystemRunner;
use std::fs;
use support::rel_tags::tag_release;
use support::{chain_stub, git, World};

#[test]
fn red_chain_gate_makes_pins_red_and_writes_nothing() {
    let w = World::new();
    tag_release(&w, &["v0.0.18"]);
    let wf = w.work().join(".github/workflows");
    fs::create_dir_all(&wf).unwrap();
    let check =
        "jobs:\n  check:\n    uses: FreeSynergy/fs-release/.github/workflows/check.yml@main\n";
    fs::write(wf.join("check.yml"), check).unwrap();
    git(&w.work(), &["add", "-A"]);
    git(&w.work(), &["commit", "--quiet", "-m", "workflow"]);
    let mut o = w.opts();
    o.env.retain(|(k, _)| k != "FS_RELEASE_BIN");
    o.env.push(chain_stub::red(&w));
    let (r, _) = lift::run(&o, &SystemRunner::default());
    let s = r.steps.iter().find(|s| s.name == "pins").unwrap();
    assert_eq!(s.status, "red", "{}", r.markdown());
    assert!(
        s.first_error
            .clone()
            .unwrap_or_default()
            .contains("chain gate"),
        "{s:?}"
    );
    assert_eq!(w.read(".github/workflows/check.yml"), check);
}
