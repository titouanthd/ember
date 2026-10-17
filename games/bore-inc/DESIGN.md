# Bore Inc. — Design Document

> Inspired by **Sandustry** (2024) for pixel-perfect rendering and
> satisfying destruction, by **Motherload** (XGen Studios, 2004)
> for 2D mining and selling economy, by **Cookie Clicker**
> (Orteil, 2013) and **Adventure Capitalist** (Hyper Hippo, 2014)
> for the classic incremental loop, by **SteamWorld Dig**
> (Image & Form, 2013) for digging, by **Factorio** (Wube,
> 2020) for automation with robot employees, and by
> **Overcooked** (Ghost Town Games, 2016) for the top-down map.
>
> An incremental mining game where a cat entrepreneur, **Archie**,
> drills tunnels through mountains with the help of robots,
> and where the goal is not to mine everything — it's to get the
> Bore Inc. truck to the other side.

Last updated: Session 2 (final design doc, jam).
Status: **in design**. Target: **browser (WASM)**.

---

## 1. Pitch

**Bore Inc.** is a cozy incremental mining game.
The player controls **Archie**, a cat entrepreneur with a hard hat
who runs a small drilling company. Each level is a mountain to
cross: Archie digs, the robot employees dig, and the extracted
resources fund the operation. The goal is not to mine everything
— it's to **get the Bore Inc. truck to the finish line**, the
tunnel pierced from side to side, the contract fulfilled.

**Key mechanics:**

- **Mining by movement**: Archie digs by pushing against a tile.
  Holding a direction = mining. **Tap** = push. It's the same
  input to mine, empty the bag into the truck bed, or enter the
  truck.
- **Jump**: 1 jump = 1 purchase on an upgrade block. Also used
  for navigation: crossing a gap, reaching a ledge, moving
  around the HQ.
- **Drivable truck**: the tunnel must be wide enough and flat
  enough for the truck to pass. Archie drives the truck. The
  tunnel must be designed **for a vehicle**, not just for him.
- **Robot employees**: robot capsules are buried in the mountain.
  Archie frees them by mining them, or hires them at the HQ.
  Robots mine automatically.
- **Upgrade blocks**: upgrades are bought by headbutting a block.
  No menu, no clicking.
- **Pixel-perfect rendering**: logic is tile-based, but rendering
  mimics Sandustry — irregular edges, physical destruction
  particles, veins glowing through rock.
- **Goal**: get the truck to the finish line. Direct path = fast
  but poor. Exploration = slow but rich.

**Visual theme**: cozy underground mine. Earthy palette: ochres,
browns, stone gray, cyan accents for crystal, amber for lanterns.
Archie is a small sprite with a miner's helmet. Robots are copper
boxes with glowing eyes. The truck is a small rugged vehicle
marked "Bore Inc.". Rendering mimics physical pixel games
(Sandustry, Noita) without simulating physics.

**Narration**: minimal. Each level is a contract. Archie is a
small entrepreneur drilling tunnels to connect villages. No
dialogue, no text — just mountains, robots, and resources. Tips
appear via iconographic signs in the world.

**Platform**: browser (WebAssembly). Optional native desktop
support. Custom engine **Ember** (2D), written in Rust.

---

## 2. Restriction Compliance

The jam restriction is **MOVEMENT INPUT ONLY**. Every interaction
in Bore Inc. is driven by movement:

- **Mine**: push (tap) against a tile.
- **Empty bag into truck**: tap the physical button on the truck
  bed.
- **Enter truck**: tap the truck door.
- **Drive truck**: left/right once inside.
- **Buy an upgrade**: jump and headbutt a block.
- **Finish a level**: get the truck to the finish line.
- **Start the game**: any movement key.
- **Next level**: via the top-down map, driving the truck to a
  mountain entrance.
- **Replay**: automatic, or via a respawn pad.

No UI buttons. No menu. No clicking. The mouse is not read.
This is not a circumvented constraint — it's the core of the
design. Archie's movement IS the game.

**Allowed inputs**: movement (left/right), jump, tap (push
against). Truck driving = left/right. Everything else is
triggered by contact, jump, or tap.

---

## 3. Inspirations

- **Sandustry** (2024) — pixel-perfect rendering, satisfying
  destruction, immediate terrain reading.
- **Motherload** (XGen Studios, 2004) — 2D mining, resources by
  depth, selling, upgrades.
- **Cookie Clicker** (Orteil, 2013) — classic incremental loop,
  multipliers.
- **Adventure Capitalist** (Hyper Hippo, 2014) — tiered
  progression, big number satisfaction.
- **SteamWorld Dig** (Image & Form, 2013) — satisfying digging,
  visual feedback.
- **Factorio** (Wube, 2020) — automation, robots working for you.
- **Overcooked** (Ghost Town Games, 2016) — top-down map, free
  vehicle movement.
- **Minecraft** (Mojang, 2011) — cube reading.
- **Monument Valley** (ustwo, 2014) — contemplative pacing.

---

## 4. Scope — V1 (jam)

| Element | Delivered | Stretch (post-jam) |
|---|---|---|
| **Grid** | ~4000×400 px per mountain, depth ~200 px | +larger |
| **Generation** | Procedural, tested seeds | +biomes |
| **Tiles** | Dirt, Stone, Coal, Iron, Crystal, HardRock, Bedrock, Capsule | +Lava, +Water, +Gas |
| **Mining** | Archie by push, robot by contact | +Pickaxe upgrades |
| **Truck** | 1 truck, 2 tiles high, driven by Archie | +multiple trucks, +robot driver |
| **Robots** | 3 types (Digger, Drill, Seeker) | +Hauler, +Scout |
| **Personalities** | 4 archetypes (Cautious, Independent, Greedy, Steady) | +AI-generated |
| **Resources** | Stone, Coal, Iron, Crystal | +others |
| **Processing** | Generator, Foundry, Nail Machine | +other workshops |
| **Upgrade blocks** | Archie, Truck, Robots | +prestige blocks |
| **Levels** | 2 mountains | +3rd, +4th |
| **Map** | Top-down, pure top-down | +2.5D |
| **HQ** | Horizontal scroll, warehouse/factory | +animated decor |
| **Prestige** | No | +permanent multiplier reset |
| **Idle / offline** | Out of jam scope | +last_seen + formula |
| **Art** | Modular SVG, pixel-perfect | +textures |
| **Sound** | Mining SFX, lantern ambience | +music |
| **Screens** | Title + Game + HQ + Map | +menu, +world map |
| **Web** | WASM build via Ember | +cloud localStorage |

**Target tests**: ~120–140 tests.

---

## 5. Core Loop

### 5.1 Frame by frame

1. Read input (arrows / WASD / space).
2. If Archie is moving:
   - Compute target tile in the direction.
   - If empty → move Archie.
   - If solid → mine the tile (progress).
3. Robot tick:
   - Each robot has a target (tile to mine).
   - Move toward target, mine on contact.
   - If target destroyed, choose a new target according to
     personality.
4. Mining tick:
   - `mining_progress += dps * dt`.
   - If `progress >= tile.hp` → destroy tile.
   - Drop resources on the ground.
   - Emit particles + floating text + shake.
5. Drop tick:
   - Archie walks over them → added to bag (if space).
6. Truck tick:
   - If Archie is inside → left/right to drive.
   - Truck moves as long as the tunnel is drivable.
   - Soft block if slope > max, tunnel too low, or obstacle.
7. Upgrade block tick (HQ):
   - Archie jumps and headbutts a block → purchase if enough
     dollars.
8. Check victory:
   - If truck reaches the finish line → level complete.
9. Juice tick (particles, floating text, shake, flash).

### 5.2 Game by game

**Title screen**
- "Press Space to start" → any movement key.

**Intro**
- Archie in front of his Bore Inc. truck, at the foot of the
  mountain.
- No text. "Bore Inc." sign + arrow to the right.

**Level**
1. Archie next to the truck, truck on the left, mountain on the
   right.
2. Archie digs, collects, fills his bag.
3. He returns to the truck, taps the bed button → bag emptied.
4. The truck advances into the tunnel Archie dug.
5. Goal: get the truck to the finish line on the right.

**Level exit**
- Archie leaves the map on the left side (or the truck reaches
  the exit on the right).
- Transition: fade to black, loading, fade in.
- Level progress saved (tunnel preserved).

**HQ**
- Horizontal scroll, warehouse/factory background.
- Zones in order: Truck parking → Selling → Machines → Upgrade
  blocks → Exit to map.
- Archie jumps on blocks to buy.
- Archie taps the sell button to sell the truck's stock.
- Archie taps machines to activate / buy them.

**Top-down map**
- Flat ground + peaks representing mountains.
- Truck drives freely, Overcooked-style.
- Archie drives the truck to a mountain entrance to select a
  level, or to the HQ to return.
- Map → level transition: truck enters the mountain, fade,
  loading.

**Full loop**
HQ → map → level → mining → tunnel for the truck → arrival
→ HQ → map → next level.

### 5.3 HQ ↔ Map ↔ Level loop
[TITLE] → [INTRO] → [LEVEL 1]
↓
[HQ] ←→ [MAP] ←→ [LEVEL 2]
↑ ↓
└────────────────────┘


Each round trip to the HQ lets you sell, process, buy upgrades.
The map acts as a level selector. Levels keep their state
(tunnel, mined resources).

---

## 6. Archie

**Identity**
- Small cat entrepreneur, hard hat, visible bag.
- Cozy, never in danger, never deadly.

**Controls**
- Left / right: movement.
- Jump: navigation + purchase on blocks.
- Tap: push against a tile (mine), a button (bed), a door
  (truck), an interactive object.
- Inside the truck: left / right to drive, jump to exit.

**Base stats (to fine-tune)**
- Movement speed: 100 px/s.
- Pickaxe damage: 1.
- Mining speed: 1×.
- Bag capacity: 10.
- Jump height: 2 tiles.
- Coyote time: 100 ms.
- Jump buffer: 150 ms.

**Bag**
- Limited capacity, upgradeable.
- Fills by walking over drops.
- Empties by tapping the truck bed button.
- If full, Archie mines but no longer collects.
- Visible on the sprite (bag swells).

**Animations**
- Idle (tail moving, ears twitching).
- Walk.
- Jump (preparation, rise, fall).
- Mining (head / paw strike).
- Deposit (tap the button).
- Driving (inside the truck).
- Purchase (jump + headbutt the block).
- Celebration (level complete).

**Feedback**
- Meows when he finds crystal.
- Puffs when the rock is hard.
- Small jump when the door opens.
- Helmet glows brighter with pickaxe power.

---

## 7. Truck

**Identity**
- Small rugged vehicle marked "Bore Inc.".
- Starts 2 tiles high, upgradeable in height and length.

**Mechanics**
- Driven by Archie (initially).
- Robot driver as a later upgrade (stretch).
- Max slope: 30° initially, upgradeable.
- Can reverse.
- No fuel.
- No mining: the truck rolls, it doesn't dig.
- Soft block: if slope too steep, tunnel too low, or blocked,
  the truck stops. Archie gets out, digs, returns.
- Doesn't fall into holes: stops at the edge.

**Storage**
- Limited capacity, upgradeable.
- Archie empties his bag by tapping the bed button.
- Stock is sold / processed at the HQ.

**Enter / exit**
- Archie taps the truck door to enter.
- Archie jumps to exit.
- The truck stays where Archie leaves it.

**Upgrades (at HQ)**
- Height / length (stretch).
- Storage capacity.
- Max slope.
- Speed.
- Robot driver (stretch).
- Multiple trucks (V2, stretch).

**Finish line**
- Visible from afar: glow through rock + sign.
- When the truck reaches it → level complete.

---

## 8. Robots

**V1 Types**
- **Digger**: mines the nearest tile.
- **Drill**: mines faster but only in a straight line.
- **Seeker**: seeks resource veins.

**Personalities**
- **Cautious**: avoids HardRock, mines nearby.
- **Independent**: goes far, chooses alone.
- **Greedy**: prioritizes Crystal / Iron.
- **Steady**: always mines the nearest tile.

**Behavior**
- Mine automatically.
- Don't collect: resources drop, Archie collects.
- Don't trigger pads / blocks.
- Pass through Archie and the truck.
- If stuck, change target.
- Pathfinding: local rules + simple target (A* if needed).

**Unlocking**
- Capsules found in the mountain (free).
- Hire blocks at the HQ (paid).
- Both coexist.

**Upgrades (at HQ)**
- Unlock robot 1 (Digger).
- Speed.
- Storage (stretch).
- Unlock Drill, Seeker (stretch).

**Limit**
- Max robots per level, upgradeable.

---

## 9. Mining & Tiles

**V1 Types**
| Tile | HP | Drop | Notes |
|---|---|---|---|
| Dirt | 1 | Stone | Fast |
| Stone | 2 | Stone | Common |
| Coal | 3 | Coal | Resource |
| Iron | 4 | Iron | Resource |
| Crystal | 6 | Crystal | Rare, precious |
| HardRock | 20 | Stone | Requires pickaxe level X |
| Bedrock | ∞ | — | Indestructible, bounds the map |
| Capsule | 5 | Robot | Frees a robot |

**Cracks**
- 4 visual stages: intact, lightly cracked, medium cracked,
  almost broken.
- Sound pitch rises as the tile breaks.

**Drop**
- Resources fall to the ground.
- Archie walks over them → added to bag (if space).
- Uncollected resources remain on the ground.

**Particles**
- Different per material: dirt, stone, coal, iron, crystal.
- Crystal → short hit-stop + flash.

**The entire map is mineable** except Bedrock. Everything is
destructible with more or less effort.

---

## 10. Obstacles

**V1**
- **HardRock**: high HP, requires pickaxe level X. Distinct
  visual. Forces detours or upgrades.
- **Bedrock**: indestructible. Limits depth (~200 px, scalable
  per map). Dark texture, veins.
- **Gas**: pocket that, once pierced, pushes Archie back and
  blocks mining for 3 s. Visual: green bubbles.
- **Water**: pocket that, once pierced, fills neighboring empty
  tiles, slows Archie, drowns resources (must mine to recover).
  Visual: translucent blue.

**Stretch**
- Lava: too punishing for a cozy game.
- Cave-in: tiles fall when mining underneath.
- Explosive gas: reacts to mining, destroys an area.
- Unstable crystals: shatter everything around if poorly mined.

**Jetpack** (stretch): lets Archie and robots reach the entire
map easily, as an upgrade.

---

## 11. Pads & Upgrade Blocks

**Principle**
- Upgrades are bought at the HQ, not in the mountain.
- They are **blocks** that you headbutt.
- Archie jumps and headbutts the block → purchase if enough
  dollars.
- Level displayed on the block: X/Y.
- Wall sign behind the block with details (icons, cost, effect).
- Cost: dollars only.
- Values to fine-tune, start with logical values.

**V1 Blocks**
| Block | Levels | Effect per level |
|---|---|---|
| Pickaxe damage | 4 | +20 % |
| Mining speed | 4 | +15 % |
| Movement speed | 4 | +10 % |
| Bag capacity | 4 | +10 |
| Truck storage | 3 | +100 |
| Unlock robot 1 | 1 | Digger |
| Robot speed | 3 | +15 % |
| Robot storage | 3 | +10 |

**Purchase animation**
- Archie jumps, headbutts the block.
- Metallic "clonk".
- Stars + floating text "+1 Pickaxe".
- The block sinks one level.
- At max: the block sinks completely, leaves an engraved
  "MAX" plate. You see all purchased upgrades.

**Indicative costs (to fine-tune)**
- Pickaxe damage 1: $25.
- Mining speed 1: $30.
- Movement speed 1: $20.
- Bag capacity 1: $20.
- Truck storage 1: $30.
- Unlock robot 1: $100.
- Robot speed 1: $40.
- Robot storage 1: $35.

---

## 12. HQ

**View**
- Horizontal scroll, warehouse / factory background.
- Archie moves left to right, jumps on platforms.

**Zones in order**
1. **Truck parking** — the truck arrives and parks.
2. **Selling** — Archie taps the button to sell the stock.
3. **Machines** — Generator, Foundry, Nail Machine.
4. **Upgrade blocks** — Archie jumps and headbutts to buy.
5. **Exit to map** — Archie drives the truck to the map.

**V1 Machines**
1. **Coal Generator** — consumes coal → electricity.
   - 3 levels (1/3, 2/3, 3/3).
   - Coal is consumed directly from storage.
2. **Coal Foundry** — smelts materials → ingots.
   - 3 levels.
   - Recipe: 2 Coal + 2 Iron → 1 Steel Ingot ($15).
3. **Nail Machine** — iron + electricity → nails.
   - 3 levels.
   - Recipe: 1 Iron + 1 Electricity → 4 Nails ($25 total).

**Unlocking**
- Workshops unlock progressively as you advance.
- An unlocked workshop can be bought with dollars.

**No menu, no clicking.**
- Only blocks, machines, physical buttons.

---

## 13. Top-Down Map

**View**
- Pure top-down for V1.
- Flat ground + peaks representing mountains.
- 2.5D as stretch.

**Movement**
- Truck drives freely, Overcooked-style.
- Archie is inside the truck.
- Transition to HQ or to a level by entering a zone.

**Level selection**
- Truck drives to a mountain entrance → level loaded.
- Truck drives to the HQ → HQ loaded.
- No menu, no clicking.

**Transition**
- Map → level: truck enters the mountain, fade, loading, fade.
- Map → HQ: truck enters the building, fade, loading, fade.

**Persistence**
- Completed levels remain visible.
- In-progress levels keep their state.
- Routes unlock progressively (stretch).

---

## 14. Levels

**Generation**
- **Procedural**.
- Test multiple seeds to select the best ones.
- Integration tests to verify resource quantity per map.

**Dimensions**
- ~4000 px wide, ~400 px high.
- Max depth: ~200 px (Bedrock, scalable per map).

**Composition**
- Each level has a material theme.
- Level 1: Dirt, Stone, Coal, Iron.
- Level 2: larger, more materials, obstacles (Gas, Water,
  HardRock), Crystal.

**Placement**
- Resource veins off the direct path, to reward exploration.
- Obstacles near rich veins, to create choices.
- Finish line on the right, visible by a glow.
- Truck starts on the left.

**Level 1 tutorial**
- Forced level design.
- Iconographic signs in the world.
- Learn to mine, fill the bag, empty into the truck, drive,
  reach the exit.

**Constraints**
- The tunnel must be drivable (2 tiles high, slope ≤ 30°).
- Bedrock limits depth.
- No fail state: Archie doesn't die.
- Anti-softlock: exit accessible, enough resources for at least
  one upgrade, HardRock never blocks the only path.

---

## 15. Economy

**Raw resources**
| Resource | Value | Usage |
|---|---|---|
| Stone | $1 | Raw sale |
| Coal | $2 | Raw sale OR fuel |
| Iron | $5 | Raw sale OR nails |
| Crystal | $20 | Raw sale, high-end |

**Processed resources**
| Resource | Recipe | Value |
|---|---|---|
| Steel Ingot | 2 Coal + 2 Iron → 1 | $15 |
| Nails | 1 Iron + 1 Electricity → 4 | $25 (total) |
| Electricity | 1 Coal → 10 | Not sellable |

**Value chain**
Stone < Coal < Iron < Steel < Nails.
Selling raw = fast, low profit.
Processing = slow, more profitable, requires machines and
electricity.
Coal is the pivot: sold, or used for electricity or the foundry.

**V1 economic loop**
1. Mine → Stone, Coal, Iron.
2. Empty into the truck.
3. Return to HQ → sell raw OR process.
4. Buy upgrades / machines / robots.
5. Return to mine deeper, wider, richer.

**Indicative costs (to fine-tune)**
- Upgrade blocks: see section 11.
- Machines: Generator $150, Foundry $200, Nail Machine $300.
- Robots: unlock $100, upgrades $35–40.

---

## 16. Semi-Diegetic HUD

**Principle**
90% of info is in the world, 10% in a minimal HUD.

**In the world (diegetic)**
- Archie's bag visible, fills up.
- Blocks show their level (X/Y).
- Wall signs behind blocks with details.
- Machine shows its state.
- Finish line glow + sign.
- Discreet compass or glow to the right.

**Minimal HUD (semi-diegetic)**
- Resources: badges at the top, woodcut style.
- Dollars: coin + number.
- Discreet bag gauge.
- Truck storage gauge on the truck.

**Style**
- No generic rectangles.
- Engraving, stacked stones, wood.
- Consistent with the earthy palette.

---

## 17. Juice & Feedback

**Mining**
- 4 crack stages.
- Particles per material.
- Floating text "+1 Stone", "+1 Iron", etc.
- Sound: rising pitch.
- Light screen shake.
- Very short hit-stop on Crystal.

**Purchase**
- Metallic "clonk" + short jingle.
- Stars + floating text "+1 Pickaxe".
- Block sinks.
- Very light screen shake.

**Jump**
- Small "boing".
- Dust on landing.

**Truck**
- Engine: continuous low sound.
- Blocked: dull "clunk".
- Arrival: short fanfare.

**Deposit**
- Suction: whoosh sound.
- Floating text "+10 Stone".

**Ambience**
- Flickering lanterns.
- Water drips.
- Robot beeps.

---

## 18. Narration & Intro

**Intro**
- Archie in front of his Bore Inc. truck, at the foot of the
  mountain.
- No text. "Bore Inc." sign + arrow to the right.
- "Press Space to start" → any movement key.

**Contracts**
- Each level = connecting two villages.
- Visual: village at the start, village at the finish.
- Lanterns light up on arrival.

**Tone**
- Cozy, relaxing, never punishing.
- Archie is a small entrepreneur, not an epic hero.
- Tips appear via iconographic signs.

---

## 19. Technical

**Engine**
- **Ember** — custom 2D engine, written in Rust.
- Adaptable to the game's needs.
- Target: browser WASM.
- Optional native desktop support.

**Languages**
- **Rust** primarily.
- **Python** to generate assets (sounds, sprites) if needed.
- **SVG** for modular assets.

**Modular SVG assets**
- Cat split into independent parts: head, helmet, paws, body,
  tail, bag.
- Dropped items: pickaxes, vehicles, machines, robots.
- Each part is a separate SVG file.
- Composited at runtime.

**Ember systems to use / adapt**
- Tilemap.
- Collisions.
- Animation (sprite sheets + SVG).
- Particles.
- Audio.
- Saving.
- WASM build.

**Save (localStorage)**
- Dollars.
- Purchased upgrades.
- Purchased machines and levels.
- Unlocked robots.
- Level state (tunnel, mined resources).
- Seed of each map.
- Truck position on the map.

**Performance**
- Pixel-perfect rendering.
- Pixel-snapped camera following Archie or the truck.
- Truck physics: grid or smooth, to be decided with Ember.

**Short section**: Ember technical details go in a separate doc.

---

## 20. Tests

**Target**: ~120–140 tests.

**Breakdown by system**
- Mining: ~20 tests.
- Truck: ~15 tests.
- Robots: ~15 tests.
- Economy: ~20 tests.
- Upgrade blocks: ~15 tests.
- HQ: ~10 tests.
- Map: ~10 tests.
- Procedural generation: ~15 tests.
- Saving: ~10 tests.
- UI / HUD: ~5 tests.

**Integration tests**
- Resource quantity per map (per seed).
- Finish line accessibility.
- Anti-softlock.
- HQ ↔ Map ↔ Level loop.

---

## 21. Art & Audio

**Sprites to produce**
- Archie: idle, walk, jump, mine, deposit, drive, purchase,
  celebration.
- Truck: idle, rolling, blocked.
- Robots: 3 types × 4 personalities.
- Tiles: each material × 4 crack stages.
- Upgrade blocks: 4 sink stages + maxed version.
- Machines: Generator, Foundry, Nail Machine.
- HQ decor: warehouse, factory.
- Map decor: ground, peaks, mountain entrances.
- Iconographic signs.
- Particles.
- Dropped items.

**Palette**
- Earthy: ochres, browns, stone gray.
- Accents: cyan (crystal), amber (lantern).
- Robot: copper + glowing eyes.

**SFX**
- Mining (per material).
- Jump, landing.
- Deposit (suction).
- Purchase (clonk + jingle).
- Headbutt on block.
- Truck engine.
- Gas, water.
- Robots (beeps).
- Victory.
- Lantern ambience.

**Music**: stretch.

---

## 22. Final Scope

**V1 jam**
- 2 levels, procedural generation, tested seeds.
- 4 resources (Stone, Coal, Iron, Crystal).
- 4 obstacles (HardRock, Bedrock, Gas, Water).
- 3 robots, 4 personalities.
- Drivable truck, 1 truck.
- HQ with selling, 3 machines, ~8 upgrade blocks.
- Top-down map (pure top-down).
- Local save.
- Title + Intro + Game + HQ + Map.
- ~120–140 tests.
- WASM via Ember.

**Stretch post-jam**
- Extended processing.
- Jetpack.
- Robot driver.
- Multiple trucks.
- Lava, cave-ins, explosive gas.
- Prestige.
- Idle / offline.
- Music.
- 2.5D map.
- More levels, more robots, more machines.

**Out of jam scope**
- Idle / offline.
- Prestige.
- Massive procedural multi-levels.
- Menu, clicking, mouse.
