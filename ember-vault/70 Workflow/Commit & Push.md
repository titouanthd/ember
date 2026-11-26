---
title: "Commit & Push"
aliases: ["Push", "Commit convention", "Git workflow"]
tags: ["ember/workflow", "ember/git"]
created: "2026-11-22"
---

# Commit & Push

How changes get into `main` on [github.com/titouanthd/ember](https://github.com/titouanthd/ember) (`origin`, SSH).

## 1. Commit message format

```
<type>(<scope>): <Description>

[optional body: why, details, ADR / issue refs]
```

Example: `fix(core-cooldown): Rename cooldown fn xy ex detail`

**Rules**
- `type` is one of the five types below, lower-case.
- `scope` is mandatory, in parentheses, lower-case kebab-case: `<area>-<module>` (see §1.2). Use only the area when the change spans the whole area (`chore(make)`).
- `: ` (colon + space), then the **description**: starts with a capital, imperative mood (*Add*, *Rename*, *Fix*), no trailing period, header ≤ 72 chars.
- Body (optional) after one blank line: what and why, wrap at ~72 chars. Reference ADRs (`ADR-030`) and roadmap phases (`Phase 5`).
- One logical change per commit. Formatting-only changes get their own `chore(fmt)` commit.

### 1.1 Types

| Type | Use for | Example |
|---|---|---|
| `feat` | New capability (engine API, game feature, tool) | `feat(ui-layout): Add VBox and HBox containers` |
| `fix` | Bug fix through the normal flow | `fix(scene-signal): Skip handlers whose target was despawned` |
| `hotfix` | Urgent fix pushed straight to `main` (broken build, crash in a shipped web game) | `hotfix(pagoda-web): Restore missing mq_js_bundle.js` |
| `chore` | Tooling, deps, CI, refactors with no behaviour change, merges | `chore(make): Add pre-push target` |
| `doc` | Vault notes, READMEs, doc comments | `doc(vault-roadmap): Mark Phase 0 done` |

### 1.2 Scopes

| Area | Scopes (examples) |
|---|---|
| `ember-core` → **core** | `core-math`, `core-cooldown`, `core-rng`, `core-variant`, `core-classdb`, `core-mainloop` |
| `ember-scene` → **scene** | `scene-tree`, `scene-signal`, `scene-packed`, `scene-context`, `scene-loop` |
| `ember-servers` → **servers** | `servers-render`, `servers-audio`, `servers-physics`, `servers-mock` |
| backends | `render-mq`, `audio-mq` |
| other engine crates | `input`, `assets`, `macros-derive`, `ui-<module>`, `editor`, `ember` (facade), `stdlib-<module>` |
| games | the game slug, optionally `-<module>`: `pong`, `pong-scene`, `ember-wars-combat`, `jade-garden-audio` |
| tooling | `xtask`, `make`, `deps`, `ci`, `fmt`, `git`, `scripts` |
| docs | `vault`, `vault-<area>` (`vault-roadmap`, `vault-adr`), `readme` |

### 1.3 Good / bad

| ✅ | ❌ (why) |
|---|---|
| `feat(scene-tree): Add find_by_class and find_all_by_class` | `feat: add find` (no scope, lower-case, vague) |
| `fix(pong-scene): Use ember_core::Color instead of macroquad` | `fix(pong): fixed colors.` (past tense, trailing period) |
| `doc(vault): Add Phase 5 plan` | `docs(engine): …` (old type name — use `doc`) |
| `chore(git): Merge Phase 0 branch into main` | `Merge branch 'x'` (rejected by the hook) |

### 1.4 Enforcement

`make hooks` installs `.githooks/commit-msg`, which rejects headers that don't match:

```
^(feat|fix|chore|hotfix|doc)\([a-z0-9][a-z0-9-]*\): [A-Z0-9].{0,70}$
```

Git's own `Revert "…"` and `fixup!`/`squash!` messages are let through. `git merge` and `git pull` default messages are **not**; use `git merge --no-ff -m "chore(git): Merge <branch> …"` and `git pull --rebase`.

## 2. Before every push

```bash
make pre-push     # fmt-check · clippy -D warnings · xtask layers + macroquad · docs guard · tests · vault links
make status       # branch, ahead/behind, unpushed local branches
```

Never push with a red `make pre-push`, except a `hotfix` (then say so in the body and fix the gate in the next commit).

## 3. Flows

### 3.1 Feature work (default)

```bash
git switch main && git pull --rebase
git switch -c feat/ui-layout           # branch: <type>/<scope>
# … work, commit in the format above …
make pre-push
git push -u origin feat/ui-layout      # push early: a branch that only lives on the laptop can be lost
# when done:
git switch main && git pull --rebase
git merge --no-ff feat/ui-layout -m "chore(git): Merge feat/ui-layout (Phase 5 layout)"
make pre-push && git push origin main
git branch -d feat/ui-layout && git push origin --delete feat/ui-layout
```

Agent worktrees (`.worktrees/<name>`) follow the same rule: push the branch as soon as it has a commit.

### 3.2 Hotfix

```bash
git switch main && git pull --rebase
# fix, then:
git commit -am "hotfix(<scope>): <What broke and how it is fixed>"
git push origin main
```

### 3.3 Docs only

Docs live only in `ember-vault/` (+ `README.md` files). `make vault-check` must be green.
`git commit -m "doc(vault-<area>): <Description>"`.

## 4. Handover

A handover = everything pushed + a dated note in `60 Progress/` ([[Handover 2026-11-22]] is the model).

```bash
make pre-push
make status                 # nothing unpushed?
git push origin main
git push origin --all       # back up every local branch
```

Then write `60 Progress/Handover <date>.md`: what changed, current state, exact next steps.

## See also
- [[Handover 2026-11-26]] · [[Next Phase Plan — Migration Close & Editor v2]] · [[Engine Spec Index#6. Contribution workflow|Spec contribution workflow]]
