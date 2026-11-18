//! Pattern: Strategy (the rules of the check as a set with presets; set `projekt` is the default, ADR 0099 and answer 133).

/// The adjustable rules. Everything else of the check (schema, kinds, unknown keys, forms) is structure and not adjustable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// A standard reference for every type field (`standard-missing`).
    StandardEvidence,
    /// A talk record for every exception (`talk-not-found`).
    TalkPerException,
    /// The program name follows the naming rule (`name-rule`).
    NameRule,
    /// Only existing building blocks: modules, bundles, tags, kits, skeletons, capabilities (the `*-unknown` codes).
    ExistingOnly,
}

impl Rule {
    pub const ALL: [Rule; 4] = [
        Rule::StandardEvidence,
        Rule::TalkPerException,
        Rule::NameRule,
        Rule::ExistingOnly,
    ];

    fn index(self) -> usize {
        Rule::ALL.iter().position(|r| *r == self).unwrap_or(0)
    }
}

/// What a rule does when it finds something.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Not checked.
    Off,
    /// A warning in the report.
    Warn,
    /// An error: the blueprint is red.
    Block,
}

/// A named set of modes, one per rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    name: &'static str,
    modes: [Mode; 4],
}

impl Rules {
    /// The set `projekt` (default): every rule blocks.
    #[must_use]
    pub fn projekt() -> Self {
        Self {
            name: "projekt",
            modes: [Mode::Block; 4],
        }
    }

    /// A set by its name; only `projekt` exists (a set for foreign repos waits for the user's word, question SK3).
    #[must_use]
    pub fn by_name(name: &str) -> Option<Self> {
        (name == "projekt").then(Self::projekt)
    }

    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    #[must_use]
    pub fn mode(&self, rule: Rule) -> Mode {
        self.modes[rule.index()]
    }

    /// The same set with one rule in another mode.
    #[must_use]
    pub fn with(mut self, rule: Rule, mode: Mode) -> Self {
        self.modes[rule.index()] = mode;
        self
    }
}
