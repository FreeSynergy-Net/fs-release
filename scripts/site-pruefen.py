#!/usr/bin/env python3
"""Pattern: none. Allowlist check of a site folder (input of site-bauen.sh; the folder is a state a person has approved).

usage: python3 -I site-pruefen.py [--passthrough] <folder>     exit 0 = folder ok, 1 = every problem named on stderr (`path: reason`)

Mode static (default): only these file types (by ending): html css txt xml json svg png jpg jpeg gif webp avif ico woff2 webmanifest toml.
  html, svg, xml and css are parsed (not grepped): no script, no on* attribute, no frame/object/embed/base, no meta refresh, no javascript:/data:
  URL, every URL attribute (href src srcset action poster data xlink:href ...) relative, except `<a href>` with http, https, mailto or tel;
  css: no @import, every url() relative, no expression().
Mode --passthrough (a person's own HTML, possibly with JavaScript, is passed through): the same file types plus js and mjs, no content rules for
  html/css/js/svg, but a log line per file (stdout, `log: <path> sha256 size [script] [external]`) so the passing is on record; the Content-Security-
  Policy of the web server (templates/site, CSP_MODE) is the second wall.
Both modes: index.html and a valid site.toml at the root (the ONE place of the host names, scripts/site_manifest.py; its csp.mode must equal the mode of this check); only regular files and folders; names of `A-Za-z0-9._-` and `/`; NO dot file or dot folder (.git, .env, ...);
  no set-uid/set-gid/sticky bit; limits on count and size; nothing that looks like a secret in any file.
"""
import hashlib
import os
import re
import stat
import sys
from html.parser import HTMLParser

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import site_manifest  # noqa: E402

STATIC_EXT = {"html", "css", "txt", "xml", "json", "svg", "png", "jpg", "jpeg", "gif", "webp", "avif", "ico", "woff2", "webmanifest", "toml"}
PASS_EXT = STATIC_EXT | {"js", "mjs"}
PARSED = {"html", "svg", "xml"}
MAX_FILES, MAX_BYTES = 5000, 200 * 1024 * 1024
COMP = re.compile(r"^[A-Za-z0-9_-][A-Za-z0-9._-]*$")
BAD_TAGS = {"script", "iframe", "frame", "frameset", "object", "embed", "applet", "base", "foreignobject"}
URL_ATTRS = {"href", "src", "srcset", "action", "formaction", "poster", "data", "xlink:href", "background", "ping", "longdesc",
             "manifest", "cite", "codebase", "icon", "usemap", "profile", "archive", "lowsrc", "dynsrc"}
LINK_OK = ("http", "https", "mailto", "tel")
SECRET_STRICT = re.compile(rb"BEGIN [A-Z ]*PRIVATE KEY|ghp_[A-Za-z0-9]{20,}|github_pat_|AKIA[0-9A-Z]{16}")
SECRET_TEXT = re.compile(rb"(password|passwort|secret|token)[ \t]*[=:]", re.I)
BACKSTOP = re.compile(rb"<\s*(script|iframe|object|embed)|javascript\s*:|vbscript\s*:", re.I)
CSS_URL = re.compile(r"url\(\s*(['\"]?)(.*?)\1\s*\)", re.I | re.S)


def url_bad(value: str, tag: str, attr: str) -> str | None:
    """Reason why a URL value is not allowed in static mode, else None."""
    v = re.sub(r"[\x00-\x20\x7f]+", "", value).lower()
    if not v or v.startswith("#"):
        return None
    if v.startswith("//"):
        return "external resource (//)"
    m = re.match(r"^([a-z][a-z0-9+.-]*):", v)
    if m is None:
        return None  # relative
    if m.group(1) in ("javascript", "vbscript", "data"):
        return f"{m.group(1)}: URL"
    if tag == "a" and attr in ("href", "xlink:href") and m.group(1) in LINK_OK:
        return None
    return f"external resource ({m.group(1)}:)"


def css_bad(css: str) -> list[str]:
    out = []
    low = css.lower()
    if "@import" in low:
        out.append("css @import")
    if "expression(" in low or "javascript:" in low or "behavior:" in low or "-moz-binding" in low:
        out.append("css expression/javascript/behavior")
    for _, target in CSS_URL.findall(css):
        r = url_bad(target, "css", "url")
        if r:
            out.append(f"css url(): {r}")
    if re.search(r"image-set\(\s*['\"]", low):
        out.append("css image-set() with a string URL")
    return out


class Page(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.bad: list[str] = []
        self.in_style = False
        self.style_text: list[str] = []

    def handle_starttag(self, tag, attrs):
        tag = tag.lower()
        if tag in BAD_TAGS:
            self.bad.append(f"<{tag}> is not allowed")
        if tag == "style":
            self.in_style = True
        d = {(k or "").lower(): (v or "") for k, v in attrs}
        for k, v in d.items():
            if k.startswith("on"):
                self.bad.append(f"event attribute {k}=")
            if k == "style":
                self.bad += css_bad(v)
            if k in URL_ATTRS:
                parts = [p.strip().split(" ")[0] for p in v.split(",")] if k == "srcset" else [v]
                for p in parts:
                    r = url_bad(p, tag, k)
                    if r:
                        self.bad.append(f"<{tag} {k}=...>: {r}")
        if tag == "meta" and d.get("http-equiv", "").strip().lower() == "refresh":
            self.bad.append("<meta http-equiv=refresh>")

    handle_startendtag = handle_starttag

    def handle_endtag(self, tag):
        if tag.lower() == "style":
            self.in_style = False
            self.bad += css_bad("".join(self.style_text))
            self.style_text = []

    def handle_data(self, data):
        if self.in_style:
            self.style_text.append(data)


def main(argv: list[str]) -> int:
    passthrough = "--passthrough" in argv
    args = [a for a in argv if a != "--passthrough"]
    if len(args) != 1:
        print("usage: site-pruefen.py [--passthrough] <folder>", file=sys.stderr)
        return 2
    src = args[0]
    ok_ext = PASS_EXT if passthrough else STATIC_EXT
    bad: list[str] = []
    if not os.path.isdir(src) or os.path.islink(src):
        print(f"{src}: not a directory", file=sys.stderr)
        return 1
    files: list[str] = []
    total = 0
    for root, dirs, names in os.walk(src, followlinks=False):
        for n in list(dirs) + names:
            p = os.path.join(root, n)
            rel = os.path.relpath(p, src)
            st = os.lstat(p)
            if not (stat.S_ISREG(st.st_mode) or stat.S_ISDIR(st.st_mode)):
                bad.append(f"{rel}: not a regular file or folder (symlink, device, fifo)")
                continue
            if st.st_mode & 0o7000:
                bad.append(f"{rel}: set-uid, set-gid or sticky bit")
            if not COMP.match(n):
                bad.append(f"{rel}: unsafe name (dot file or character outside A-Za-z0-9._-)")
            if stat.S_ISREG(st.st_mode):
                files.append(rel)
                total += st.st_size
    if not os.path.isfile(os.path.join(src, "index.html")):
        bad.append("no index.html at the root")
    if not os.path.isfile(os.path.join(src, "site.toml")):
        bad.append("no site.toml at the root")
    else:
        try:
            man = site_manifest.load(os.path.join(src, "site.toml"))
            if man["csp_mode"] != ("passthrough" if passthrough else "static"):
                bad.append(f"site.toml: csp.mode is {man['csp_mode']} but this check runs in mode {'passthrough' if passthrough else 'static'}")
        except (ValueError, OSError) as e:
            bad.append(f"site.toml: {e}")
    if len(files) > MAX_FILES or total > MAX_BYTES:
        bad.append(f"too large ({len(files)} files, {total} bytes)")
    files.sort(key=lambda f: f.encode())
    for rel in files:
        p = os.path.join(src, rel)
        if not COMP.match(os.path.basename(rel)) or os.path.islink(p):
            continue
        ext = rel.rsplit(".", 1)[-1].lower() if "." in rel else ""
        if ext not in ok_ext:
            bad.append(f"{rel}: file type '.{ext}' is not on the allowlist")
            continue
        data = open(p, "rb").read()
        text_like = ext not in ("png", "jpg", "jpeg", "gif", "webp", "avif", "ico", "woff2")
        if SECRET_STRICT.search(data) or (text_like and SECRET_TEXT.search(data)):
            bad.append(f"{rel}: looks like a secret")
        script = bool(re.search(rb"<\s*script|\bon[a-z]+\s*=|javascript\s*:", data, re.I)) if ext in PARSED | {"js", "mjs"} else False
        external = bool(re.search(rb"(https?:)?//[A-Za-z0-9.-]+\.[a-z]{2,}", data)) if ext in PARSED | {"css", "js", "mjs"} else False
        if passthrough:
            flags = (" script" if script or ext in ("js", "mjs") else "") + (" external" if external else "")
            print(f"log: {rel} sha256={hashlib.sha256(data).hexdigest()} size={len(data)}{flags}")
            continue
        if ext in PARSED:
            if BACKSTOP.search(data):
                bad.append(f"{rel}: script, frame, object, embed or javascript: in the text")
            page = Page()
            try:
                page.feed(data.decode("utf-8", "replace"))
                page.close()
            except Exception as e:  # a page the parser cannot read is not accepted
                bad.append(f"{rel}: cannot be parsed ({e})")
            bad += [f"{rel}: {b}" for b in page.bad]
        elif ext == "css":
            bad += [f"{rel}: {b}" for b in css_bad(data.decode("utf-8", "replace"))]
    for b in dict.fromkeys(bad):
        print(b, file=sys.stderr)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
