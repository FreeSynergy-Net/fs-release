#!/usr/bin/env python3
"""Pattern: none. Allowlist check of a site folder (input of site-bauen.sh; the folder is a state a person has approved).

usage: python3 -I site-pruefen.py [--passthrough] <folder>     exit 0 = folder ok, 1 = every problem named on stderr (`path: reason`)

Mode static (default): ALLOWLIST, not a blocklist (Zentinel 26.11_2 sends no CSP on static routes, so this check is the only wall). File types by ending:
  html css txt xml json png jpg jpeg gif webp avif ico woff2 webmanifest toml (no svg, no js).
  html: strict tag grammar (every `<` starts a plain tag, double-quoted attribute values, no comment/CDATA/declaration), only the tags and attributes
    the exporter fs-text-html writes (no svg, math, style, noscript, template, script, frame ...), all URL attributes relative (a href also http, https, mailto,
    tel; link canonical/alternate also http, https), no backslash/space/control character in an address, no style attribute, no inline style; and a
    PAGE-OWN Content-Security-Policy <meta> (default-src 'self' or 'none', values only 'self'/'none') as the first element of <head> after <meta charset>.
  xml: sitemaps only (urlset, sitemapindex). css: a fixed style sheet: no url(), no @-rule but @media, no backslash (CSS escapes), no image-set.
Mode --passthrough (a person's own HTML, possibly with JavaScript, is passed through): the same file types plus js and mjs, no content rules for
  html/css/js/svg, but a log line per file (stdout, `log: <path> sha256 size [script] [external]`) so the passing is on record; the Content-Security-
  Policy of the web server (templates/site, CSP_MODE) is the second wall.
Both modes: index.html and a valid site.toml at the root (the ONE place of the host names, scripts/site_manifest.py; its csp.mode must equal the mode of this check); only regular files and folders; names of `A-Za-z0-9._-` and `/`; NO dot file or dot folder (.git, .env, ...);
  no set-uid/set-gid/sticky bit; limits on count and size; nothing that looks like a secret in any file.
"""
import hashlib
import html
import os
import re
import stat
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import site_manifest  # noqa: E402

STATIC_EXT = {"html", "css", "txt", "xml", "json", "png", "jpg", "jpeg", "gif", "webp", "avif", "ico", "woff2", "webmanifest", "toml"}
PASS_EXT = STATIC_EXT | {"js", "mjs", "svg"}  # svg runs script when opened on its own: only in the explicit passthrough mode
PARSED = {"html", "svg", "xml"}
MAX_FILES, MAX_BYTES = 5000, 200 * 1024 * 1024
COMP = re.compile(r"^[A-Za-z0-9_-][A-Za-z0-9._-]*$")
SECRET_STRICT = re.compile(rb"BEGIN [A-Z ]*PRIVATE KEY|ghp_[A-Za-z0-9]{20,}|github_pat_|AKIA[0-9A-Z]{16}")
SECRET_TEXT = re.compile(rb"(password|passwort|secret|token)[ \t]*[=:]", re.I)

# --- mode static: ALLOWLIST (what the exporter fs-text-html writes, fs-text-html/DESIGN.md "Rules of the export"). Zentinel 26.11_2 sends no CSP on
# static routes, so this check is the only wall: whatever is not listed here is refused, nothing is "parsed and judged".
HTML_TAGS = {"html", "head", "meta", "title", "link", "body", "a", "header", "nav", "main", "footer", "div", "section", "aside", "p", "span", "ul", "ol",
             "li", "dl", "dt", "dd", "h1", "h2", "h3", "h4", "h5", "h6", "pre", "code", "table", "thead", "tbody", "tr", "th", "td", "blockquote",
             "em", "strong", "del", "hr", "br", "img", "sup", "sub", "kbd", "u", "mark", "details", "summary", "input", "label"}
# NOT in the list on purpose: svg math style script noscript template xmp plaintext textarea iframe object embed base form button select audio video source
# (raw-text, foreign-content and plug-in elements: where a parser and a browser read the same bytes differently, or where script or a fetch starts).
HTML_VOID = {"meta", "link", "hr", "br", "img", "input"}
GLOBAL_ATTRS = {"class", "id", "lang", "title", "role", "dir", "aria-label", "aria-current", "aria-hidden", "aria-describedby", "aria-labelledby"}
TAG_ATTRS = {"html": {"lang"}, "meta": {"charset", "name", "content", "http-equiv"}, "link": {"rel", "href", "hreflang"},
             "a": {"href", "hreflang"}, "img": {"src", "alt", "width", "height", "loading"}, "input": {"type", "disabled", "checked"},
             "ol": {"start"}, "th": {"scope"}, "td": {"scope"}, "details": {"open"}, "code": set(), "li": set()}
IDENT = re.compile(r"^[A-Za-z0-9_:. -]{0,200}$")
LANG = re.compile(r"^[A-Za-z0-9-]{1,35}$")
NUM = re.compile(r"^[0-9]{1,6}$")
LINK_RELS = {"stylesheet", "canonical", "alternate", "icon"}
CSP_DIRECTIVES = {"default-src", "script-src", "style-src", "img-src", "font-src", "connect-src", "object-src", "base-uri", "form-action", "media-src",
                  "manifest-src", "frame-src", "child-src", "worker-src"}
SCHEME = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")
WS = "[ \t\n\r\f]"
TAG = re.compile(rf"<(/?)([A-Za-z][A-Za-z0-9:_-]*)((?:{WS}+[A-Za-z][A-Za-z0-9:_-]*(?:{WS}*={WS}*\"[^\"<>]*\")?)*){WS}*(/?)>")
ATTR = re.compile(rf"({WS}+)([A-Za-z][A-Za-z0-9:_-]*)(?:{WS}*={WS}*\"([^\"<>]*)\")?")
CTRL = re.compile(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]")
DOCTYPE = re.compile(r"<!DOCTYPE html>", re.I)
XML_TAGS = {"urlset", "url", "loc", "lastmod", "changefreq", "priority", "sitemapindex", "sitemap", "xhtml:link"}
XML_NS = "http://www.sitemaps.org/schemas/sitemap/0.9"
XHTML_NS = "http://www.w3.org/1999/xhtml"
XML_DECL = re.compile(r'<\?xml version="1\.0" encoding="UTF-8"\?>')
CSS_BAD = ("url(", "@import", "expression(", "javascript:", "behavior", "-moz-binding", "image-set(", "image(", "src(", "element(", "paint(", "cross-fade(",
           "<", "\\", "\x00")


def url_bad(value: str, how: str) -> str | None:
    """Reason why a URL value is refused, else None. how: `rel` (relative only), `link` (relative, http, https, mailto, tel), `page` (relative, http, https)."""
    if not value:
        return "empty address"
    if "\\" in value or CTRL.search(value) or re.search(r"\s", value):
        return "backslash, space or control character in an address"
    if value.startswith("//"):
        return "external resource (//)"
    m = SCHEME.match(value)
    if m is None:
        return None  # relative
    sch = m.group(0)[:-1].lower()
    if how == "rel":
        return f"{sch}: not a relative address"
    if sch in ("http", "https") or (how == "link" and sch in ("mailto", "tel")):
        return None
    return f"{sch}: URL is not allowed here"


def csp_bad(content: str) -> str | None:
    """The page's own policy must be strict: default-src 'self' or 'none', every value only 'self' or 'none', known directives only."""
    seen: dict[str, list[str]] = {}
    for part in content.split(";"):
        toks = part.split()
        if not toks:
            continue
        if toks[0].lower() not in CSP_DIRECTIVES or toks[0].lower() in seen:
            return f"policy directive {toks[0]!r} unknown or twice"
        if not toks[1:] or any(t not in ("'self'", "'none'") for t in toks[1:]):
            return f"policy value of {toks[0]} must be 'self' or 'none' only"
        seen[toks[0].lower()] = toks[1:]
    if "default-src" not in seen:
        return "policy has no default-src"
    return None


def tokens(text: str, xml: bool):
    """Yield ('tag', name, attrs, closing, selfclose) for every `<...>` and raise ValueError at the first one that is not a plain, well-formed tag.
    Every `<` must start a tag of the strict grammar (quoted values, no `<` `>` inside values); nothing else (comment, CDATA, PI, declaration) passes."""
    i, first = 0, True
    while True:
        j = text.find("<", i)
        if j < 0:
            return
        if first:
            first = False
            if xml and XML_DECL.match(text, j):
                i = XML_DECL.match(text, j).end()
                first = False
                continue
            if not xml and DOCTYPE.match(text, j):
                i = DOCTYPE.match(text, j).end()
                continue
        m = TAG.match(text, j)
        if m is None:
            raise ValueError(f"markup near byte {j} is not a plain tag (comment, CDATA, declaration, unquoted or odd attribute, or a stray `<`): {text[j:j + 40]!r}")
        name = m.group(2) if xml else m.group(2).lower()
        attrs: dict[str, str | None] = {}
        for am in ATTR.finditer(m.group(3)):
            k = am.group(2) if xml else am.group(2).lower()
            if k in attrs:
                raise ValueError(f"<{name}> has the attribute {k} twice")
            attrs[k] = None if am.group(3) is None else html.unescape(am.group(3))
        yield name, attrs, bool(m.group(1)), bool(m.group(4))
        i = m.end()


def check_html(text: str) -> list[str]:
    bad: list[str] = []
    if CTRL.search(text):
        return ["control character in the text"]
    if not DOCTYPE.match(text.lstrip("\ufeff")):
        bad.append("does not start with <!DOCTYPE html>")
    starts: list[tuple[str, dict]] = []
    try:
        for name, attrs, closing, selfclose in tokens(text.lstrip("\ufeff"), False):
            if name not in HTML_TAGS:
                bad.append(f"<{'/' if closing else ''}{name}> is not on the allowlist")
                continue
            if closing:
                if attrs or name in HTML_VOID:
                    bad.append(f"</{name}> with attributes or on an element without end tag")
                continue
            starts.append((name, attrs))
            ok_attrs = GLOBAL_ATTRS | TAG_ATTRS.get(name, set())
            for k, v in attrs.items():
                if k not in ok_attrs:
                    bad.append(f"<{name} {k}=...>: attribute is not on the allowlist")
                    continue
                if v is None:
                    if k not in ("disabled", "checked", "open"):
                        bad.append(f"<{name} {k}> needs a value")
                    continue
                if CTRL.search(v):
                    bad.append(f"<{name} {k}=...>: control character")
                if k in ("class", "id", "role", "aria-current", "aria-hidden", "aria-describedby", "aria-labelledby") and not IDENT.match(v):
                    bad.append(f"<{name} {k}=...>: not a plain identifier")
                if k in ("lang", "hreflang", "dir") and not LANG.match(v):
                    bad.append(f"<{name} {k}=...>: not a plain value")
                if k in ("width", "height", "start") and not NUM.match(v):
                    bad.append(f"<{name} {k}=...>: not a number")
            bad += check_tag(name, attrs)
    except ValueError as e:
        bad.append(str(e))
    bad += check_head(starts)
    return bad


def check_tag(name: str, a: dict) -> list[str]:
    out: list[str] = []
    def u(k: str, how: str) -> None:
        if a.get(k) is not None:
            r = url_bad(a[k], how)
            if r:
                out.append(f"<{name} {k}=...>: {r}")
    if name == "a":
        u("href", "link")
    elif name == "img":
        u("src", "rel")
        if a.get("loading") not in (None, "lazy", "eager"):
            out.append("<img loading=...>: lazy or eager only")
    elif name == "link":
        rel = (a.get("rel") or "").lower()
        if rel not in LINK_RELS:
            out.append(f"<link rel={rel!r}>: rel must be one of {sorted(LINK_RELS)}")
        elif "href" not in a:
            out.append("<link> without href")
        else:
            u("href", "page" if rel in ("canonical", "alternate") else "rel")
    elif name == "meta":
        he, nm = (a.get("http-equiv") or "").lower(), a.get("name")
        if "charset" in a:
            if set(a) != {"charset"} or (a["charset"] or "").lower() != "utf-8":
                out.append('<meta charset> must be exactly charset="utf-8"')
        elif he:
            if he != "content-security-policy" or set(a) != {"http-equiv", "content"}:
                out.append("<meta http-equiv> is only allowed for Content-Security-Policy (no refresh, no set-cookie)")
            else:
                r = csp_bad(a["content"] or "")
                if r:
                    out.append(f"<meta CSP>: {r}")
        elif nm in ("viewport", "description") and set(a) == {"name", "content"}:
            pass
        else:
            out.append("<meta> is only allowed as charset, viewport, description or the Content-Security-Policy")
    elif name == "input" and (a.get("type") != "checkbox" or "disabled" not in a):
        out.append('<input> is only allowed as type="checkbox" disabled')
    return out


def check_head(starts: list[tuple[str, dict]]) -> list[str]:
    """A page must open with <html>, <head>, optionally <meta charset>, and THEN its own Content-Security-Policy <meta>: before anything that loads."""
    names = [n for n, _ in starts]
    for once in ("html", "head", "body", "title"):
        if names.count(once) > 1:
            return [f"<{once}> twice"]
    if names[:2] != ["html", "head"]:
        return ["a page must start with <html><head> (no content before the head)"]
    rest = starts[2:]
    if rest and rest[0][0] == "meta" and "charset" in rest[0][1]:
        rest = rest[1:]
    if not rest or rest[0][0] != "meta" or (rest[0][1].get("http-equiv") or "").lower() != "content-security-policy":
        return ["no Content-Security-Policy <meta> as the first element of <head> (only <meta charset> may come before it; Zentinel 26.11_2 sends no CSP header)"]
    return []


def check_xml(text: str) -> list[str]:
    """Only sitemaps (urlset/sitemapindex): a browser shows application/xml with an XHTML root as a page, so no other root is accepted."""
    bad: list[str] = []
    if CTRL.search(text):
        return ["control character in the text"]
    if not XML_DECL.match(text.lstrip("\ufeff")):
        bad.append('must start with <?xml version="1.0" encoding="UTF-8"?>')
    first = True
    try:
        for name, attrs, closing, _ in tokens(text.lstrip("\ufeff"), True):
            if name not in XML_TAGS:
                bad.append(f"<{name}> is not allowed in xml (sitemaps only)")
                continue
            if first:
                first = False
                if name not in ("urlset", "sitemapindex"):
                    bad.append("the root must be urlset or sitemapindex")
            if closing:
                continue
            for k, v in attrs.items():
                if name in ("urlset", "sitemapindex") and ((k == "xmlns" and v == XML_NS) or (k == "xmlns:xhtml" and v == XHTML_NS)):
                    continue
                if name == "xhtml:link" and k in ("rel", "hreflang", "href") and v is not None:
                    r = url_bad(v, "page") if k == "href" else None
                    if r:
                        bad.append(f"<xhtml:link href>: {r}")
                    continue
                bad.append(f"<{name} {k}=...>: attribute is not allowed in xml")
    except ValueError as e:
        bad.append(str(e))
    return bad


def css_bad(css: str) -> list[str]:
    """A fixed style sheet only: no url(), no @-rule but @media, no backslash (CSS escapes hide `url(`, `@import`), no comment tricks."""
    low = css.lower()
    out = [f"css: `{t.strip()}` is not allowed" for t in CSS_BAD if t in low]
    out += [f"css: @{m} is not allowed (only @media)" for m in dict.fromkeys(re.findall(r"@([a-z-]+)", low)) if m != "media"]
    return out


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
        if ext == "html":
            try:
                text = data.decode("utf-8")
            except UnicodeDecodeError:
                bad.append(f"{rel}: not valid UTF-8")
                continue
            bad += [f"{rel}: {b}" for b in check_html(text)]
        elif ext == "xml":
            try:
                text = data.decode("utf-8")
            except UnicodeDecodeError:
                bad.append(f"{rel}: not valid UTF-8")
                continue
            bad += [f"{rel}: {b}" for b in check_xml(text)]
        elif ext == "css":
            try:
                bad += [f"{rel}: {b}" for b in css_bad(data.decode("utf-8"))]
            except UnicodeDecodeError:
                bad.append(f"{rel}: not valid UTF-8")
    for b in dict.fromkeys(bad):
        print(b, file=sys.stderr)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
