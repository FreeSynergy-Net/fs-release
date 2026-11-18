//! Pattern: Template method (a closed list of slots; a template names them as `{{slot}}`; an unknown slot or an open `{{` is an error, never left in the output).

use crate::error::Coded;
use crate::model::Blueprint;

/// The slots of a template. The list is closed: an unknown name is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// `tasks`
    Program,
    /// `fs-tasks`
    Kebab,
    /// `fs_tasks`
    Snake,
    /// `Tasks`
    Pascal,
    /// `net.freesynergy.Tasks`
    AppId,
    /// `tasks` (the capability key)
    Capability,
    /// `0.1`
    ContractVersion,
    /// `0xc04d258971543476`
    ContractId,
    /// `0.0.1`: the version of the program (`[program] version`)
    Version,
    /// `v0.0.2`: the tag of fs-rpc of the skeleton version
    PinRpc,
    /// `v0.0.11`: the tag of fs-release of the skeleton version
    ReleaseTag,
    /// The summary in English, on one line
    SummaryEn,
    /// The summary in German, on one line
    SummaryDe,
    /// `Office;ProjectManagement;`
    Categories,
}

impl Slot {
    pub const ALL: [Slot; 14] = [
        Slot::Program,
        Slot::Kebab,
        Slot::Snake,
        Slot::Pascal,
        Slot::AppId,
        Slot::Capability,
        Slot::ContractVersion,
        Slot::ContractId,
        Slot::Version,
        Slot::PinRpc,
        Slot::ReleaseTag,
        Slot::SummaryEn,
        Slot::SummaryDe,
        Slot::Categories,
    ];

    /// The name in a template: `{{name}}`.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Slot::Program => "program",
            Slot::Kebab => "kebab",
            Slot::Snake => "snake",
            Slot::Pascal => "pascal",
            Slot::AppId => "app_id",
            Slot::Capability => "capability",
            Slot::ContractVersion => "contract_version",
            Slot::ContractId => "contract_id",
            Slot::Version => "version",
            Slot::PinRpc => "pin_rpc",
            Slot::ReleaseTag => "release_tag",
            Slot::SummaryEn => "summary_en",
            Slot::SummaryDe => "summary_de",
            Slot::Categories => "categories",
        }
    }

    #[must_use]
    pub fn parse(key: &str) -> Option<Slot> {
        Slot::ALL.into_iter().find(|s| s.key() == key)
    }
}

/// `my-task` becomes `MyTask`.
fn pascal(name: &str) -> String {
    name.split('-')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// The value of every slot, taken from the blueprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Values {
    list: Vec<(Slot, String)>,
}

/// A text on one line: white space of every kind becomes one space.
fn one_line(text: Option<&str>) -> String {
    text.unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

impl Values {
    /// The values of every slot.
    ///
    /// # Errors
    /// `skeleton-unknown{found}` when the skeleton version of the blueprint is not in the table of `skeleton.rs`.
    pub fn of(bp: &Blueprint) -> Result<Self, Coded> {
        let skeleton = super::skeleton::of(&bp.program.skeleton).ok_or_else(|| {
            Coded::new("skeleton-unknown").with("found", bp.program.skeleton.as_str())
        })?;
        let name = bp.program.name.as_str();
        let pascal = pascal(name);
        let value = |s: Slot| match s {
            Slot::Program => name.to_string(),
            Slot::Kebab => format!("fs-{name}"),
            Slot::Snake => format!("fs_{}", name.replace('-', "_")),
            Slot::Pascal => pascal.clone(),
            Slot::AppId => format!("net.freesynergy.{pascal}"),
            Slot::Capability => bp.program.capability.clone(),
            Slot::ContractVersion => bp.contract.version.clone(),
            Slot::ContractId => bp.contract.id.clone(),
            Slot::Version => bp.program.version.clone(),
            Slot::PinRpc => skeleton.rpc_tag.to_string(),
            Slot::ReleaseTag => skeleton.release_tag.to_string(),
            Slot::SummaryEn => one_line(bp.program.summary.en.as_deref()),
            Slot::SummaryDe => one_line(bp.program.summary.de.as_deref()),
            Slot::Categories => bp.program.categories.join(";") + ";",
        };
        Ok(Self {
            list: Slot::ALL.into_iter().map(|s| (s, value(s))).collect(),
        })
    }

    #[must_use]
    pub fn get(&self, slot: Slot) -> &str {
        self.list
            .iter()
            .find(|(s, _)| *s == slot)
            .map_or("", |(_, v)| v.as_str())
    }
}

/// Fills a template. `{{name}}` is a slot; `{{open}}` and `{{close}}` are the two literal brace pairs (for `${{ ... }}` of a workflow). `path` names the file for the error.
///
/// # Errors
/// `slot-unknown{name}` for a name that is no slot; `slot-open{path}` for a `{{` without `}}` and for a value that holds `{{` or `}}`; `line-ending{path}` for a lone CR.
pub fn fill(template: &str, values: &Values, path: &str) -> Result<String, Coded> {
    // a template checked out with CRLF (git autocrlf) gives the same bytes as one with LF
    let template = template.replace("\r\n", "\n");
    let mut out = String::with_capacity(template.len());
    let mut rest = template.as_str();
    while let Some(i) = rest.find("{{") {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 2..];
        let Some(end) = tail.find("}}") else {
            return Err(Coded::new("slot-open").with("path", path));
        };
        let key = tail[..end].trim();
        match key {
            "open" => out.push_str("{{"),
            "close" => out.push_str("}}"),
            _ => {
                let slot =
                    Slot::parse(key).ok_or_else(|| Coded::new("slot-unknown").with("name", key))?;
                let value = values.get(slot);
                // a value never brings a slot or a brace pair of its own
                if value.contains("{{") || value.contains("}}") {
                    return Err(Coded::new("slot-open").with("path", path));
                }
                out.push_str(value);
            }
        }
        rest = &tail[end + 2..];
    }
    out.push_str(rest);
    if out.contains('\r') {
        return Err(Coded::new("line-ending").with("path", path));
    }
    Ok(out)
}
