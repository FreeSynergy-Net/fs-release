#!/usr/bin/env python3
"""Turn the caller's release inputs into one validated JSON plan (pattern: Facade over inputs).

All values arrive as environment variables (see action.yml). Output: `plan` and
`version` on $GITHUB_OUTPUT. Fails on a tag that is not `vMAJOR.MINOR.PATCH[-pre][+build]`. `kind: library` (with empty `binaries`) = library mode (`library: true`, no platform job; a binary plan has no `library` key, which counts as false).
"""
import json
import os
import pathlib
import re
import subprocess
import sys

SEMVER = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?"
    r"(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$"
)
LINUX = {
    "x86_64": ("ubuntu-24.04", "x86_64-unknown-linux-gnu"),
    "aarch64": ("ubuntu-24.04-arm", "aarch64-unknown-linux-gnu"),
}
WINDOWS = {
    "x86_64": ("windows-2022", "x86_64-pc-windows-msvc"),
    "aarch64": ("windows-11-arm", "aarch64-pc-windows-msvc"),
}
DEFAULT_FORMATS = {
    "linux": {"app": "tar.gz,deb,rpm,appimage,arch", "cli": "tar.gz,deb,rpm,arch", "daemon": "tar.gz,deb,rpm,arch", "module": "rpm", "data": "rpm"},
    # module/data: Linux RPM only. Windows (WiX template `modules\\<name>\\service.exe`, WIN-b) and macOS are not built yet: no job, never a silent half release.
    "windows": {"app": "zip,msi,exe", "cli": "zip,msi", "daemon": "zip,msi", "module": "", "data": ""},
    "macos": {"app": "dmg", "cli": "tar.gz", "daemon": "tar.gz", "module": "", "data": ""},
}
PACKAGE_KINDS = ("module", "data")  # built by `fs-blueprint package module|rpm`, not by the binary formats
MODULE_NAME = re.compile(r"^[a-z][a-z0-9]*(-[a-z0-9]+)*$")


def env(name, default=""):
    return os.environ.get(name, default).strip()


def csv(value):
    return [v.strip() for v in re.split(r"[,\s]+", value) if v.strip()]


def fail(msg):
    print(f"::error::{msg}")
    sys.exit(1)


def matrix(kind, wanted, table):
    out = []
    for arch in csv(wanted):
        if arch not in table:
            fail(f"unknown {kind} target '{arch}' (known: {', '.join(table)})")
        runner, triple = table[arch]
        out.append({"arch": arch, "runner": runner, "triple": triple})
    return out


def cargo_packages(workdir):
    raw = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=workdir, check=True, capture_output=True, text=True,
    ).stdout
    return json.loads(raw)["packages"]


def main():
    tag = env("TAG") or env("GITHUB_REF_NAME")
    if not tag.startswith("v") or not SEMVER.match(tag[1:]):
        fail(f"tag '{tag}' is not a SemVer release tag (expected vX.Y.Z[-pre][+build])")
    version = tag[1:]
    core = ".".join(version.split("-")[0].split("+")[0].split(".")[:3])

    kind = env("KIND", "cli")
    if kind != "library" and kind not in DEFAULT_FORMATS["linux"]:
        fail("kind must be cli, daemon, app, module, data or library")
    binaries = csv(env("BINARIES"))
    if kind == "data":
        # Data package (noarch RPM of `fs-blueprint package rpm scaffold.toml`): no binary, no cargo build.
        if binaries:
            fail("kind 'data' with a non-empty 'binaries': a data package has no binary (use kind 'module' for a module with a service)")
        name = env("NAME") or env("GITHUB_REPOSITORY").rsplit("/", 1)[-1]
        if not name:
            fail("kind 'data' needs the input 'name' (the program name)")
        plan = {
            "tag": tag, "version": version, "version_core": core, "prerelease": "-" in version, "kind": "data",
            "name": name, "binaries": [], "workdir": env("WORKDIR", "."), "blueprint": "scaffold.toml",
            "linux_formats": "rpm", "windows_formats": "", "macos_formats": "",
            # the RPM is noarch: ONE Linux job (x86_64) builds it, no aarch64 twin
            "linux_matrix": [{"arch": "x86_64", "runner": LINUX["x86_64"][0], "triple": LINUX["x86_64"][1]}], "windows_matrix": [],
            "has_linux": True, "has_windows": False, "has_macos": False, "has_arch": False,
        }
        # Decided HERE, long before the approval of the signing job: no SBOM for a data package (see README "SBOM per kind"), said out loud, never silent.
        print("::notice::kind data: no SBOM (a data package has no Rust code; cargo sbom would list crates that are not in the RPM, so none is made). The release notes say so.")
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as out:
            out.write(f"plan={json.dumps(plan, separators=(',', ':'))}\nversion={version}\n")
        print(json.dumps(plan, indent=2))
        return
    if kind == "module":
        # `name` is the HOST module name (planning), never the repo name; the RPM is fs-module-<name>. Exactly one service binary.
        if len(binaries) != 1:
            fail("kind 'module' needs exactly one binary (the service) in 'binaries'")
        if not MODULE_NAME.match(env("NAME")) or env("NAME").startswith("fs-"):
            fail("kind 'module' needs 'name' = the host module name (lowercase, e.g. planning, no 'fs-' prefix)")
    if kind == "library" and binaries:
        fail("kind 'library' with a non-empty 'binaries': a library has no binaries to package (remove one of the two)")
    if kind != "library" and not binaries:
        # A forgotten or empty `binaries` of an app repo must not turn into a release without packages: library mode is asked for explicitly.
        fail("input 'binaries' is empty (a pure library must ask for it with `kind: library`)")
    if kind == "library":
        # Library mode: nothing to build, package or sign; the plan only carries the tag and every platform job stays off.
        # (tag = Cargo version = non-empty CHANGELOG section is checked by actions/library-release/notes.py in the plan action.)
        name = env("NAME") or env("GITHUB_REPOSITORY").rsplit("/", 1)[-1] or "library"
        plan = {
            "tag": tag, "version": version, "version_core": core, "prerelease": "-" in version, "library": True,
            "name": name, "binaries": [], "workdir": env("WORKDIR", "."),
            "has_linux": False, "has_windows": False, "has_macos": False, "has_arch": False,
        }
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as out:
            out.write(f"plan={json.dumps(plan, separators=(',', ':'))}\nversion={version}\n")
        print(json.dumps(plan, indent=2))
        return

    workdir = env("WORKDIR", ".")
    pkgs = cargo_packages(workdir)
    wanted = env("CARGO_PACKAGE")
    chosen = [p for p in pkgs if not wanted or p["name"] == wanted]
    if not chosen:
        fail(f"cargo package '{wanted}' not found")
    owners = [p for p in chosen if any(t["name"] in binaries and "bin" in t["kind"] for t in p["targets"])]
    if not owners:
        fail(f"no cargo package provides the binaries {binaries}")
    # Binaries that cargo-package does not provide belong to other packages of the workspace (one deb/rpm per package).
    have = {t["name"] for p in owners for t in p["targets"] if "bin" in t["kind"]}
    for b in binaries:
        if b in have:
            continue
        extra = [p for p in pkgs if any(t["name"] == b and "bin" in t["kind"] for t in p["targets"])]
        if not extra:
            fail(f"no cargo package provides the binary '{b}'")
        owners += [p for p in extra[:1] if p not in owners]
        have.add(b)
    # A binary under crates/ that is not in `binaries` would silently never be packaged: fail unless explicitly ignored.
    ignore = set(csv(env("IGNORE_BINARIES")))
    if "*" not in ignore:
        root = pathlib.Path(json.loads(subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps"],
            cwd=workdir, check=True, capture_output=True, text=True).stdout)["workspace_root"])
        loose = sorted(
            f"{t['name']} ({p['name']})" for p in pkgs for t in p["targets"]
            if "bin" in t["kind"] and t["name"] not in binaries and t["name"] not in ignore
            and "crates" in pathlib.Path(p["manifest_path"]).relative_to(root).parts[:-1]
        )
        if loose:
            fail(f"binaries under crates/ are not packaged: {', '.join(loose)}. Add them to 'binaries' "
                 "or list them in 'ignore-binaries' (or '*') to skip them on purpose")
    # Packaging tools (cargo-deb, cargo-wix, rpm) need these; fail here, not after 20 minutes of builds.
    for p in owners:
        missing = [k for k in ("authors", "description", "license", "repository") if not p.get(k)]
        if missing:
            fail(f"{p['name']}: Cargo.toml lacks {', '.join(missing)} (needed for deb/rpm/msi metadata)")
    if not any(p["version"] == version for p in owners):
        fail(f"tag version {version} differs from Cargo.toml ({', '.join(p['version'] for p in owners)})")
    owner = owners[0]

    name = env("NAME") or owner["name"]
    formats = {
        os_: env(f"{os_.upper()}_FORMATS") or DEFAULT_FORMATS[os_][kind] for os_ in DEFAULT_FORMATS
    }
    linux, windows = matrix("linux", env("LINUX_TARGETS"), LINUX), matrix("windows", env("WINDOWS_TARGETS"), WINDOWS)
    if kind == "module":
        windows = []  # TODO WIN-b: WiX template, MSI per machine into the anchor; until then no Windows job
        print("::warning::kind module: no Windows package yet (not built, WIN-b); windows-targets is ignored")
    plan = {
        "tag": tag, "version": version, "version_core": core, "prerelease": "-" in version,
        "name": name, "binaries": binaries, "cargo_package": wanted, "workdir": workdir,
        "features": env("FEATURES"), "kind": kind,
        "app_id": env("APP_ID"), "app_name": env("APP_NAME") or name,
        "description": env("DESCRIPTION") or owner.get("description") or name,
        "license": owner.get("license") or "MIT OR Apache-2.0",
        "homepage": owner.get("homepage") or owner.get("repository") or "",
        "linux_formats": formats["linux"], "windows_formats": formats["windows"],
        "macos_formats": formats["macos"],
        "linux_matrix": linux, "windows_matrix": windows,
        "has_linux": bool(linux), "has_windows": bool(windows),
        "has_macos": env("MACOS") == "true" and kind not in PACKAGE_KINDS,
        "has_arch": bool(linux) and "arch" in formats["linux"] and any(m["arch"] == "x86_64" for m in linux),
    }
    if plan["has_arch"] and "tar.gz" not in formats["linux"]:
        fail("the 'arch' format is built from the linux tar.gz; add tar.gz to linux-formats")
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as out:
        out.write(f"plan={json.dumps(plan, separators=(',', ':'))}\nversion={version}\n")
    print(json.dumps(plan, indent=2))


if __name__ == "__main__":
    main()
