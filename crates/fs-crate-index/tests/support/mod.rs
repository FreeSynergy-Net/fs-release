//! Pattern: none (test support). A throw-away registry (git index + `.crate` files, `file://`, no GitHub, no network): index line, checksum,
//! fresh resolution of a consumer with `registry = "freesynergy"`, counter-test "a changed .crate is rejected by cargo", counter-test "a branch run writes nothing".

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use fs_crate_index::PublishArgs;

pub fn run(dir: &Path, cmd: &str, args: &[&str], envs: &[(&str, &Path)]) -> Output {
    let mut c = Command::new(cmd);
    c.args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.org")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.org");
    for (k, v) in envs {
        c.env(k, v);
    }
    c.output().unwrap()
}

pub fn ok(o: &Output) -> String {
    assert!(
        o.status.success(),
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).into_owned()
}

pub struct World {
    _tmp: tempfile::TempDir,
    pub root: PathBuf,
    pub lib: PathBuf,
    pub reg: PathBuf,
    pub url: String,
    pub n: u32,
}

#[allow(dead_code)] // each test file uses a part
impl World {
    pub fn new() -> Self {
        for (k, v) in [
            ("GIT_AUTHOR_NAME", "t"),
            ("GIT_AUTHOR_EMAIL", "t@example.org"),
            ("GIT_COMMITTER_NAME", "t"),
            ("GIT_COMMITTER_EMAIL", "t@example.org"),
        ] {
            std::env::set_var(k, v);
        }
        // TEST_TMPDIR: a folder OUTSIDE every cargo workspace (a nested fixture would believe it is a workspace member)
        let tmp = match std::env::var("TEST_TMPDIR") {
            Ok(d) => tempfile::Builder::new()
                .prefix("k2-")
                .tempdir_in(d)
                .unwrap(),
            Err(_) => tempfile::tempdir().unwrap(),
        };
        let root = tmp.path().to_path_buf();
        let reg = root.join("fs-crates");
        let url = format!("file://{}", reg.display());
        reg_index::init(
            &reg,
            &format!("{url}/crates/{{crate}}/{{crate}}-{{version}}.crate"),
            None,
        )
        .unwrap();
        let lib = root.join("demo-lib");
        fs::create_dir_all(lib.join("src")).unwrap();
        fs::write(lib.join("src/lib.rs"), "pub fn answer() -> u32 { 42 }\n").unwrap();
        let w = Self {
            _tmp: tmp,
            root,
            lib,
            reg,
            url,
            n: 0,
        };
        w.set_version("0.0.7");
        w
    }
    pub fn set_version(&self, v: &str) {
        fs::write(
            self.lib.join("Cargo.toml"),
            format!("[package]\nname = \"demo-lib\"\nversion = \"{v}\"\nedition = \"2021\"\nlicense = \"MIT\"\ndescription = \"demo\"\n"),
        )
        .unwrap();
        ok(&run(&self.lib, "cargo", &["generate-lockfile"], &[]));
    }
    pub fn args(&mut self, git_ref: &str) -> PublishArgs {
        self.n += 1;
        PublishArgs {
            git_ref: git_ref.into(),
            package: "demo-lib".into(),
            workspace: self.lib.clone(),
            registry_dir: self.reg.clone(),
            index_url: self.url.clone(),
            upload: "crates/{crate}".into(),
            target_dir: self.root.join(format!("target-{}", self.n)),
        }
    }
    pub fn head(&self) -> String {
        ok(&run(&self.reg, "git", &["rev-parse", "HEAD"], &[]))
            .trim()
            .into()
    }
    pub fn consumer(&self, req: &str) -> PathBuf {
        let c = self
            .root
            .join(format!("consumer-{}", req.replace(['~', '.', '='], "_")));
        fs::create_dir_all(c.join("src")).unwrap();
        fs::create_dir_all(c.join(".cargo")).unwrap();
        fs::write(
            c.join("src/main.rs"),
            "fn main() { println!(\"{}\", demo_lib::answer()); }\n",
        )
        .unwrap();
        fs::write(
            c.join(".cargo/config.toml"),
            format!("[registries.freesynergy]\nindex = \"{}\"\n", self.url),
        )
        .unwrap();
        fs::write(
            c.join("Cargo.toml"),
            format!("[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ndemo-lib = {{ version = \"{req}\", registry = \"freesynergy\" }}\n"),
        )
        .unwrap();
        c
    }
    pub fn cargo_home(&self) -> PathBuf {
        let h = self.root.join("cargo-home");
        fs::create_dir_all(&h).unwrap();
        h
    }
}
