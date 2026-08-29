#!/usr/bin/env python3
"""Documentation build script for The Machine.

Assembles the architecture/design specs and auto-generates an implementation
reference (module / class / function / test inventory) from the actual source
code, then renders a single self-contained HTML book and a concatenated Markdown
file.

No external tooling required beyond the Python standard library and the
`markdown` package (already available in the project virtualenv).

Usage:
    python3 docs/build.py            # build into docs/build/
    python3 docs/build.py --clean    # remove docs/build/ first
"""

from __future__ import annotations

import argparse
import ast
import os
import re
import shutil
import sys

try:
    import markdown
    from markdown.extensions.toc import TocExtension
except ImportError:
    sys.exit("The 'markdown' package is required: uv pip install markdown")

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BUILD_DIR = os.path.join(ROOT, "docs", "build")
BOOK_DIR = os.path.join(ROOT, "docs", "book")

# (source markdown, component_dir_or_None)
# component_dir is scanned for an auto-generated implementation reference.
SOURCES = [
    ("docs/book/index.md", None),
    ("docs/spec.md", None),  # top-level architecture definition
    ("lambda-server/docs/spec.md", "lambda-server"),
    ("state-store/docs/spec.md", "state-store"),
    ("event-bus/docs/spec.md", "event-bus"),
    ("policy-broker/docs/spec.md", "policy-broker"),
    ("docs/agent-core-spec.md", "agent-core"),
    ("local-model/docs/spec.md", "local-model"),
    ("ui-engine/docs/spec.md", "ui-engine"),
    ("docs/book/ui-engine-demo.md", "ui-engine-demo"),
    ("docs/mcp-bus-spec.md", None),
    ("docs/system-daemon-spec.md", None),
    ("docs/fallback-shell-spec.md", None),
    ("docs/compositor-spec.md", None),
]

IGNORE_DIRS = {"__pycache__", ".venv", ".pytest_cache", "node_modules", "build", "dist"}


def scan_component(component_dir: str):
    """Return (modules, test_count) for a component source tree."""
    base = os.path.join(ROOT, component_dir)
    if not os.path.isdir(base):
        return {}, 0

    modules: dict[str, tuple[list, list]] = {}
    test_count = 0

    for dirpath, dirs, files in os.walk(base):
        dirs[:] = [d for d in dirs if d not in IGNORE_DIRS]
        for fname in files:
            if not fname.endswith(".py"):
                continue
            path = os.path.join(dirpath, fname)
            rel = os.path.relpath(path, base)
            try:
                with open(path, encoding="utf-8") as fh:
                    tree = ast.parse(fh.read())
            except (OSError, SyntaxError):
                continue

            is_test = fname.startswith("test_") or rel.startswith("tests") or "/tests/" in rel
            if is_test:
                test_count += sum(
                    1
                    for n in ast.walk(tree)
                    if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef))
                    and n.name.startswith("test")
                )
                continue

            classes: list[tuple[str, list[str], list[str]]] = []
            funcs: list[str] = []
            for node in tree.body:
                if isinstance(node, ast.ClassDef):
                    methods = [
                        m.name
                        for m in node.body
                        if isinstance(m, (ast.FunctionDef, ast.AsyncFunctionDef))
                        and not m.name.startswith("_")
                    ]
                    bases = [ast.unparse(b) for b in node.bases]
                    classes.append((node.name, bases, methods))
                elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and not node.name.startswith("_"):
                    funcs.append(node.name)
            if classes or funcs:
                modules[rel] = (classes, funcs)

    return modules, test_count


def render_impl_reference(component_dir: str) -> str:
    modules, test_count = scan_component(component_dir)
    base = os.path.join(ROOT, component_dir)
    if not os.path.isdir(base):
        return "\n\n### Implementation Status\n\n*Design draft — not yet implemented.*\n"

    lines = ["\n", "### Implementation Reference (auto-generated)", ""]
    if not modules:
        lines.append("*No Python modules scanned under `%s` yet (design draft).*" % component_dir)
    else:
        lines.append("Scanned `%s`. Module / public-symbol inventory:" % component_dir)
        lines.append("")
        for mod in sorted(modules):
            classes, funcs = modules[mod]
            lines.append("- **`%s`**" % mod)
            for cname, bases, methods in classes:
                base_str = "(%s)" % ", ".join(bases) if bases else ""
                lines.append("  - `class %s%s`" % (cname, base_str))
                for m in methods:
                    lines.append("    - `%s()`" % m)
            for fn in funcs:
                lines.append("  - `def %s()`" % fn)
        lines.append("")

    if test_count:
        lines.append("**Tests discovered:** %d `test_*` functions." % test_count)
    lines.append("")
    return "\n".join(lines)


def build(clean: bool) -> None:
    if clean and os.path.isdir(BUILD_DIR):
        shutil.rmtree(BUILD_DIR)
    os.makedirs(BUILD_DIR, exist_ok=True)

    parts: list[str] = []
    for src_rel, component_dir in SOURCES:
        src_path = os.path.join(ROOT, src_rel)
        if not os.path.isfile(src_path):
            print("! missing source:", src_rel, file=sys.stderr)
            continue
        with open(src_path, encoding="utf-8") as fh:
            text = fh.read().rstrip()
        if component_dir:
            text += render_impl_reference(component_dir)
        parts.append(text)
        print("+ included", src_rel)

    full_md = "\n\n---\n\n".join(parts)

    # Markdown -> HTML with TOC
    md = markdown.Markdown(
        extensions=[
            TocExtension(marker="[TOC]", baselevel=1),
            "fenced_code",
            "tables",
            "nl2br",
        ]
    )
    body_html = md.convert(full_md)
    toc_html = md.toc or ""

    page = HTML_TEMPLATE.replace("{{TOC}}", toc_html).replace("{{BODY}}", body_html)

    out_html = os.path.join(BUILD_DIR, "index.html")
    out_md = os.path.join(BUILD_DIR, "book.md")
    with open(out_html, "w", encoding="utf-8") as fh:
        fh.write(page)
    with open(out_md, "w", encoding="utf-8") as fh:
        fh.write(full_md)

    print("\nBuilt:")
    print("  ", out_html)
    print("  ", out_md)


HTML_TEMPLATE = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>The Machine — Documentation</title>
<style>
  :root { --fg:#1f2328; --muted:#57606a; --bg:#ffffff; --sidebar:#f6f8fa;
          --border:#d0d7de; --accent:#0969da; --code-bg:#f6f8fa; }
  * { box-sizing: border-box; }
  html, body { margin:0; padding:0; }
  body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
         color: var(--fg); background: var(--bg); line-height:1.6; }
  .layout { display:flex; min-height:100vh; }
  nav.toc { width: 280px; flex: 0 0 280px; position: sticky; top:0;
            align-self: flex-start; height:100vh; overflow-y:auto;
            background: var(--sidebar); border-right:1px solid var(--border); padding:1.2rem 1rem; }
  nav.toc ul { list-style:none; padding-left:0; margin:0; }
  nav.toc li { margin:0.15rem 0; }
  nav.toc a { color: var(--muted); text-decoration:none; font-size:0.9rem; }
  nav.toc a:hover { color: var(--accent); }
  nav.toc > ul > li > a { font-weight:600; color: var(--fg); }
  nav.toc ul ul { padding-left:0.9rem; }
  main { flex:1 1 auto; max-width: 880px; margin:0 auto; padding:2.5rem 3rem 6rem; }
  h1 { font-size:2rem; border-bottom:1px solid var(--border); padding-bottom:.3em; }
  h2 { font-size:1.5rem; margin-top:2.2rem; border-bottom:1px solid var(--border); padding-bottom:.2em; }
  h3 { font-size:1.2rem; margin-top:1.6rem; }
  h4 { font-size:1.05rem; }
  code { background: var(--code-bg); padding:0.15em 0.4em; border-radius:4px;
         font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size:0.88em; }
  pre { background: var(--code-bg); padding:1rem; border-radius:8px; overflow-x:auto;
        border:1px solid var(--border); }
  pre code { background:none; padding:0; }
  table { border-collapse: collapse; width:100%; margin:1rem 0; }
  th, td { border:1px solid var(--border); padding:0.5rem 0.7rem; text-align:left; }
  th { background: var(--sidebar); }
  blockquote { border-left:4px solid var(--border); margin:1rem 0; padding:0.2rem 1rem;
               color: var(--muted); }
  hr { border:none; border-top:1px solid var(--border); margin:2.5rem 0; }
  a { color: var(--accent); }
</style>
</head>
<body>
<div class="layout">
<nav class="toc">
  <strong>The Machine</strong>
  {{TOC}}
</nav>
<main>
{{BODY}}
</main>
</div>
</body>
</html>
"""


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Build The Machine documentation")
    parser.add_argument("--clean", action="store_true", help="remove docs/build before building")
    args = parser.parse_args()
    build(clean=args.clean)
