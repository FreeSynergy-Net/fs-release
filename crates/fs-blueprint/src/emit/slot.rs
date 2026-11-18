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
}

impl Slot {
    pub const ALL: [Slot; 8] = [
        Slot::Program,
        Slot::Kebab,
        Slot::Snake,
        Slot::Pascal,
        Slot::AppId,
        Slot::Capability,
        Slot::ContractVersion,
        Slot::ContractId,
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

impl Values {
    #[must_use]
    pub fn of(bp: &Blueprint) -> Self {
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
        };
        Self {
            list: Slot::ALL.into_iter().map(|s| (s, value(s))).collect(),
        }
    }

    #[must_use]
    pub fn get(&self, slot: Slot) -> &str {
        self.list
            .iter()
            .find(|(s, _)| *s == slot)
            .map_or("", |(_, v)| v.as_str())
    }
}

/// Fills a template. `path` names the file for the error.
///
/// # Errors
/// `slot-unknown{name}` for a name that is no slot; `slot-open{path}` for a `{{` without `}}` and for a `{{` that is left in the result; `line-ending{path}` for a lone CR.
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
        let slot = Slot::parse(key).ok_or_else(|| Coded::new("slot-unknown").with("name", key))?;
        out.push_str(values.get(slot));
        rest = &tail[end + 2..];
    }
    out.push_str(rest);
    if out.contains("{{") {
        return Err(Coded::new("slot-open").with("path", path));
    }
    if out.contains('\r') {
        return Err(Coded::new("line-ending").with("path", path));
    }
    Ok(out)
}
