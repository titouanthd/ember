# Session 1 Report — Bore Inc. Design Doc

## Objective
Finalize the design document for **Bore Inc.**, a cozy incremental mining game built for the jam restriction **MOVEMENT INPUT ONLY**. The session focused on turning the initial concept into a complete, production-ready design doc covering core loop, systems, economy, tech stack, and scope.

## What Was Done

### 1. Core Concept Locked
- **Bore Inc.** is an incremental mining game where **Archie**, a cat entrepreneur, drills tunnels through mountains with robot employees.
- The goal is **not to mine everything** — it’s to **get the Bore Inc. truck to the finish line** on the other side of the mountain.
- Each level is a contract: connect two villages by piercing a tunnel.

### 2. Movement-Only Compliance Confirmed
Every interaction is driven by movement:
- **Mine**: push / tap against a tile.
- **Empty bag into truck**: tap the physical button on the truck bed.
- **Enter truck**: tap the truck door.
- **Drive truck**: left / right.
- **Buy upgrade**: jump and headbutt an upgrade block.
- **Finish level**: drive the truck to the finish line.
- **Start game**: any movement key.

No menus, no clicking, no mouse input.

### 3. Archie Defined
- Archie is the player character: a small cat entrepreneur with a hard hat and a visible bag.
- Controls: left/right movement, jump, tap.
- Stats planned: movement speed, pickaxe damage, mining speed, bag capacity, jump height, coyote time, jump buffer.
- Animations listed: idle, walk, jump, mine, deposit, drive, purchase, celebration.
- Feedback: meows on crystal, puffs on hard rock, helmet glow tied to pickaxe power.

### 4. Truck System Designed
- The truck is the real objective and the main level-design constraint.
- Starts at **2 tiles high**, upgradeable in height, length, storage, slope, speed.
- Max slope: **30°** initially.
- Can reverse, no fuel, no mining.
- Soft-blocks if the tunnel is too steep, too low, or obstructed.
- Archie taps the door to enter, jumps to exit.
- Robot driver and multiple trucks are stretch goals.

### 5. Level Structure and Generation
- Levels are **procedural**, with multiple seeds tested to select the best ones.
- Target size: **~4000×400 px**, max depth ~200 px (Bedrock, scalable per map).
- Level 1: Dirt, Stone, Coal, Iron.
- Level 2: larger, more materials, obstacles, Crystal.
- Entire map is mineable except Bedrock.
- Finish line visible via glow + sign.
- Tutorial level uses forced level design and iconographic signs.

### 6. Mining, Tiles, and Obstacles
- Tile types: Dirt, Stone, Coal, Iron, Crystal, HardRock, Bedrock, Capsule.
- Each tile has HP, drops, and 4 crack stages.
- Particles differ per material; Crystal gets hit-stop and flash.
- V1 obstacles: **HardRock**, **Bedrock**, **Gas**, **Water**.
- Stretch: Lava, cave-ins, explosive gas, unstable crystals, jetpack.

### 7. Robots
- 3 types: **Digger**, **Drill**, **Seeker**.
- 4 personalities: **Cautious**, **Independent**, **Greedy**, **Steady**.
- Robots mine automatically but do not collect; resources drop and Archie picks them up.
- Robots do not trigger pads or blocks.
- Unlocked via capsules in the mountain or hire blocks at HQ.
- Upgrades: speed, storage, unlock more types.

### 8. HQ and Upgrade Blocks
- HQ is a horizontal-scroll warehouse/factory.
- Zones: truck parking → selling → machines → upgrade blocks → exit to map.
- Upgrades are bought by **headbutting blocks** with Archie’s head.
- Blocks show level (X/Y); wall signs give details.
- Cost is **dollars only**.
- V1 blocks: pickaxe damage, mining speed, movement speed, bag capacity, truck storage, unlock robot, robot speed, robot storage.
- Maxed blocks sink and leave an engraved “MAX” plate.

### 9. Machines and Processing
- V1 machines:
  1. **Coal Generator** — coal → electricity.
  2. **Coal Foundry** — 2 Coal + 2 Iron → 1 Steel Ingot.
  3. **Nail Machine** — 1 Iron + 1 Electricity → 4 Nails.
- Each machine has 3 levels.
- Coal is consumed directly from storage.
- Workshops unlock progressively.

### 10. Top-Down Map
- Pure top-down for V1; 2.5D as stretch.
- Truck drives freely, Overcooked-style.
- Drive to a mountain entrance to load a level, or to the HQ to return.
- No menu, no clicking.
- Transitions use fade + loading + fade.

### 11. Economy
- Raw resources: Stone ($1), Coal ($2), Iron ($5), Crystal ($20).
- Processed: Steel Ingot ($15), Nails ($25 total), Electricity (not sellable).
- Value chain: Stone < Coal < Iron < Steel < Nails.
- Selling raw is fast but low profit; processing is slower but more profitable.
- Coal is the pivot resource: sell it, burn it for electricity, or use it in the foundry.

### 12. HUD and Feedback
- Semi-diegetic HUD: 90% in-world, 10% minimal UI.
- In-world: Archie’s bag, block levels, wall signs, machine states, finish line glow.
- Minimal HUD: resource badges, dollars, bag gauge, truck storage gauge.
- Juice: crack stages, particles, floating text, screen shake, hit-stop on crystal, purchase clonk, truck engine sounds, lantern ambience.

### 13. Technical Stack
- **Rust** primary language.
- **Ember** — custom 2D engine, adaptable to the game’s needs.
- Target: **browser WASM**.
- **Python** for generating assets if needed.
- **SVG** for modular assets: Archie split into head, helmet, paws, body, tail, bag; items, vehicles, machines, robots.
- Save via localStorage: dollars, upgrades, machines, robots, level state, seeds, truck position.
- ~120–140 tests planned across mining, truck, robots, economy, blocks, HQ, map, generation, saving, UI.

### 14. Final Scope
**V1 jam:**
- 2 levels, procedural, tested seeds.
- 4 resources, 4 obstacles, 3 robots, 4 personalities.
- 1 drivable truck.
- HQ with selling, 3 machines, ~8 upgrade blocks.
- Top-down map.
- Title + Intro + Game + HQ + Map.
- Local save, WASM via Ember.

**Stretch:**
- Extended processing, jetpack, robot driver, multiple trucks, lava, cave-ins, explosive gas, prestige, idle/offline, music, 2.5D map, more levels/robots/machines.

## Artifacts Produced
- **Final design.md** in English, ready to copy/paste.
- Complete sections: Pitch, Restriction Compliance, Inspirations, Scope, Core Loop, Archie, Truck, Robots, Mining & Tiles, Obstacles, Pads & Upgrade Blocks, HQ, Top-Down Map, Levels, Economy, Semi-Diegetic HUD, Juice & Feedback, Narration & Intro, Technical, Tests, Art & Audio, Final Scope.

## Next Steps
1. Prototype Ember integration.
2. Build Level 1 with procedural generation.
3. Test multiple seeds and select the best.
4. Implement first playable loop: mine → fill bag → empty into truck → drive → HQ → sell → upgrade → next level.
5. Write integration tests for resource quantity and anti-softlock.
