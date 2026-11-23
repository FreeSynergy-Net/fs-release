//! Pattern: none (data: what the runner recorded of one run in `out-<task>-<date>.json`, stream-json lines).

use serde_json::Value;

/// The facts of a run the approval needs: the model, the working folder and every report the agent wrote with `Write`.
#[derive(Debug, Default)]
pub struct RunOut {
    pub model: String,
    pub cwd: String,
    /// `(file_path, content)` of each `Write` tool call.
    pub writes: Vec<(String, String)>,
}

impl RunOut {
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut out = Self::default();
        for ev in text
            .lines()
            .filter(|l| l.starts_with('{'))
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        {
            if ev["type"] == "system" && ev["subtype"] == "init" {
                ev["model"]
                    .as_str()
                    .unwrap_or("")
                    .clone_into(&mut out.model);
                ev["cwd"].as_str().unwrap_or("").clone_into(&mut out.cwd);
            }
            let Some(parts) = ev["message"]["content"].as_array() else {
                continue;
            };
            for p in parts
                .iter()
                .filter(|p| p["type"] == "tool_use" && p["name"] == "Write")
            {
                let path = p["input"]["file_path"].as_str().unwrap_or("");
                let body = p["input"]["content"].as_str().unwrap_or("");
                out.writes.push((path.into(), body.into()));
            }
        }
        out
    }
}
