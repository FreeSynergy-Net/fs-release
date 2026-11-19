//! Pattern: none (test). The safety rules of a status command (`freesynergy.net:status-command`): `timeout` 1..3600 s, local origin, no shell
//! syntax, no shell, no interpreter, a script under `scripts/` or a program on the allow list.

mod support;
use support::workflows::{assert_green, assert_red};

const META: &str = "      metadata:\n";
const ACTOR: &str = "        freesynergy.net:actor: human\n";
const LOCAL: &str = "        freesynergy.net:origin: local\n";
const T30: &str = "      timeout:\n        after: PT30S\n";

/// `bugfix` whose first step runs a status command; `timeout` is the block before `metadata`, `extra` further metadata lines.
fn with_status(t: &str, cmd: &str, extra: &str, timeout: &str) -> String {
    let t = t.replacen(META, &format!("{timeout}{META}"), 1);
    t.replacen(
        ACTOR,
        &format!("{ACTOR}        freesynergy.net:status-command: {cmd}\n{extra}"),
        1,
    )
}

fn after(a: &str) -> String {
    format!("      timeout:\n        after: {a}\n")
}

#[test]
fn valid_status_commands_are_green() {
    assert_green("bugfix", |t| {
        with_status(&t, "scripts/check-x --fast", LOCAL, T30)
    });
    for a in [
        "PT1H",
        "PT60M",
        "PT3600S",
        "PT1S",
        "{hours: 1}",
        "{minutes: 5}",
        "{seconds: 3600}",
    ] {
        assert_green("bugfix", |t| {
            with_status(&t, "scripts/check-x", LOCAL, &after(a))
        });
    }
    assert_green("bugfix", |t| {
        with_status(
            &t,
            "cargo test",
            &format!("{LOCAL}        freesynergy.net:allow: [cargo]\n"),
            &after("PT5M"),
        )
    });
}

#[test]
fn mutation_a_status_command_without_timeout_or_with_a_bad_one_is_red() {
    assert_red("bugfix", "a status command needs a `timeout`", |t| {
        with_status(&t, "scripts/check-x", LOCAL, "")
    });
    for (a, why) in [
        ("PT2H", "7200 s"),
        ("PT0S", "0 s"),
        ("PT1H1S", "3601 s"),
        ("{hours: 2}", "7200 s"),
        ("{seconds: 3601}", "3601 s"),
        ("{seconds: 0}", "0 s"),
    ] {
        assert_red("bugfix", &format!("is {why}, allowed 1..3600"), |t| {
            with_status(&t, "scripts/check-x", LOCAL, &after(a))
        });
    }
    for a in [
        "soon",
        "{seconds: true}",
        "{minutes: 2, seconds: -100}",
        "{days: 1}",
        "{seconds: 1.5}",
        "{}",
    ] {
        assert_red("bugfix", "a status command needs a `timeout`", |t| {
            with_status(&t, "scripts/check-x", LOCAL, &after(a))
        });
    }
}

#[test]
fn mutation_a_status_command_that_is_empty_or_has_shell_syntax_or_a_foreign_origin_is_red() {
    for cmd in ["\"\"", "\"   \""] {
        assert_red("bugfix", "is a non-empty command string", |t| {
            with_status(&t, cmd, LOCAL, T30)
        });
    }
    assert_red("bugfix", "runs only for runs of local origin", |t| {
        with_status(&t, "scripts/check-x", "", T30)
    });
    assert_red("bugfix", "runs only for runs of local origin", |t| {
        with_status(
            &t,
            "scripts/check-x",
            "        freesynergy.net:origin: remote\n",
            T30,
        )
    });
    for cmd in [
        "scripts/check-x; rm",
        "scripts/check-x | tee",
        "scripts/check-x $(id)",
        "scripts/check-x > out",
        "\"scripts/check-x\\nrm\"",
        "\"scripts/check-x\\rrm\"",
        "scripts/check-x `id`",
        "scripts/check-x && id",
    ] {
        assert_red("bugfix", "contains shell syntax", |t| {
            with_status(&t, cmd, LOCAL, T30)
        });
    }
}

#[test]
fn mutation_a_shell_or_an_interpreter_is_no_status_command() {
    for cmd in ["bash scripts/check-x", "/bin/sh -c x", "env cargo test"] {
        assert_red("bugfix", "a shell is no status command", |t| {
            with_status(&t, cmd, LOCAL, T30)
        });
    }
    assert_red("bugfix", "a shell is no status command", |t| {
        with_status(
            &t,
            "cargo test",
            &format!("{LOCAL}        freesynergy.net:allow: [cargo, bash]\n"),
            T30,
        )
    });
    for cmd in [
        "perl -e1",
        "python3 x.py",
        "/usr/bin/python3.11 x.py",
        "node x.js",
        "ruby x.rb",
        "awk 1",
        "find . -x",
        "xargs ls",
        "sudo ls",
    ] {
        assert_red("bugfix", "an interpreter is no status command", |t| {
            with_status(&t, cmd, LOCAL, T30)
        });
    }
    for ip in ["perl", "python3", "node", "ruby", "awk", "find", "xargs"] {
        assert_red("bugfix", "an interpreter is no status command", |t| {
            with_status(
                &t,
                "scripts/check-x",
                &format!("{LOCAL}        freesynergy.net:allow: [{ip}]\n"),
                T30,
            )
        });
    }
}

#[test]
fn mutation_a_status_command_outside_scripts_and_the_allow_list_is_red() {
    for cmd in ["cargo test", "scripts/../evil", "tools/check-x"] {
        assert_red("bugfix", "is neither a script under scripts/ nor in", |t| {
            with_status(&t, cmd, LOCAL, T30)
        });
    }
    assert_red("bugfix", "is a list of program names", |t| {
        with_status(
            &t,
            "cargo test",
            &format!("{LOCAL}        freesynergy.net:allow: cargo\n"),
            T30,
        )
    });
}
