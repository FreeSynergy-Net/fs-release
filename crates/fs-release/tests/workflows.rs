//! Pattern: none (test). The workflow files in `workflows/`: schema check (`scripts/check_workflows.py`), step names equal to
//! the code, and mutations (broken YAML, missing actor, duplicate name, ...) that must turn the check red.

mod support;
use fs_release::lift::{self, LiftOpts};
use std::fs;
use std::path::PathBuf;
use support::workflows::{assert_red, read, root, script, step_body, step_names, FILES};
use support::{have_yaml, python};

#[test]
fn the_real_files_pass_the_schema_check() {
    for f in FILES {
        assert_ne!(read(f), "");
    }
    let (code, out) = python(&script(), &[&root().join("workflows")]);
    assert_eq!(code, if have_yaml() { 0 } else { 2 }, "{out}");
}

#[test]
fn the_list_equals_the_files_in_the_folder_and_all_carry_the_crate_version() {
    let mut on_disk: Vec<String> = fs::read_dir(root().join("workflows"))
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().into_string().unwrap();
            name.strip_suffix(".yaml").map(str::to_string)
        })
        .collect();
    on_disk.sort();
    assert_eq!(on_disk, FILES);
    let version = format!("  version: {}\n", env!("CARGO_PKG_VERSION"));
    for f in FILES {
        assert!(read(f).contains(&version), "{f}.yaml: document.version");
    }
}

#[test]
fn every_catalog_call_and_the_chain_file_carry_the_crate_version() {
    let version = env!("CARGO_PKG_VERSION");
    for f in FILES {
        for line in read(f).lines() {
            if let Some((_, call)) = line.trim().split_once("call: ") {
                let call = call.trim();
                assert!(
                    call.ends_with(&format!(":{version}@fs")),
                    "{f}.yaml: catalog call {call:?} is not at {version}"
                );
            }
        }
    }
    let chain = fs::read_to_string(root().join("chains/release.chain.yaml")).unwrap();
    assert!(
        chain.contains(&format!("\nversion: {version}\n")),
        "chain version"
    );
    assert!(
        chain.contains(&format!("\ncatalog: fs/{version}\n")),
        "chain catalog"
    );
}

#[test]
fn nobody_pushes_or_merges_as_an_agent_and_the_coordinator_is_the_person() {
    for f in FILES {
        assert!(
            !read(f).contains("gate: coordinator"),
            "{f}.yaml: the coordinator is `gate: human`"
        );
    }
    for (file, step) in [
        ("publish-doc", "push"),
        ("adr-write", "push"),
        ("workflow-pflege", "to-main"),
    ] {
        let text = read(file);
        let body = text.split(&format!("  - {step}:")).nth(1).unwrap();
        let body = body.split("\n  - ").next().unwrap();
        assert!(
            body.contains("actor: human") && body.contains("gate: human"),
            "{file}.yaml step {step}: push and merge are the person's"
        );
    }
}

#[test]
fn the_cost_figures_are_a_script_and_the_agent_only_explains_a_deviation() {
    let text = read("cost-reconcile");
    for step in ["compare", "unknown", "correction", "limit"] {
        let body = text.split(&format!("  - {step}:")).nth(1).unwrap();
        let body = body.split("\n  - ").next().unwrap();
        assert!(
            body.contains("actor: machine"),
            "cost-reconcile step {step} is a script"
        );
    }
    let body = text.split("  - explain:").nth(1).unwrap();
    assert!(
        body.contains("actor: ai") && body.contains("if: "),
        "the agent only on a deviation"
    );
}

#[test]
fn picture_acceptance_keeps_the_user_verdict_human_and_subjective() {
    let text = read("picture-acceptance");
    let step = text.split("  - user-verdict:").nth(1).unwrap();
    let step = step.split("\n  - ").next().unwrap();
    assert!(step.contains("actor: human") && step.contains("kind: subjective"));
}

#[test]
fn lift_yaml_has_exactly_the_steps_of_the_code_in_order() {
    let mut o = LiftOpts::new(PathBuf::new(), PathBuf::new());
    o.commit = true;
    let code: Vec<&str> = lift::tasks(&o).iter().map(|t| t.name()).collect();
    assert_eq!(step_names(&read("lift")), code);
}

#[test]
fn release_flow_yaml_names_the_flow_steps_in_order_and_bumps_after_the_release() {
    let names = step_names(&read("release-flow"));
    let code: Vec<&str> = fs_release::flow::tasks().iter().map(|t| t.name()).collect();
    let at = |n: &str| {
        names
            .iter()
            .position(|x| x == n)
            .unwrap_or_else(|| panic!("no step {n}"))
    };
    let positions: Vec<usize> = code.iter().map(|n| at(n)).collect();
    assert!(
        positions.windows(2).all(|p| p[0] < p[1]),
        "{names:?} vs {code:?}"
    );
    assert_eq!(names.first().map(String::as_str), Some("lift"));
    assert!(at("push") < at("release") && at("release") < at("bump") && at("bump") < at("sign"));
    let text = read("release-flow");
    assert!(text.contains("workflow: bump.yml") && text.contains("repo: FreeSynergy/Catalog"));
}

#[test]
fn the_files_hold_no_secrets() {
    for f in FILES {
        let t = read(f).to_lowercase();
        for bad in [
            "ghp_",
            "github_pat_",
            "-----begin",
            "password:",
            "token:",
            "secret:",
        ] {
            assert!(!t.contains(bad), "{f}.yaml contains {bad:?}");
        }
    }
}

#[test]
fn mutation_broken_yaml_is_red_with_file_and_line() {
    assert_red("lift", "lift.yaml:", |t| format!("{t}  - [unclosed\n"));
}

#[test]
fn mutation_missing_actor_is_red() {
    assert_red("lift", "actor", |t| {
        t.replacen("        freesynergy.net:actor: machine\n", "", 1)
    });
}

#[test]
fn mutation_unknown_actor_value_is_red() {
    assert_red("agent-run", "'robot'", |t| {
        t.replacen(
            "freesynergy.net:actor: human",
            "freesynergy.net:actor: robot",
            1,
        )
    });
}

#[test]
fn mutation_actor_type_that_does_not_match_the_actor_is_red() {
    assert_red("agent-run", "actor-type must be Person", |t| {
        t.replacen("actor-type: Person", "actor-type: Service", 1)
    });
}

#[test]
fn mutation_missing_gate_is_red() {
    assert_red("release-flow", "gate", |t| {
        t.replacen("        freesynergy.net:gate: none\n", "", 1)
    });
}

#[test]
fn mutation_duplicate_step_name_is_red() {
    assert_red("lift", "duplicate step name 'deps'", |t| {
        t.replacen("  - lock:\n", "  - deps:\n", 1)
    });
}

#[test]
fn mutation_missing_document_version_is_red() {
    assert_red("release-flow", "document.version", |t| {
        t.replacen("  version: 0.0.12\n", "", 1)
    });
}

#[test]
fn mutation_missing_or_unknown_kind_is_red() {
    assert_red("bugfix", "kind", |t| {
        t.replacen("        freesynergy.net:kind: objective\n", "", 1)
    });
    assert_red("bugfix", "'maybe'", |t| {
        t.replacen("kind: objective", "kind: maybe", 1)
    });
}

#[test]
fn mutation_wrong_dsl_and_empty_do_are_red() {
    assert_red("lift", "document.dsl", |t| {
        t.replacen("dsl: 1.0.3", "dsl: 0.8", 1)
    });
    assert_red("cleanup-targets", "`do`", |t| {
        let head = t.split("\ndo:").next().unwrap().to_string();
        format!("{head}\ndo: []\n")
    });
}

#[test]
fn a_step_without_a_task_type_is_red() {
    assert_red("lift", "no task type", |t| {
        t.replacen("      call: deps:0.0.12@fs\n", "", 1)
    });
}

#[test]
fn lift_v1plus_switches_the_api_before_the_lift_and_release_flow_has_no_switch() {
    let names = step_names(&read("lift-v1plus"));
    assert_eq!(names, ["api-umstellung", "lift", "umstellung-commit"]);
    let text = read("lift-v1plus");
    assert!(text.contains("name: lift\n") && text.contains("fs-rpc: ${ .input.fs_rpc }"));
    // no lock-bound check right after the switch: the lock has to change, the lift checks and commits
    let step = text.split("  - lift:").next().unwrap();
    assert!(step.contains("NO `cargo check --locked` here"));
    // conditions 4 and 5: the lift does not commit (it takes no *.rs), gets the lock and the env as input
    let lift = step_body(&text, "lift");
    assert!(lift.contains("commit: false\n") && !lift.contains("commit: true"));
    assert!(lift.contains("online: true\n") && lift.contains("FS_LIFT_V1PLUS: \"1\"\n"));
    // the own step after the switch commits everything in one commit, by name
    let commit = step_body(&text, "umstellung-commit");
    assert!(commit.contains("ONE commit") && commit.contains("never `git add -A`"));
    assert!(commit.contains("support@freesynergy.net") && commit.contains("Co-Authored-By"));
    assert!(!step_names(&read("release-flow")).contains(&"api-umstellung".to_string()));
    assert!(!step_names(&read("lift")).contains(&"api-umstellung".to_string()));
}

#[test]
fn lift_v1plus_without_the_commit_step_after_the_switch_is_red() {
    // mutant: the step `umstellung-commit` removed -> the structure test fails
    let text = read("lift-v1plus");
    let cut = text.replace("  - umstellung-commit:", "  - weg:");
    assert_ne!(step_names(&cut), step_names(&text));
    let names = step_names(&text);
    let pos = |n: &str| names.iter().position(|x| x == n).unwrap();
    assert!(
        pos("umstellung-commit") > pos("api-umstellung") && pos("umstellung-commit") > pos("lift")
    );
    assert!(!step_names(&cut).contains(&"umstellung-commit".to_string()));
}

const STATES_LINE: &str = "    freesynergy.net:states: [offen, wartet-auf-entscheidung, auf-anfrage, in-prüfung, pausiert, abgebrochen, erledigt, archiviert]\n";

fn drop_line(t: &str, start: &str) -> String {
    let mut gone = false;
    let mut out = String::new();
    for l in t.lines() {
        if !gone && l.starts_with(start) {
            gone = true;
            continue;
        }
        out.push_str(l);
        out.push('\n');
    }
    out
}

#[test]
fn every_workflow_carries_the_eight_states_and_the_transition_rules() {
    for f in FILES {
        let t = read(f);
        assert!(
            t.contains(STATES_LINE),
            "{f}.yaml: the eight states (workflows/ZUESTAENDE.md)"
        );
        for rule in [
            "      pausiert: [",
            "      archiviert: []",
            "      abgebrochen: [offen, archiviert]",
        ] {
            assert!(t.contains(rule), "{f}.yaml: transition rule {rule:?}");
        }
    }
}

#[test]
fn the_state_standard_document_names_every_state_and_rule() {
    let doc = fs::read_to_string(root().join("workflows/ZUESTAENDE.md")).unwrap();
    for s in [
        "offen",
        "wartet-auf-entscheidung",
        "auf-anfrage",
        "in-prüfung",
        "pausiert",
        "abgebrochen",
        "erledigt",
        "archiviert",
        "stationen",
    ] {
        assert!(
            doc.contains(&format!("`{s}`")) || doc.contains(s),
            "ZUESTAENDE.md: {s}"
        );
        assert!(STATES_LINE.contains(s) || s == "stationen", "{s}");
    }
}

#[test]
fn mutation_state_deleted_from_any_workflow_is_red() {
    for f in FILES {
        assert_red(f, "state 'wartet-auf-entscheidung' is missing", |t| {
            t.replacen("wartet-auf-entscheidung, auf-anfrage, in-prüfung, pausiert, abgebrochen, erledigt, archiviert]", "auf-anfrage, in-prüfung, pausiert, abgebrochen, erledigt, archiviert]", 1)
        });
    }
}

#[test]
fn mutation_states_block_removed_from_any_workflow_is_red() {
    for f in FILES {
        assert_red(f, &format!("{f}.yaml:"), |t| {
            drop_line(&t, STATES_LINE.trim_end())
        });
    }
}

#[test]
fn mutation_state_renamed_is_red() {
    assert_red("lift", "state 'pausiert' is missing", |t| {
        t.replacen(
            "in-prüfung, pausiert, abgebrochen, erledigt, archiviert]",
            "in-prüfung, angehalten, abgebrochen, erledigt, archiviert]",
            1,
        )
    });
    assert_red("lift", "unknown state 'angehalten'", |t| {
        t.replacen(
            "in-prüfung, pausiert, abgebrochen, erledigt, archiviert]",
            "in-prüfung, angehalten, abgebrochen, erledigt, archiviert]",
            1,
        )
    });
}

#[test]
fn mutation_transition_rule_missing_or_incomplete_is_red() {
    assert_red("bugfix", "transition rule of 'pausiert' is missing", |t| {
        drop_line(&t, "      pausiert: [")
    });
    assert_red(
        "bugfix",
        "transition 'abgebrochen' -> 'offen' is missing",
        |t| {
            t.replacen(
                "abgebrochen: [offen, archiviert]",
                "abgebrochen: [archiviert]",
                1,
            )
        },
    );
    assert_red(
        "bugfix",
        "transition 'offen' -> 'pausiert' is missing",
        |t| {
            t.replacen(
                "offen: [stationen, wartet-auf-entscheidung, auf-anfrage, pausiert, abgebrochen]",
                "offen: [stationen, wartet-auf-entscheidung, auf-anfrage, abgebrochen]",
                1,
            )
        },
    );
    assert_red("bugfix", "freesynergy.net:transitions is missing", |t| {
        t.replacen(
            "freesynergy.net:transitions:",
            "freesynergy.net:uebergaenge:",
            1,
        )
    });
}

#[test]
fn mutation_archiviert_only_from_erledigt_or_abgebrochen_and_without_way_out() {
    assert_red("bugfix", "'pausiert' -> 'archiviert' is not allowed", |t| {
        t.replacen(
            "in-prüfung, abgebrochen]\n",
            "in-prüfung, abgebrochen, archiviert]\n",
            1,
        )
    });
    assert_red("bugfix", "'archiviert' has no way out", |t| {
        t.replacen("archiviert: []", "archiviert: [offen]", 1)
    });
}

#[test]
fn mutation_a_step_named_like_a_state_is_red() {
    assert_red("lift", "carries the name of a state", |t| {
        t.replacen("  - lock:\n", "  - pausiert:\n", 1)
    });
}

#[test]
fn mutation_pausiert_and_abgebrochen_must_be_reachable_from_every_live_state() {
    let stationen = "      stationen: [stationen, wartet-auf-entscheidung, auf-anfrage, in-prüfung, pausiert, abgebrochen, erledigt]";
    assert_red(
        "bugfix",
        "transition 'stationen' -> 'pausiert' is missing",
        |t| t.replacen(stationen, &stationen.replace("pausiert, ", ""), 1),
    );
    assert_red(
        "bugfix",
        "transition 'stationen' -> 'abgebrochen' is missing",
        |t| t.replacen(stationen, &stationen.replace("abgebrochen, ", ""), 1),
    );
    assert_red(
        "bugfix",
        "transition 'offen' -> 'abgebrochen' is missing",
        |t| {
            t.replacen(
                "offen: [stationen, wartet-auf-entscheidung, auf-anfrage, pausiert, abgebrochen]",
                "offen: [stationen, wartet-auf-entscheidung, auf-anfrage, pausiert]",
                1,
            )
        },
    );
    assert_red(
        "bugfix",
        "transition 'in-prüfung' -> 'pausiert' is missing",
        |t| {
            t.replacen(
                "auf-anfrage, pausiert, abgebrochen, erledigt]\n      pausiert",
                "auf-anfrage, abgebrochen, erledigt]\n      pausiert",
                1,
            )
        },
    );
}
