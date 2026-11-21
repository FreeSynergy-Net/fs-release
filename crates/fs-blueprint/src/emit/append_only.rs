//! Pattern: Guard (the contract text is append-only: against the text that is already there, only a member at the end of a struct and a new struct at the
//! end of the top level are allowed; inserting, reordering, deleting and changing a member or a type are breaks).
//!
//! The numbers `@N` of the emitter are the places of the members, so a change anywhere but at the end would number everything after it again without a word.
//! A struct nested in another one is found by its name (`Parent.Child`), the structs of the top level by their place.

/// A `struct` of the text with its member lines; `name` is qualified (`Parent.Child`) for a nested one.
struct Block<'a> {
    name: String,
    nested: bool,
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

/// The file id (the first line that is only `@0x...;`) and the blocks of a text as the emitter writes it (a stack of the open structs).
fn parse(text: &str) -> (Option<&str>, Vec<Block<'_>>) {
    let mut id = None;
    let mut blocks: Vec<Block<'_>> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    for line in text.lines() {
        let word = line
            .trim_start()
            .strip_prefix("struct ")
            .and_then(|r| r.split_whitespace().next());
        if line.starts_with('@') && id.is_none() {
            id = Some(line.trim_end());
        } else if let Some(name) = word {
            let name = match open.last() {
                Some(&p) => format!("{}.{name}", blocks[p].name),
                None => name.to_string(),
            };
            blocks.push(Block {
                name,
                nested: !open.is_empty(),
                members: Vec::new(),
            });
            open.push(blocks.len() - 1);
        } else if line.trim() == "}" {
            open.pop();
        } else if line.starts_with("  ") {
            if let Some(&i) = open.last() {
                blocks[i].members.push(line.trim());
            }
        }
    }
    (id, blocks)
}

/// Why the members of `is` break those of `was`, or `None` when they only append.
fn members(was: &Block<'_>, is: &Block<'_>) -> Option<String> {
    let kind = if was.nested {
        "nested struct"
    } else {
        "struct"
    };
    for (j, member) in was.members.iter().enumerate() {
        match is.members.get(j) {
            Some(m) if m == member => {}
            Some(m) => return Some(format!("{kind} {}: {member} became {m}", was.name)),
            None => return Some(format!("{kind} {}: {member} removed", was.name)),
        }
    }
    None
}

/// Why `new` would break `old`, or `None` when it only appends.
#[must_use]
pub fn broken(old: &str, new: &str) -> Option<String> {
    let (old_id, old_blocks) = parse(old);
    let (new_id, new_blocks) = parse(new);
    if old_id != new_id {
        return Some("file id changed".to_string());
    }
    let now: Vec<&Block<'_>> = new_blocks.iter().filter(|b| !b.nested).collect();
    for (i, was) in old_blocks.iter().filter(|b| !b.nested).enumerate() {
        let Some(is) = now.get(i).filter(|b| b.name == was.name) else {
            return Some(format!("struct {} moved or removed", was.name));
        };
        if let Some(why) = members(was, is) {
            return Some(why);
        }
    }
    for was in old_blocks.iter().filter(|b| b.nested) {
        let Some(is) = new_blocks.iter().find(|b| b.nested && b.name == was.name) else {
            return Some(format!("nested struct {} removed", was.name));
        };
        if let Some(why) = members(was, is) {
            return Some(why);
        }
    }
    None
}
