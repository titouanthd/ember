#!/usr/bin/env python3
"""After merging `titouanthd-docs-crates-games-plan` into main, make its reviewed
spec files the canonical ones in the vault.

Copies ember-vault/90 Branch Snapshot/docs-crates-games-plan/*.md over
ember-vault/10 Engine Spec v0.1/, strips the " (branch)" suffix from names and
wikilinks, drops the branch tag, then removes the snapshot folder.
Run: python3 scripts/promote_branch_docs.py  (then `make vault-check`)
"""
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SNAP = ROOT / "ember-vault/90 Branch Snapshot/docs-crates-games-plan"
SPEC = ROOT / "ember-vault/10 Engine Spec v0.1"

if not SNAP.is_dir():
    raise SystemExit("no branch snapshot to promote")
for src in sorted(SNAP.glob("*.md")):
    text = src.read_text(encoding="utf-8")
    text = text.replace(" (branch)", "").replace(', "ember/branch-snapshot"', "")
    dst = SPEC / src.name.replace(" (branch)", "")
    dst.write_text(text, encoding="utf-8")
    print("promoted", dst.relative_to(ROOT))
shutil.rmtree(SNAP.parent)
for note in (ROOT / "ember-vault").rglob("*.md"):
    t = note.read_text(encoding="utf-8")
    if "(branch)" in t:
        note.write_text(t.replace("[[Engine Spec Index (branch)]]", "[[Engine Spec Index]]")
                         .replace("[[50-design-decisions (branch)]]", "[[50-design-decisions]]"), encoding="utf-8")
print("done — now run: make vault-check")
