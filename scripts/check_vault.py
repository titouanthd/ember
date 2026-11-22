#!/usr/bin/env python3
"""Docs guard for the Ember workspace.

1. Every [[wikilink]] in ember-vault/ resolves to a note (and heading, if given).
2. No documentation lives outside the vault: the only .md files allowed
   outside ember-vault/ are README.md files.

Usage: python3 scripts/check_vault.py [--links-only | --guard-only]
Exit code 1 on any problem.
"""
import os, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VAULT = ROOT / "ember-vault"
SKIP_DIRS = {".git", "target", ".worktrees", "dist", "node_modules", ".obsidian", "ember-vault"}

def headings(text):
    out, code = [], False
    for line in text.splitlines():
        if line.startswith(("```", "~~~")):
            code = not code
        elif not code and (m := re.match(r"^#{1,6}\s+(.*)$", line)):
            out.append(m.group(1).strip())
    return out

def check_links():
    notes = {p.stem: p for p in VAULT.rglob("*.md") if ".obsidian" not in p.parts}
    errors, total = [], 0
    for p in notes.values():
        text = re.sub(r"```.*?```|~~~.*?~~~", "", p.read_text(encoding="utf-8"), flags=re.S)
        for m in re.finditer(r"(?<!!)\[\[([^\]]+)\]\]", text):
            total += 1
            target = re.split(r"\\?\|", m.group(1))[0]
            name, _, head = target.partition("#")
            if name not in notes:
                errors.append(f"{p.relative_to(ROOT)}: missing note [[{name}]]")
            elif head and head not in headings(notes[name].read_text(encoding="utf-8")):
                errors.append(f"{p.relative_to(ROOT)}: missing heading [[{name}#{head}]]")
    print(f"vault: {len(notes)} notes, {total} links, {len(errors)} broken")
    return errors

def guard():
    errors = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for f in filenames:
            if f.lower().endswith(".md") and f != "README.md":
                errors.append(f"doc outside vault: {Path(dirpath, f).relative_to(ROOT)}")
    print(f"docs guard: {len(errors)} stray doc(s) outside ember-vault/")
    return errors

if __name__ == "__main__":
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    errs = []
    if mode != "--guard-only":
        errs += check_links()
    if mode != "--links-only":
        errs += guard()
    for e in errs:
        print("  " + e)
    sys.exit(1 if errs else 0)
