# Zhuo Ji — Design Document

## 1. Overview

A 4-player Guiyang-style Mahjong game with one human and three AI opponents. The game uses 108 tiles (three suits, no honors) and features two signature mechanics that distinguish it from standard Mahjong: **豆 (Dou)** — Gang bonuses that act as a "passport" for winning on a discard — and **鸡 (Ji)** — a bonus tile revealed after the hand ends that multiplies the score. The result is a fast, aggressive variant where players compete not just to win, but to collect the most Dou and Ji along the way.

**Core loop:** Draw → Discard → Claim (Peng/Gang/Hu) → Repeat until win or wall exhaustion.

**Match structure:** N hands (configurable, default 4). Highest cumulative score wins.

## 2. Tile Model

### 2.1 Tiles

```rust
pub enum Suit { Wan, Tiao, Tong }  // 万 (characters), 条 (bamboo), 筒 (dots)

pub enum Tile {
    Number { suit: Suit, rank: u8 },  // rank 1..=9
}
```

Each suit has ranks 1 through 9, with **four copies of each tile**. Total: 3 suits × 9 ranks × 4 copies = **108 tiles**.

### 2.2 Meld Types

```rust
pub enum Meld {
    Peng { tile: Tile, from: usize },       // 碰 — claim a triplet from a discard
    Gang { tile: Tile, from: GangSource },  // 杠 — claim or form a quad
}

pub enum GangSource {
    An,                    // 闷豆 (An Gang) — self-drawn quad
    Bu,                    // 爬坡豆 (Bu Gang) — upgrade a Peng to a Gang
    Ming { from: usize },  // 点豆 (Ming Gang) — claim a discard to complete a quad
}
```

**No Chi (吃).** Sequences cannot be claimed from other players. All sequences must be formed within your own hand.

### 2.3 Player

```rust
pub struct Player {
    pub concealed: Vec<Tile>,  // tiles in hand (13, or 14 on your turn)
    pub melds: Vec<Meld>,      // exposed sets (Peng / Gang)
    pub discards: Vec<Tile>,   // this player's river (打出的牌)
    pub is_ai: bool,
    pub score: i32,
}
```

## 3. Game State

```rust
pub struct Game {
    pub phase: Phase,
    pub wall: Vec<Tile>,               // remaining drawable tiles
    pub players: [Player; 4],
    pub turn: usize,                   // current player index (0..4)
    pub dealer: usize,                 // 庄家 index
    pub last_discard: Option<(usize, Tile)>,
    pub ji_tile: Option<Tile>,         // determined after hand ends
    pub rng: Rng,
}
```

## 4. Phase Machine

```rust
pub enum Phase {
    Deal { t: f32 },                                        // 发牌 animation
    AwaitingDraw { player: usize },                         // player about to draw
    DrawAnim { player: usize, t: f32 },                     // 摸牌 animation
    AwaitingDiscard { player: usize },                      // player must act
    AwaitingClaims { discard: Tile, from: usize, t: f32 },  // 碰/杠/胡 window
    ClaimAnim { claimer: usize, meld: Meld, t: f32 },
    Hu { winner: usize, method: HuMethod },                 // 胡牌
    HuangZhuang,                                            // 黄庄 (exhaustive draw)
    GameOver { winner: usize },                             // match end
}

pub enum HuMethod {
    Zimo,               // 自摸 — win on self-draw
    Hu { from: usize }, // 胡 — win on another's discard
}
```

### 4.1 Timing

| Phase | Duration | Notes |
|---|---|---|
| Deal | 0.5 s | Tile distribution animation |
| DrawAnim | 0.3 s | Tile sliding into hand |
| AwaitingDiscard (AI) | 0.6–0.8 s | Perceived thinking time |
| AwaitingClaims | 1.0 s | Human can click Peng/Gang/Hu buttons |
| ClaimAnim | 0.4 s | Meld slides into place |

## 5. Turn Flow

1. **Draw** — Current player draws one tile from the wall.
2. **Act** — The player may:
   - **Discard** (most common),
   - **Zimo** (胡 — win on self-draw, if hand is complete),
   - **An Gang** (闷豆 — declare a concealed quad, then draw again).
3. **Claim window** — After a discard, other players may claim in strict priority:
   - **Hu** (胡 — win) — any player
   - **Peng** (碰 — triplet) — any player
   - **Gang** (杠 — quad) — any player
4. **Resolve** — Highest-priority claim wins. Ties within a priority go to the player closest counter-clockwise to the discarder.
5. **Wall empty** → **Huangzhuang** (黄庄 — exhaustive draw), triggering 查叫 (see §7.4).

## 6. Winning Hands

### 6.1 Standard Hand

**4 melds + 1 pair.** A meld is either a triplet (刻子) or a sequence (顺子). A pair (将牌) is two identical tiles.

```
Example: 1W 2W 3W | 5T 5T 5T | 7D 8D 9D | 2W 3W 4W | 9T 9T
         (sequence)  (triplet)   (sequence)   (sequence)   (pair)
```

### 6.2 Win Detection

```rust
pub fn is_winning_hand(concealed: &[Tile], melds: &[Meld]) -> bool
```

Backtracking algorithm:

1. Try each tile as the pair; remove it.
2. Recursively partition remaining tiles into triplets and sequences.
3. Succeed if exactly `4 - melds.len()` melds are formed.

14 tiles → tractable with naive backtracking. No optimization needed.

### 6.3 The "Dou Passport" Rule

**Without at least one Dou (Gang), you may only Hu on Zimo (self-draw) or with a hand of 平胡 or higher** (i.e., a non-平胡 pattern like 大对子). To Hu on another player's discard with only 平胡, you must have a Dou. This makes Gang declaration a strategic necessity, not just a scoring bonus.

## 7. Scoring

### 7.1 Base Values

| Pattern | Chinese | Fan (番) | Condition |
|---|---|---|---|
| 平胡 | Ping Hu | 1 | Standard 4 melds + 1 pair |
| 大对子 | Da Dui Zi | 5 | All melds are triplets (对对胡) |
| 七对 | Qi Dui | 10 | Seven pairs |
| 清一色 | Qing Yi Se | 10 | All tiles in one suit |
| 清大对 | Qing Da Dui | 15 | All triplets, one suit |
| 龙七对 | Long Qi Dui | 20 | Seven pairs with a concealed quad |
| 青龙背 | Qing Long Bei | 30 | All one suit, seven pairs with concealed quad |

### 7.2 Dou (Gang) Scoring

| Dou Type | Chinese | Payment | Notes |
|---|---|---|---|
| 闷豆 | An Gang | Each other player pays **2 fan** | Self-drawn quad |
| 爬坡豆 | Bu Gang | Each other player pays **3 fan** | Peng upgraded to Gang |
| 点豆 | Ming Gang | The discarder pays **1 fan** | Claimed quad |

### 7.3 Ji (Chicken) Scoring

After a player wins (and tiles remain in the wall), flip the next tile from the wall. The **next tile in sequence** becomes the Ji (鸡):

- Flip 5 Wan → 6 Wan is Ji
- Flip 7 Tong → 8 Tong is Ji
- Flip 8 Tong → 9 Tong is Ji
- Flip 9 Tiao → 1 Tiao is Ji (wraps around)

**Two fixed Ji tiles with special status:**

- **幺鸡 (1 Tiao):** Always counted as Ji. When the flipped tile is 9 Tiao, 1 Tiao becomes **金鸡 (Golden Ji)**, worth **2 fan per copy** instead of 1.
- **八筒 (8 Tong):** Also always counted as Ji under the same wrap-around rule as 1 Tiao. When the flipped tile is 7 Tong, 8 Tong is the Ji (standard). When the flipped tile is 8 Tong itself, 8 Tong is still counted — it carries the same elevated status as 1 Tiao in the scoring table.

**Special multipliers:**

- **冲锋鸡 (Charge Ji):** The **first tile discarded** by any player, if it is a Ji tile, counts double.
- **责任鸡 (Responsibility Ji):** If the first discarded Ji tile is Peng'd, the discarder pays an extra 1 fan.

Every player counts their Ji tiles (in hand, melds, and discards). Each Ji tile is worth **1 fan from each other player** (2 fan if Golden Ji).

### 7.4 Huangzhuang (黄庄) — Exhaustive Draw

When the wall is empty and no one has won, all players reveal their hands. This is called **查叫 (Cha Jiao)** — checking who was tenpai (听牌, one tile from winning).

| Scenario | Outcome |
|---|---|
| No player is tenpai | No payments, dealer remains |
| All four are tenpai | No payments |
| Some are tenpai | Non-tenpai players pay tenpai players the value of their hand (without Dou/Ji) |

All Dou and Ji scores are **not counted** during Huangzhuang.

### 7.5 Score Formula

```
Total = Base_Fan × Pattern_Multiplier + Dou_Fan + Ji_Fan
```

**Payment direction:**

- **Zimo (自摸):** All three other players pay the winner.
- **Hu (胡):** Only the discarder pays.

Dealer wins/losses are doubled.

## 8. AI Design (V1)

The AI is purely offensive. It plays efficiently but does not defend, making it beatable but competitive.

### 8.1 Shanten Calculation

Shanten (向听数) = how many tile swaps away from tenpai.

```
shanten = (4 - melds) * 2 - partials - has_pair
```

`partials` counts incomplete melds (pairs, or two tiles that can become a sequence). Pairs can double-count as either the hand's pair or a partial set.

Implementation: recursive search over tile assignments. ~100 lines, deeply testable against known examples.

### 8.2 Discard Decision

```rust
pub fn decide_discard(hand: &[Tile], melds: &[Meld]) -> Tile {
    // For each tile, compute shanten after discarding it.
    // Pick the tile that minimizes shanten.
    // Tie-break: honors first, then 1s and 9s, then middle tiles.
}
```

### 8.3 Claim Decision

```rust
pub fn decide_claim(
    hand: &[Tile],
    melds: &[Meld],
    discard: Tile,
    claimer_idx: usize,
    discarder_idx: usize,
) -> AiAction {
    // Peng: if it reduces shanten, take it.
    // Gang: if it reduces shanten, take it.
    // Hu: if hand is complete, declare.
    // Otherwise: Pass.
}
```

### 8.4 No Defense (V1)

The AI ignores what other players are collecting. It does not avoid dangerous discards. This keeps the AI simple and makes the human's game feel winnable.

**V2 addition:** A simple danger heuristic — avoid discarding tiles that appear in an opponent's river when that opponent's shanten is low.

## 9. Rendering (V1: ASCII)

DejaVu Sans Mono does not cover CJK characters (万, 条, 筒). V1 uses text labels:

| Tile | Label | Tile | Label |
|---|---|---|---|
| 1 Wan | `1W` | 1 Tiao | `1T` |
| 5 Wan | `5W` | 5 Tiao | `5T` |
| 9 Wan | `9W` | 9 Tiao | `9T` |
| 1 Tong | `1D` | 8 Tong | `8D` |

**V2 polish:** Load Noto Sans Symbols 2 (~2 MB) for proper CJK rendering.

### 9.1 Layout

```
┌──────────────────────────────────────────────────────────────┐
│  [AI North]  tiles: ██ ██ ██  melds: [5T 5T 5T]              │
│                                                              │
│  [AI West]                    [Center]           [AI East]   │
│  ██ ██ ██                    Wall: 42                        │
│  [2W 2W 2W]                  Turn: You                       │
│                              Phase: Your discard             │
│                                                              │
│  [You]                                                       │
│  Hand: 1W 2W 3W 5T 5T 5T 7D 8D 9D 2W 3W 4W 9T 9T             │
│  Melds: none          River: [1W] [3D] [9T]                  │
│  Score: 0                                                    │
└──────────────────────────────────────────────────────────────┘
```

- **Bottom:** Human hand (click a tile to discard).
- **Left/Right/Top:** AI players (tile backs, exposed melds, rivers).
- **Center:** Wall count, turn indicator, phase label.
- **Claim buttons:** Appear when the human can Peng / Gang / Hu a discard.

## 10. Project Structure

```
games/mahjong/
├── Cargo.toml
├── .env                    # colors, layout, AI timings
├── DESIGN.md               # this file
├── assets/
│   └── DejaVuSansMono.ttf
├── src/
│   ├── main.rs             # game loop, input, rendering
│   ├── lib.rs              # module declarations, re-exports
│   ├── config.rs           # GameContext (colors, layout, tuning)
│   ├── components.rs       # Tile, Suit, Meld, Player, Game
│   ├── wall.rs             # tile creation, shuffle, deal
│   ├── hand.rs             # is_winning_hand, shanten, tenpai
│   ├── ai.rs               # AiState, decide_discard, decide_claim
│   ├── systems.rs          # Game, Phase, turn flow, scoring
│   └── font.rs             # font loading
└── tests/
    └── game_scenarios.rs   # integration tests
```

## 11. Known Traps

1. **Tile is `Copy`.** Removing from a hand needs `Vec::remove(i)` (order-preserving), not `swap_remove` (breaks sorted invariants).

2. **Shanten edge cases.** 13 vs 14 tile hands, open melds vs concealed, pair vs partial ambiguity. Test against known examples.

3. **Claim priority order.** Hu > Peng > Gang. Two players Peng the same tile → closer counter-clockwise to the discarder wins. Off-by-one here ruins games.

4. **No Chi.** Guiyang Mahjong does not allow Chi. Do not implement it.

5. **Gang draws again.** After any Gang (An, Bu, or Ming), the same player draws again. The wall can empty mid-turn.

6. **Dou passport rule.** Without a Dou, you cannot Hu on a discard with only 平胡. This is the single most-forgotten rule in implementations.

7. **Ji flip timing.** Ji is only determined if the wall has tiles remaining after the win. If the wall is empty, no Ji is counted.

8. **Twin fixed Ji tiles.** Both **1 Tiao** and **8 Tong** carry special status. When the flip lands on 9 Tiao, 1 Tiao becomes Golden Ji; when the flip lands on 8 Tong, 8 Tong is still counted (same rule as 1 Tiao). Do not forget the 8 Tong case — it is unique to the Guiyang variant and easy to miss when porting generic Mahjong code.

9. **Huangzhuang查叫.** Dou and Ji are not counted during exhaustive draw. Only the hand's base pattern value is paid.

10. **AI hesitation timing.** AI acting instantly feels robotic. Add 0.6–0.8 s dwell between draw and discard.

11. **Dealer rotation.** Rotate dealer clockwise, except on Huangzhuang with no tenpai (dealer remains).

## 12. Testing Strategy

### 12.1 Pure Functions (highest priority)

| Function | Test Coverage |
|---|---|
| `is_winning_hand` | Canonical hands, near-wins, garbage hands, hands with melds |
| `shanten` | Known examples from Mahjong literature, edge cases (7 pairs, 13 orphans) |
| `decide_discard` | Given a hand, chosen discard minimizes shanten (deterministic tie-break) |

### 12.2 Integration

- Scripted AI decisions + deterministic RNG for the wall.
- Verify phase transitions and final scores.
- A full N-hand match with seeded RNG ends at a deterministic final score.

**Target:** 60–80 tests, comparable to FreeCell.

## 13. Session Plan (5 Sessions)

| Session | Scope | Deliverable |
|---|---|---|
| **1** | Tile model, wall, deal, human draw/discard, basic rendering | Playable hot-seat with no AI, no calls |
| **2** | Win detection + shanten + tenpai as pure tested functions | All hand-evaluation logic green |
| **3** | AI discard logic (no calls) | AI opponents that discard intelligently |
| **4** | Calls (Peng/Gang/Hu), claim priority, full turn flow, AI call decisions | Complete game loop |
| **5** | Scoring (Dou, Ji, pattern fan), match flow, integration tests, polish | Full game, all tests green |

## 14. What This Adds to Ember

| New Capability | Previous Max |
|---|---|
| Turn-based game loop | All previous games real-time |
| Complex hand evaluation (backtracking) | Simple grid/line checks |
| AI reasoning about hidden information | Air Hockey AI sees full state |
| 4-player state management | 2 players max |
| Asynchronous claim windows (timed input) | No previous timed-decision UI |

## 15. Extraction Candidates (Rule of Three)

| Pattern | Occurrences | Action |
|---|---|---|
| Turn-based state machine | 1 (Mahjong) | Local. Extract if a 2nd turn-based game appears. |
| Timed claim window / prompt | 1 (Mahjong) | Local. Watch for reuse. |
| Shanten / hand evaluation | 1 (Mahjong) | Too domain-specific; stays local. |
| Tile rendering | 1 (Mahjong) | Local. Extract if a 2nd tile game appears. |

---

*This design prioritizes a playable, testable core over rule completeness. Features marked V2 (defense AI, CJK rendering, yaku, furiten) can be added incrementally once the base game is solid.*