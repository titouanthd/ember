---
title: "Token Budget"
aliases: ["Token consumption", "Working efficiently with Claude"]
tags: ["ember/workflow", "ember/claude"]
created: "2026-11-27"
---

# Token Budget — how we work to keep usage low

What the 2026-11-26/27 session taught us, and the rules we follow
from now on. Commit and push rules stay in [[Commit & Push]].

## 1. What we measured

One session ran 10 rounds (Pong M1.2–M1.4, taiji, pagoda, zhuo-ji,
jade-garden, accretion, bore-inc, the ember-wars port and its node tree):
616 model steps. Usage is weighed as cost: re-read context ≈ 0.1×,
newly cached context ≈ 2×, written output ≈ 5× a plain input token.

| Round | Share |
|---|---:|
| jade-garden (+ MP3 decoder, window options) | 20 % |
| pagoda | 14 % |
| bore-inc | 13 % |
| accretion | 13 % |
| ember-wars port | 11 % |
| Pong + editor (M1.2–M1.4) | 9 % |
| zhuo-ji | 7 % |
| ember-wars node tree | 5 % |
| taiji | 4 % |
| plan reading, recaps, small asks | 4 % |

By kind of work:

- **Re-reading the conversation is ≈ 80 % of the cost.** Every step
  re-reads everything said so far. By the end each step re-read
  ≈ 300 k tokens, most of it from rounds long finished.
- Shell commands (build, test, git, scripts) were 75 % of the steps,
  so they carry most of that re-reading; the commands themselves are
  cheap.
- Browser checks of web builds ≈ 5 %; writing replies ≈ 3 %; reading
  and editing files ≈ 1 %.
- The costliest rounds were the ones that grew an **engine feature**
  mid-round (jade-garden: MP3 + window options; pagoda: `fx`), not the
  biggest games. The ember-wars node tree, the hardest restructure, was
  only 5 % because it was planned before it was built.

## 2. Rules

### 2.1 One session per round
- Start a **new session for each round** (one game, one engine feature,
  one milestone step). Do not chain rounds in one conversation.
- A session starts from the vault, not from memory of a past
  conversation: [[Roadmap Tracker]], the current plan, the last
  handover, and this note.
- When a round ends, the session writes what the next one needs into
  the vault (tracker, plan, handover), then stops.

### 2.2 Plan before building
- An engine feature a game needs is its **own round**, decided before
  the game round starts (the M2 pattern), not discovered halfway.
- For a restructure, write the design first (in the plan or an ADR),
  then build. Agree on choices up front (one question, several options)
  rather than mid-round.

### 2.3 Keep tool output small
- Run **targeted checks**: `cargo test -p <crate>`, `cargo clippy -p
  <crate>`, `make macroquad layers vault-check docs-guard`. The full
  gate (`make pre-push`) is the user's (see [[Commit & Push]]).
- Filter long output (`| tail`, `| grep -E "^error|test result"`):
  a full build log re-read on every later step costs far more than
  the command did.
- Read files in parts (`sed -n a,bp`, `grep -n`) instead of whole
  1 000-line files; never re-read a file just edited.
- Prefer one script that does a batch of mechanical edits over many
  single edits.

### 2.4 Check cheaply
- Prove behaviour headlessly (`SceneLoop` on `MockRenderer` /
  `MockAudio`; a recorded-draw-call parity run for restructures, see
  [[41-migration-guide#5.17 ember-wars (acceptance test) — **node tree** (`f16fe0c` port, `0ad070c` nodes; 2026-11-27)|§5.17]]).
- Use the browser only for what headless runs cannot show (web builds,
  storage), and read pages as text, not screenshots.
- The user plays the game; Claude does not try to replace that.

### 2.5 Ask for less, answer shorter
- One clear request per message ("Start M4.1"), with the choices the
  round needs.
- Recaps stay short and point to the vault; the vault is the record.

## 3. Rough budget per round

| Round kind | Target |
|---|---|
| Small game port, no engine change | ≈ 1 session, under ~60 steps |
| Engine feature + ADR | ≈ 1 session, its own round |
| Big game or restructure | design first, then ≈ 1 session |

If a round goes past ~100 steps, stop, write the state into the vault,
and continue in a fresh session.
