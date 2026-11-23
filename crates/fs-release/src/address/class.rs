//! Pattern: Value object (the class of a file path; letters as in the stock list `BESTAND-vorher.md`).

/// Class of a path: a Cargo sources, b lock, c CI, d scripts, e catalog data, f docs, g workflows, h rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    CargoSource,
    CargoLock,
    Ci,
    Scripts,
    Catalog,
    Docs,
    Workflows,
    Rest,
}

/// Files that decide which git source cargo or the lock gate sees: part 2, never touched by the replace step.
const CARGO_SOURCE_FILES: &[&str] = &[
    "scripts/lock-sources.toml",
    "scripts/lock-gleich",
    "scripts/check_rust_rules.py",
];

fn ext(name: &str, e: &str) -> bool {
    std::path::Path::new(name).extension().is_some_and(|x| x.eq_ignore_ascii_case(e))
}

impl Class {
    #[must_use]
    pub fn of(path: &str) -> Self {
        let name = path.rsplit('/').next().unwrap_or(path);
        if name == "Cargo.lock" {
            Self::CargoLock
        } else if name == "Cargo.toml"
            || name == "deny.toml"
            || path.split('/').any(|c| c == ".cargo")
            || CARGO_SOURCE_FILES.contains(&path)
        {
            Self::CargoSource
        } else if path.starts_with(".github/") {
            Self::Ci
        } else if path.starts_with("scripts/") {
            Self::Scripts
        } else if (name.contains("catalog") || name.contains("packages")) && (ext(name, "xml") || ext(name, "toml")) {
            Self::Catalog
        } else if path.starts_with("workflows/") {
            Self::Workflows
        } else if ext(name, "md") || path.starts_with("docs/") {
            Self::Docs
        } else {
            Self::Rest
        }
    }

    #[must_use]
    pub fn letter(self) -> char {
        match self {
            Self::CargoSource => 'a',
            Self::CargoLock => 'b',
            Self::Ci => 'c',
            Self::Scripts => 'd',
            Self::Catalog => 'e',
            Self::Docs => 'f',
            Self::Workflows => 'g',
            Self::Rest => 'h',
        }
    }

    /// Part 2 (Cargo sources and lock files): the after-check of part 1 ignores them, the replace step never writes them.
    #[must_use]
    pub fn is_cargo(self) -> bool {
        matches!(self, Self::CargoSource | Self::CargoLock)
    }

    /// The replace step leaves these alone: Cargo classes and the signed catalog data (only the `katalog` workflow changes those).
    #[must_use]
    pub fn replace_forbidden(self) -> bool {
        self.is_cargo() || self == Self::Catalog
    }
}
