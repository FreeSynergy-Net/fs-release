//! Pattern: Guard (the contract text is append-only: against the text that is already there, only a member at the end of a struct or interface and a new
//! struct or interface at the end of its kind are allowed; inserting, reordering, deleting and changing a type are breaks).
//!
//! The numbers `@N` of the emitter are the places of the members, so a change anywhere but at the end would number everything after it again without a word.

/// A `struct` or `interface` of the text with its member lines.
struct Block<'a> {
    kind: &'static str,
    name: &'a str,
    members: Vec<&'a str>,
}

/// True for the path of a contract text of the generator.
#[must_use]
pub fn is_contract(path: &str) -> bool {
    path.starts_with("schema/")
        && std::path::Path::new(path)
            .extension()
            .is_some_and(|e| e == "capnp")
}

/// The file id (the first line that is only `@0x...;`) and the blocks of a text as the emitter writes it.
fn parse(text: &str) -> (Option<&str>, Vec<Block<'_>>) {
    let mut id = None;
    let mut blocks: Vec<Block<'_>> = Vec::new();
    let mut open = false;
    for line in text.lines() {
        let word = |k: &str| line.strip_prefix(k).and_then(|r| r.split_whitespace().next());
        if line.starts_with('@') && id.is_none() {
            id = Some(line.trim_end());
        } else if let Some(name) = word("struct ") {
            blocks.push(Block {
                kind: "struct",
                name,
                members: Vec::new(),
            });
            open = true;
        } else if let Some(name) = word("interface ") {
            blocks.push(Block {
                kind: "interface",
                name,
                members: Vec::new(),
            });
            open = true;
        } else if line.starts_with('}') {
            open = false;
        } else if open && line.starts_with("  ") {
            if let Some(b) = blocks.last_mut() {
                b.members.push(line.trim());
            }
        }
    }
    (id, blocks)
}

/// Why `new` would break `old`, or `None` when it only appends.
#[must_use]
pub fn broken(old: &str, new: &str) -> Option<String> {
    let (old_id, old_blocks) = parse(old);
    let (new_id, new_blocks) = parse(new);
    if old_id != new_id {
        return Some("file id changed".to_string());
    }
    for kind in ["struct", "interface"] {
        let now: Vec<&Block<'_>> = new_blocks.iter().filter(|b| b.kind == kind).collect();
        for (i, was) in old_blocks.iter().filter(|b| b.kind == kind).enumerate() {
            let Some(is) = now.get(i).filter(|b| b.name == was.name) else {
                return Some(format!("{kind} {} moved or removed", was.name));
            };
            for (j, member) in was.members.iter().enumerate() {
                match is.members.get(j) {
                    Some(m) if m == member => {}
                    Some(m) => {
                        return Some(format!("{kind} {}: {member} became {m}", was.name));
                    }
                    None => return Some(format!("{kind} {}: {member} removed", was.name)),
                }
            }
        }
    }
    None
}
