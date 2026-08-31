#!/usr/bin/env python3
"""Parse and re-tally docs/architecture/agentic-desktop-scenarios.md.

Used by the scenario audit workflow: `list` prints scenarios filtered by status
or keyword, `tally` recomputes the summary table, and `set` flips statuses for a
named set of ids once the code behind them exists.
"""
import re
import sys
from collections import Counter, OrderedDict

DOC = "docs/architecture/agentic-desktop-scenarios.md"

FIELDS = ("Perspective", "Goal", "Interaction", "Expected", "Status")


def parse(path=DOC):
    text = open(path, encoding="utf-8").read()
    blocks = re.split(r"^### (S\d{3,4})\s*$", text, flags=re.M)
    head = blocks[0]
    out = OrderedDict()
    for i in range(1, len(blocks), 2):
        sid = blocks[i]
        body = blocks[i + 1]
        rec = {"id": sid, "raw": body}
        for f in FIELDS:
            m = re.search(rf"^- \*\*{f}:\*\* (.*)$", body, flags=re.M)
            rec[f.lower()] = m.group(1).strip() if m else ""
        out[sid] = rec
    return head, out


def tally(recs):
    return Counter(r["status"] for r in recs.values())


def set_status(ids, status, path=DOC):
    text = open(path, encoding="utf-8").read()
    want = set(ids)
    changed = []

    def repl(m):
        sid, body = m.group(1), m.group(2)
        if sid not in want:
            return m.group(0)
        new, n = re.subn(
            r"^- \*\*Status:\*\* .*$",
            f"- **Status:** {status}",
            body,
            flags=re.M,
        )
        if n:
            changed.append(sid)
        return f"### {sid}\n{new}"

    text = re.sub(r"### (S\d{3,4})\n(.*?)(?=\n### S|\Z)", repl, text, flags=re.S)
    open(path, "w", encoding="utf-8").write(text)
    return changed


def set_field(sid, field, value, path=DOC):
    text = open(path, encoding="utf-8").read()

    def repl(m):
        if m.group(1) != sid:
            return m.group(0)
        body = re.sub(
            rf"^- \*\*{field}:\*\* .*$",
            f"- **{field}:** {value}",
            m.group(2),
            flags=re.M,
        )
        return f"### {sid}\n{body}"

    text = re.sub(r"### (S\d{3,4})\n(.*?)(?=\n### S|\Z)", repl, text, flags=re.S)
    open(path, "w", encoding="utf-8").write(text)


def rewrite_summary(path=DOC):
    _, recs = parse(path)
    counts = tally(recs)
    text = open(path, encoding="utf-8").read()
    table = (
        "| Status | Count |\n|---|---|\n"
        f"| NOW | {counts.get('NOW', 0)} |\n"
        f"| PARTIAL | {counts.get('PARTIAL', 0)} |\n"
        f"| GAP | {counts.get('GAP', 0)} |\n"
        f"| **Total** | **{sum(counts.values())}** |\n"
    )
    text = re.sub(
        r"\| Status \| Count \|\n\|---\|---\|\n\| NOW \| \d+ \|\n\| PARTIAL \| \d+ \|\n\| GAP \| \d+ \|\n\| \*\*Total\*\* \| \*\*\d+\*\* \|\n",
        table,
        text,
        count=1,
    )
    open(path, "w", encoding="utf-8").write(text)
    return counts


THEMES = [
    ("chat", r"chat|turn|transcript|conversation|reply|message|suggest|voice|mic|dictat|attach"),
    ("spawn", r"spawn|workspace|primitive|toggle|slider|media|chart|icon|grid|stack|text|field|button|list|dialog|place|clear|replace"),
    ("plans", r"plan|multi-step|step|llm|model|cloud|heuristic|classif|intent"),
    ("policy", r"policy|broker|deny|allow|confirm|consent|E_DENIED|fail-closed|audit"),
    ("system", r"system-daemon|display|brightness|network|wifi|audio|volume|power|battery|suspend|clipboard|mount|thermal"),
    ("a11y", r"a11y|accessib|AT-SPI|atspi|screen reader|announce|live region|contrast|focus order|role|aria"),
    ("i18n", r"i18n|locale|rtl|translat|language|catalog|bidi"),
    ("keyboard", r"key|shortcut|chord|tab|caret|selection|undo|redo|paste|copy|cut|ime|compose|dead key"),
    ("wayland", r"wayland|xdg|surface|compositor|xwayland|wlroots|damage|frame|output|dmabuf|seat"),
    ("errors", r"error|fail|timeout|retry|crash|restart|degrade|offline|unavailable|fault"),
    ("boot", r"boot|initramfs|first-run|install|session start|greet"),
    ("pointer", r"pointer|mouse|click|hover|drag|wheel|scroll|touch"),
]


def theme_of(rec):
    hay = " ".join(
        [rec["goal"], rec["interaction"], rec["expected"], rec["perspective"]]
    ).lower()
    for name, pat in THEMES:
        if re.search(pat, hay):
            return name
    return "other"


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else "tally"
    _, recs = parse()
    if cmd == "tally":
        c = tally(recs)
        for k in ("NOW", "PARTIAL", "GAP"):
            print(f"{k}: {c.get(k, 0)}")
        print(f"TOTAL: {sum(c.values())}")
        others = {k: v for k, v in c.items() if k not in ("NOW", "PARTIAL", "GAP")}
        if others:
            print("UNEXPECTED:", others)
    elif cmd == "themes":
        want = sys.argv[2] if len(sys.argv) > 2 else None
        c = Counter()
        for r in recs.values():
            if want and r["status"] != want:
                continue
            c[theme_of(r)] += 1
        for k, v in c.most_common():
            print(f"{k}: {v}")
    elif cmd == "list":
        status = sys.argv[2]
        theme = sys.argv[3] if len(sys.argv) > 3 else None
        for r in recs.values():
            if status != "*" and r["status"] != status:
                continue
            if theme and theme_of(r) != theme:
                continue
            print(f"{r['id']}\t{r['status']}\t{r['goal']} :: {r['expected']}")
    elif cmd == "ids":
        status = sys.argv[2]
        theme = sys.argv[3] if len(sys.argv) > 3 else None
        ids = [
            r["id"]
            for r in recs.values()
            if (status == "*" or r["status"] == status)
            and (not theme or theme_of(r) == theme)
        ]
        print(" ".join(ids))
    elif cmd == "set":
        status = sys.argv[2]
        ids = sys.argv[3:]
        changed = set_status(ids, status)
        print(f"set {len(changed)} to {status}")
    elif cmd == "summary":
        print(rewrite_summary())
    elif cmd == "show":
        for sid in sys.argv[2:]:
            r = recs[sid]
            print(f"### {sid}")
            for f in FIELDS:
                print(f"  {f}: {r[f.lower()]}")
    else:
        raise SystemExit(f"unknown command {cmd}")


if __name__ == "__main__":
    main()
