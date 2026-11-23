//! Pattern: Command (the `dag-check` subcommand: reads the Cargo.toml graph, checks or emits the header blocks of `pin-kaskade.yaml`).

use std::path::{Path, PathBuf};

use clap::Args;

#[derive(Args, Clone)]
pub struct DagArgs {
    /// The workflow file (`pin-kaskade.yaml`).
    pub workflow: PathBuf,
    /// Folder with one checkout per repo (`<dir>/<repo>/**/Cargo.toml`).
    #[arg(long)]
    pub repos: PathBuf,
    /// The trigger repo: Glieder are the repos that depend on it.
    #[arg(long, default_value = "fs-types")]
    pub root: String,
    /// Known foreign sources (`OWNER/NAME` per line), no edges.
    #[arg(long)]
    pub foreign: Option<PathBuf>,
    /// Print the generated header blocks instead of checking.
    #[arg(long)]
    pub emit: bool,
}

pub fn dag_check(a: &DagArgs) -> i32 {
    let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    let run = || -> Result<i32, String> {
        let text = read(&a.workflow)?;
        let foreign = match &a.foreign {
            Some(p) => crate::dag::foreign_repos(&read(p)?),
            None => std::collections::BTreeSet::default(),
        };
        let g = crate::dag::scan(&a.repos, &foreign)?;
        let h = crate::dag::parse_header(&text);
        for u in &g.unknown {
            eprintln!("WARNING owner is no repo of the folder: {u}");
        }
        if a.emit {
            return match crate::dag::emit(&g, &a.root, &h.order) {
                Ok(s) => {
                    print!("{s}");
                    Ok(0)
                }
                Err(c) => Err(format!("cycle among: {}", c.join(", "))),
            };
        }
        let f = crate::dag::check(&g, &h, &a.root);
        for l in &f {
            println!("RED {l}");
        }
        if f.is_empty() {
            println!("GREEN dag-check: every edge of the Cargo.toml files stands in the DAG, the order is topological");
        }
        Ok(i32::from(!f.is_empty()))
    };
    run().unwrap_or_else(|e| {
        eprintln!("fs-release dag-check: {e}");
        2
    })
}
