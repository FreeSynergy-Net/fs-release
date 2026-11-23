//! Pattern: none (test support). A throw-away world, no GitHub, no network: a registry (git index + `.crate` files, `file://`), a source repo with three
//! crates that mirror fs-types / fs-conformance / fs-records (a path dependency WITH `version` and `registry`, an optional path dependency,
//! a feature `conf = ["dep:demo-lib"]`), tags, an allow list and a consumer with `registry = "freesynergy"`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use fs_crate_index::{ApplyArgs, ScanArgs};

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
    /// the source repo (a git repo with the workspace of the three crates)
    pub src: PathBuf,
    pub reg: PathBuf,
    pub url: String,
    pub allow: PathBuf,
    pub n: u32,
}

const CRATES: [&str; 3] = ["demo-base", "demo-lib", "demo-app"];

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
        let src = root.join("owner/demo");
        let allow = root.join("allowed-sources.txt");
        fs::write(
            &allow,
            "# test\nowner/demo 0.0.7 demo-base demo-lib demo-app\n",
        )
        .unwrap();
        let w = Self {
            _tmp: tmp,
            root,
            src,
            reg,
            url,
            allow,
            n: 0,
        };
        fs::create_dir_all(w.src.join(".cargo")).unwrap();
        fs::write(
            w.src.join(".cargo/config.toml"),
            format!("[registries.freesynergy]\nindex = \"{}\"\n", w.url),
        )
        .unwrap();
        fs::write(
            w.src.join("Cargo.toml"),
            "[workspace]\nresolver = \"2\"\nmembers = [\"demo-base\", \"demo-lib\", \"demo-app\"]\n",
        )
        .unwrap();
        std::os::unix::fs::symlink(&w.src, w.root.join("owner/demo.git")).unwrap(); // the clone URL ends with `.git`
        ok(&run(&w.src, "git", &["init", "-q", "-b", "main"], &[]));
        w.commit("init");
        w
    }

    /// Writes the three crates in `version`; `answer` is the value `demo-base` returns.
    pub fn write_crates(&self, version: &str, answer: u32) {
        let dep = |n: &str, extra: &str| {
            format!("{n} = {{ path = \"../{n}\", version = \"{version}\", registry = \"freesynergy\"{extra} }}\n")
        };
        for c in CRATES {
            fs::create_dir_all(self.src.join(c).join("src")).unwrap();
            let deps = match c {
                "demo-lib" => dep("demo-base", ""),
                "demo-app" => dep("demo-base", "") + &dep("demo-lib", ", optional = true"),
                _ => String::new(),
            };
            let features = if c == "demo-app" {
                "\n[features]\nconf = [\"dep:demo-lib\"]\n"
            } else {
                ""
            };
            fs::write(
                self.src.join(c).join("Cargo.toml"),
                format!("[package]\nname = \"{c}\"\nversion = \"{version}\"\nedition = \"2021\"\nlicense = \"MIT\"\ndescription = \"demo\"\npublish = [\"freesynergy\"]\n{features}\n[dependencies]\n{deps}"),
            )
            .unwrap();
            let lib = match c {
                "demo-base" => format!("pub fn answer() -> u32 {{ {answer} }}\n"),
                "demo-lib" => "pub fn lib_answer() -> u32 { demo_base::answer() + 1 }\n".into(),
                _ => "pub fn app_answer() -> u32 { demo_base::answer() }\n#[cfg(feature = \"conf\")]\npub fn conf_answer() -> u32 { demo_lib::lib_answer() }\n".into(),
            };
            fs::write(self.src.join(c).join("src/lib.rs"), lib).unwrap();
        }
        ok(&run(&self.src, "cargo", &["generate-lockfile"], &[]));
    }

    pub fn commit(&self, msg: &str) {
        ok(&run(&self.src, "git", &["add", "-A"], &[]));
        ok(&run(&self.src, "git", &["commit", "-q", "-m", msg], &[]));
    }

    /// New release: crates in `version`, commit on main, annotated tag `v<version>`.
    pub fn release(&self, version: &str, answer: u32) {
        self.write_crates(version, answer);
        self.commit(&format!("release {version}"));
        ok(&run(
            &self.src,
            "git",
            &["tag", "-a", "-m", "release", &format!("v{version}")],
            &[],
        ));
    }

    pub fn scan_args(&mut self) -> ScanArgs {
        self.n += 1;
        ScanArgs {
            allow: self.allow.clone(),
            registry_dir: self.reg.clone(),
            source_base: format!("file://{}", self.root.display()),
            work_dir: self.root.join(format!("work-{}", self.n)),
            out: self.root.join(format!("out-{}", self.n)),
            semver_checks: None,
        }
    }
    pub fn apply_args(&self, scan: &ScanArgs) -> ApplyArgs {
        ApplyArgs {
            allow: self.allow.clone(),
            dir: scan.out.clone(),
            registry_dir: self.reg.clone(),
            index_url: self.url.clone(),
            source_base: scan.source_base.clone(),
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
            "fn main() { println!(\"{} {}\", demo_app::app_answer(), demo_app::conf_answer()); }\n",
        )
        .unwrap();
        fs::write(
            c.join(".cargo/config.toml"),
            format!("[registries.freesynergy]\nindex = \"{}\"\n", self.url),
        )
        .unwrap();
        fs::write(
            c.join("Cargo.toml"),
            format!("[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ndemo-app = {{ version = \"{req}\", registry = \"freesynergy\", features = [\"conf\"] }}\n"),
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
