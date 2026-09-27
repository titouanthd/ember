# ember-wars — Design Document

> Inspiré de **Cartoon Wars** (Gamevil, 2009) : un hybride
> Tower Defense / RTS side-scrolling. Deux tours s'affrontent,
> chaque camp spawn des unités qui avancent automatiquement,
> le joueur gère une ressource et pilote une **catapulte** à tir
> balistique. Objectif : détruire la tour ennemie ou survivre.

Dernière mise à jour : Phase 25, après catapulte balistique +
chapitres Chengdu/Beijing + tree 28 nœuds + floating damage texts.
Statut : **V1 jouable, 15 niveaux, tree étendu, UI polishée.**

---

## 1. Pitch

**ember-wars** est un jeu d'affrontement en temps réel à deux camps,
vu de côté. Le joueur incarne le défenseur d'une tour et doit
accomplir l'objectif du niveau en :

1. **Spawnant des unités** depuis sa tour, qui avancent
   automatiquement vers l'ennemi et se battent toutes seules.
2. **Pilotant une catapulte** montée devant sa tour — la souris
   définit le **point de chute**, la catapulte résout la vélocité
   balistique. La catapulte ne peut **jamais** atteindre la tour
   ennemie (portée max 1000px, distance tour ≥ 1300px) : c'est une
   arme **défensive**, anti-unités.
3. **Gérant une ressource (Mana)** qui régénère passivement et
   gagne un bonus à chaque kill ennemi.
4. **Achetant des upgrades** dans l'arbre de progression, **entre
   les niveaux**, avec l'or gagné en jeu.

La partie se termine selon l'objectif du niveau :
- **Destroy** — détruire la tour ennemie (via les unités uniquement).
- **Survive(N)** — tenir N secondes sans que la tour joueur tombe.

**Thème visuel** : cinq chapitres, chacun avec une identité propre.

- **Chapitre 1 : Shanghai** — nuit urbaine, skyline de gratte-ciels
  à fenêtres jaunes, enseignes néon, ciel violet/rose.
- **Chapitre 2 : Guiyang** — aube brumeuse sur les montagnes
  karstiques, pagodes, rizières.
- **Chapitre 3 : Hong Kong** — coucher de soleil sur Victoria
  Harbour, néons cyan/magenta, asphalte mouillé.
- **Chapitre 4 : Chengdu** — brume verte du Sichuan, bambous,
  montagnes sacrées (Qingcheng).
- **Chapitre 5 : Beijing** — jade et or impériaux, murailles rouges,
  Temple du Ciel, Grande Muraille.

Tout le décor est **procédural** (aucun asset externe), dessiné en
code avec des palettes par chapitre.

**Unités** : stick men procéduraux avec animation 3-phases
(windup / strike / recovery), 6 armes distinctes (épée, gourdin,
arc, bâton doré, bombe, épée longue). Les dégâts sont appliqués au
moment du strike, pas au début de l'animation.

**Feedback** : floating damage numbers sur **toutes** les attaques
(melee, archer, catapulte, bomber AoE, tours, catapulte) et sur les
soins. Couleurs : bleu (dégâts joueur), rouge (dégâts ennemis),
vert (heal).

**Plateforme** : desktop, clavier + souris. Fenêtre 1600×900 par
défaut, redimensionnable.

---

## 2. Inspirations

- **Cartoon Wars** (Gamevil, 2009) — modèle principal : spawn
  d'unités, artillerie manuelle, Mana, upgrades.
- **Age of War** (Louissi, 2007) — vue de côté, tours à HP,
  vagues d'unités.
- **Worms / Scorched Earth** — balistique parabolique pour la
  catapulte (résolution de vélocité initiale).
- **Plants vs Zombies** — lisibilité des coûts / cooldowns.
- **Auto-battlers** — composition d'unités.
- **Blade Runner / Ghost in the Shell** — ambiance nocturne du
  chapitre Shanghai.
- **Hero** (Zhang Yimou) — palette rouge/or de Beijing.

---

## 3. Scope V1

| Élément | Implémenté | V2 (hors-scope) |
|---|---|---|
| **Campagne** | 5 chapitres × 3 niveaux | +chapitres |
| **Objectifs** | Destroy / Survive(N) | +Escort / +Boss |
| **Tours** | HP data-driven par niveau | +attaques de tour, +passifs |
| **Unités** | 6 types : Grunt, Brute, Archer, Healer, Bomber, Hero | +volants, +soigneurs avancés |
| **Stick men** | 3 phases, 6 armes, strike différé | +ragdoll, +cadavres persistants |
| **Catapulte** | Tir balistique, 3 tirs, HP, rebuild auto | +catapulte ennemie, +nouveaux tirs |
| **Mana** | Régén passive + bonus kill | +upgrades actifs |
| **Or** | Gagné sur kill, persistant entre niveaux | — |
| **Upgrade tree** | 28 nœuds, 4 branches, menu scrollable | +choix mutuellement exclusifs |
| **Déblocage d'unités** | Via l'arbre | — |
| **Progression** | Étoiles (1-3), chapitres verrouillés | +achievements |
| **IA** | Waves temporelles, agressivité data-driven, cible la catapulte | +IA adaptative |
| **Art** | Formes procédurales + 5 palettes | +textures |
| **Police** | DejaVu Sans Mono (Unicode) | +Noto Sans CJK |
| **Écrans** | Menu + modal upgrade, tous scrollables | +écran de fin enrichi |
| **Feedback** | Floating damage/heal texts, juice, screen shake | +sons |

**Tests actuels** : **~295 unit + ~20 integration = ~315 tests** dans `ember-wars`.

---

## 4. Core loop

### 4.1 Frame par frame (en jeu)

1. Lire Input (souris → catapulte, clavier → caméra / tir).
2. Tick Mana regen (dt × regen_rate, clampé au cap).
3. Tick Tower regen (si upgrade acheté).
4. Tick Catapult (cooldown + rebuild timer).
5. Tick cooldowns des unités (chaque type, chaque camp).
6. Tick Juice (particules, floating numbers, screen shake).
7. Scroll caméra manuel (flèches ← / →).
8. Si clic gauche hors UI → tir de catapulte (vise point de chute,
   résout vélocité balistique, applique crit si roll réussi).
9. IA : tick Mana, waves, décide spawn selon `AiConfig`.
10. **Combat** :
    - **Phase 1** : appliquer les pending attacks dont le strike
      est arrivé (fin du windup).
    - **Phase 2** : par unité — acquérir cible, avancer si pas de
      cible à portée, décider d'une nouvelle attaque si cooldown
      prêt.
11. Résoudre projectiles (avance + gravité + collision + AoE).
12. Résoudre morts → gains d'or + Mana + floating texts.
13. Détecter fin de niveau (objectif atteint).

### 4.2 Partie par partie

```
Menu (campagne) → clic sur un niveau débloqué → Playing
     → objectif atteint → Won → Écran de fin (étoiles + gold)
     → Enter → retour menu (progression sauvegardée)

Depuis le menu :
  Tab → modal Upgrade Tree (achats persistés, scrollable)
```

Pas de sauvegarde mid-partie. Chaque niveau repart de zéro (mêmes
stats, mêmes coûts). L'**or** et les **upgrades achetés** sont les
seuls éléments persistants.

---

## 5. Modèle de données (.ron)

### 5.1 `assets/chapters.ron`

**5 chapitres × 3 niveaux.** Chaque niveau référence une palette
et un `AiConfig` avec des waves temporelles.

```ron
[
    (
        id: "shanghai",
        name: "Shanghai",
        subtitle: "La ville qui ne dort jamais",
        levels: [
            (
                id: "sh_01",
                name: "Les quais",
                palette: "shanghai",
                width: 1600.0,
                tower_hp: 400.0,
                objective: Survive(45.0),
                reward_gold: 150.0,
                ai: (
                    aggression: 0.85,
                    mana_regen_mult: 0.85,
                    waves: [
                        (start_at: 0.0, priority: ["grunt"]),
                        (start_at: 20.0, priority: ["grunt", "brute"]),
                    ],
                ),
            ),
            // sh_02 "Le Bund" (Destroy, 1800px, 3 waves)
            // sh_03 "Pudong" (Destroy, 2000px, 5 waves)
        ],
    ),
    // gy_01..gy_03 Guiyang
    // hk_01..hk_03 Hong Kong
    // cd_01..cd_03 Chengdu
    // bj_01..bj_03 Beijing
]
```

**Tableau des 15 niveaux** :

| ID | Nom | Obj | Width | Tower HP | Waves |
|---|---|---|---|---|---|
| sh_01 | Les quais | Survive 45 | 1600 | 400 | 2 |
| sh_02 | Le Bund | Destroy | 1800 | 500 | 3 |
| sh_03 | Pudong | Destroy | 2000 | 600 | 5 |
| gy_01 | Les rizières | Survive 60 | 1700 | 500 | 3 |
| gy_02 | La montagne | Destroy | 1900 | 700 | 4 |
| gy_03 | La citadelle | Destroy | 2100 | 800 | 5 |
| hk_01 | Kowloon | Survive 50 | 1900 | 550 | 3 |
| hk_02 | Victoria Harbour | Destroy | 2100 | 700 | 4 |
| hk_03 | The Peak | Destroy | 2300 | 900 | 5 |
| cd_01 | Les bambous | Survive 65 | 2000 | 600 | 3 |
| cd_02 | Le temple Wuhou | Destroy | 2200 | 850 | 4 |
| cd_03 | Le mont Qingcheng | Destroy | 2400 | 1100 | 5 |
| bj_01 | La Cité Interdite | Survive 70 | 2100 | 750 | 4 |
| bj_02 | Le Temple du Ciel | Destroy | 2300 | 1000 | 5 |
| bj_03 | La Grande Muraille | Destroy | 2600 | 1300 | 6 |

**Notes** :
- `objective` : `Destroy` (détruire) ou `Survive(f32)` (tenir N secondes).
- `ai.waves[*].start_at` : temps de début en secondes.
- `ai.waves[*].priority` : ordre de préférence des unités.
- `ai.aggression` : divise le seuil de Mana (Hard = spawn plus tôt).
- `reward_gold` : or crédité à la victoire.
- `palette` : id dans `palettes.ron`.

### 5.2 `assets/palettes.ron`

**5 palettes**, une par chapitre.

```ron
[
    (
        id: "shanghai",
        style: Urban,
        sky_top: (0.03, 0.04, 0.12, 1.0),        // nuit violette
        sky_bottom: (0.35, 0.12, 0.28, 1.0),
        far_layer: (0.16, 0.18, 0.32, 1.0),
        mid_layer: (0.08, 0.10, 0.20, 1.0),
        ground_base: (0.06, 0.07, 0.12, 1.0),
        ground_accent: (0.95, 0.72, 0.35, 1.0),
        tower_player: (0.28, 0.60, 1.00, 1.0),   // bleu
        tower_enemy: (1.00, 0.30, 0.35, 1.0),    // rouge
        window_color: (1.00, 0.85, 0.40, 1.0),
        accent_light: (0.95, 0.42, 0.62, 1.0),
        neon_1: (0.95, 0.35, 0.55, 1.0),         // rose
        neon_2: (0.35, 0.85, 1.00, 1.0),         // cyan
        mist_color: (0.0, 0.0, 0.0, 0.0),
    ),
    // guiyang (Mountain, aube dorée + brume)
    // hongkong (Urban, coucher corail + néons forts)
    // chengdu (Mountain, brume verte + bambous)
    // beijing (Urban, rouge/or impérial)
]
```

**Style** : `Urban` (immeubles, fenêtres, néons) ou `Mountain`
(montagnes, pagodes, brume).

### 5.3 `assets/balance.ron`

```ron
(
    mana_start: 30.0,
    mana_max: 100.0,
    mana_regen: 10.0,
    mana_per_kill: 5.0,
    gold_per_kill: 3.0,
    gold_start: 0.0,
)
```

### 5.4 `assets/units.ron`

6 unités. Chacune a ses stats, son arme (déduite du `kind` par
`weapons.rs`), et éventuellement des comportements spéciaux
(`projectile`, `heal`, `suicide`, `max_alive`).

| Unité | Coût | Rôle | Arme | Débloqué par |
|---|---|---|---|---|
| Grunt | 15 | Melee basique | Épée courte | Défaut |
| Brute | 40 | Melee lourd, tank | Gourdin | Défaut |
| Archer | 30 | Ranged (projectile) | Arc | Arbre |
| Healer | 45 | Soigne allié le plus blessé | Bâton doré | Arbre |
| Bomber | 35 | Suicide AoE (70px) | Bombe | Arbre |
| Hero | 90 | Melee fort, max 1 vivant | Épée longue | Arbre |

### 5.5 `assets/upgrade_tree.ron`

**28 nœuds, 4 branches.**

```
                             [Awakening]  (racine, gratuit)
                                  |
        +-------------+-----------+-----------+-------------+
        |             |                       |             |
    DÉFENSE        ARMÉE                 CATAPULTE      ÉCONOMIE
    (col -4)      (col -2)                (col +2)      (col +4)
        |             |                       |             |
    HP I          Unit HP I              Range I        Mana I
        |             |                       |             |
    HP II         Unit HP II             Range II       Mana cap
        |             |                       |             |
    HP III        Unit speed             Catapult HP    Mana/kill
        |             |                       |             |
    Regen         Unlock Archer        Rebuild         Gold I
        |             |                       |             |
    Regen II      Unlock Healer          Dmg I         Gold II
        |             |                       |             |
    Fortress      Unlock Bomber        Fire rate       Gold III
                      |                       |
                  Unlock Hero           Multi-shot
                                              |
                                          Crit
```

**Effets disponibles** (19 variants) :

```rust
pub enum UpgradeEffect {
    None,
    TowerHpMult(f32),           // mult multiplicatif
    TowerRegen(f32),            // additif (HP/s)
    Fortress,                   // 2e barre HP (V2)
    UnitHpMult(f32),
    UnitDamageMult(f32),
    UnitSpeedMult(f32),         // NOUVEAU
    UnlockUnit(String),
    TurretDamageMult(f32),      // catapult damage
    TurretFireRateMult(f32),    // <1 = plus rapide
    MultiShot(u32),
    TurretCritChance(f32),      // NOUVEAU
    ManaRegenMult(f32),
    ManaCapMult(f32),
    ManaOnKillAdd(f32),         // NOUVEAU
    GoldPerKillAdd(f32),
    CatapultRangeAdd(f32),      // NOUVEAU
    CatapultHpMult(f32),        // NOUVEAU
    CatapultRebuildMult(f32),   // NOUVEAU (<1 = plus rapide)
}
```

**Validation** au chargement : ids uniques, requires existants,
exactement 1 racine, pas de cycle (Kahn).

---

## 6. Systèmes

Chaque système est un module de `src/`. Ordre d'exécution = §4.1.

### 6.1 `components.rs` — Types de base

```rust
pub enum Team { Player, Enemy }
pub enum Shape { Rect, Circle, Triangle }
pub struct UnitId(pub u32);
pub struct UnitIdGen { next: u32 }

pub struct Unit {
    pub id: UnitId,
    pub kind: String,
    pub team: Team,
    pub pos: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub damage: f32,
    pub attack_cd: f32,
    pub attack_cd_max: f32,
    pub heal_cd: f32,
    pub hit_flash: f32,
    pub pose_phase: f32,
    pub target: Option<UnitId>,
    pub pending_attack: Option<PendingAttack>,
}

pub struct PendingAttack {
    pub target: AttackTarget,
    pub damage: f32,
    pub time_to_hit: f32,
}

pub enum AttackTarget {
    Unit(UnitId),
    Tower(Team),
    Catapult(Team),      // NOUVEAU
}

pub struct Projectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub gravity: f32,    // NOUVEAU : 0 = ligne droite, > 0 = balistique
    pub damage: f32,
    pub radius: f32,
    pub team: Team,
    pub kind: ProjectileKind,
    pub pierce_remaining: u8,
    pub color: Color,
}

pub struct Tower { ... }
```

### 6.2 `units.rs` — Stats et spawn

- Charge `units.ron` via `OnceLock`.
- `spawn_unit(gen, kind, team, pos, hp_mult, damage_mult) -> Option<Unit>`.
- `can_spawn(kind, team, units)` respecte `max_alive`.
- `alive_count(kind, team, units)`.

### 6.3 `catapult.rs` — Catapulte balistique

Remplace l'ancienne `tower.rs::Turret`.

```rust
pub struct Catapult {
    pub pos: Vec2,           // bas-centre, posé au sol
    pub hp: f32,
    pub max_hp: f32,
    pub cooldown: f32,
    pub shot_kind: ShotKind, // Basic | Piercing | Explosive
    pub fire_rate_mult: f32,
    pub multi_shot: u32,
    pub rebuild_timer: f32,  // > 0 = détruite
    pub rebuild_time: f32,   // configurable via upgrades
    pub max_range: f32,      // configurable via upgrades
    pub aim_target: Vec2,    // point de chute visé
}
```

**Constantes de base** :
- `CATAPULT_HP = 300.0`
- `CATAPULT_REBUILD_TIME = 10.0`
- `CATAPULT_OFFSET_X = 150.0` (devant la tour joueur)
- `CATAPULT_MIN_RANGE = 150.0`
- `CATAPULT_MAX_RANGE = 1000.0` (base — augmentable via upgrades)
- `CATAPULT_FIRE_COOLDOWN = 0.9`
- `CATAPULT_BASE_DAMAGE = 14.0`
- `CATAPULT_H_SPEED = 650.0` (fixe, détermine le temps de vol)
- `CATAPULT_GRAVITY = 900.0`
- `MULTI_SHOT_SPREAD_PX = 20.0`
- `CATAPULT_HITBOX_W = 50.0`, `CATAPULT_HITBOX_H = 60.0`
- `LAUNCH_HEIGHT = 45.0`

**Trajectoire balistique** :

```rust
// Résout vy pour qu'un projectile passe par `target`.
//   y(t) = dy = vy*t + 0.5*g*t²   avec t = dx / vx
//   →  vy = (dy - 0.5*g*t²) / t
pub fn solve_trajectory(&self, target: Vec2) -> Option<Vec2>
```

**Multi-tir** : N projectiles espacés de `MULTI_SHOT_SPREAD_PX` en X
de cible (donc arcs quasi-parallèles). 20px entre le tir de gauche
et le tir central.

**Reconstruction** : `take_damage` met `rebuild_timer` à
`rebuild_time`. `tick` décrémente, à 0 restaure `hp = max_hp`.

**Méthodes clés** :
- `configure(max_range, max_hp, rebuild_time)` — applique les upgrades.
- `is_alive() / is_rebuilding() / is_ready()`.
- `aim(mouse_world)` — clamp à [MIN_RANGE, max_range].
- `solve_trajectory(target)` — vélocité balistique.
- `preview_points(steps)` — échantillonne l'arc pour le rendu.
- `try_fire_multi(team, damage_mult)` — spawn N projectiles.
- `cycle_shot_kind()` — Basic → Piercing → Explosive.

**Important** : la catapulte ne peut JAMAIS atteindre la tour
ennemie. Portée max après tous les upgrades = 1500px, et la
distance minimale catapulte → tour ennemie est de 1330px (sur
sh_01, niveau le plus court). Test d'intégration
`catapult_never_reaches_enemy_tower` vérifie cette invariant.

### 6.4 `combat.rs` — Targeting, attaques, projectiles, juice

**`strike_delay() -> f32`** : `ATTACK_DURATION * WINDUP_END` = 0.165 s.

**`acquire_target`** : cible par priorité :
1. Unité ennemie la plus proche à portée.
2. Si catapulte vivante et plus proche que la tour → `Catapult`.
3. Sinon tour ennemie si à portée.

**`resolve_attacks`** — boucle à deux phases :

**Phase 1 — pending attacks** : applique les `PendingAttack` dont
`time_to_hit <= 0`. Ranged → spawn projectile. Bomber → AoE. Melee
→ damage direct. Reset pending.

**Phase 2 — décision + mouvement** :
- **Healer** : soigne l'allié le plus blessé à portée. Si rien à
  soigner, avance (jusqu'à `enemy_tower.x - HEALER_MIDLINE_MARGIN`).
- **Autres** : `acquire_target`.
  - Pas de cible → avance (`speed * unit_speed_mult` si joueur).
  - Cible → si `pending_attack.is_none()` et `attack_cd == 0` →
    crée pending, arme `attack_cd = max(stats.attack_cooldown,
    ATTACK_DURATION)`.

**`tick_unit_timers`** : décrémente `attack_cd`, `heal_cd`,
`hit_flash`, incrémente `pose_phase`. **Ne consomme PAS**
`pending_attack`.

**`resolve_projectiles`** : avance (avec gravité si > 0),
collision unité, tour, catapulte, AoE explosive, pierce.

**Floating texts** : `apply_damage_to_unit` /
`apply_damage_to_tower` / `apply_damage_to_catapult` / healer
spawn un floating text à chaque application.
- Dégâts joueur → `DMG_COLOR_PLAYER` (bleu clair).
- Dégâts ennemis → `DMG_COLOR_ENEMY` (rouge clair).
- Heal → `HEAL_COLOR` (vert clair).

### 6.5 `mana.rs` — Ressource

```rust
pub struct ManaPool { pub current: f32, pub max: f32, pub regen: f32 }
```

- `tick(dt)`, `try_spend(amount)`, `gain(amount)`, `fraction()`.
- Un pool par camp (joueur + IA).

### 6.6 `ai.rs` — IA adverse

`decide_spawn(ai_mana, ai_units, player_units, ai_config, elapsed,
cooldowns)`.

Détermine la wave active = dernière avec `start_at <= elapsed`.
Ordre défensif si `ai_alive + 2 < player_alive`. Prend le premier
type abordable, hors cooldown, `can_spawn`.

### 6.7 `upgrades.rs` — UpgradeTree

`UpgradeTreeDefs` (28 nœuds, 4 branches) +
`UpgradeTree { gold, purchased }` sérialisable.

**Méthodes** : `is_purchased`, `is_accessible`, `can_buy`, `buy`,
`grant` (tests), et agrégations : `tower_hp_mult`, `tower_regen`,
`has_fortress`, `unit_hp_mult`, `unit_damage_mult`,
`unit_speed_mult`, `unlocked_units(defaults)`,
`turret_damage_mult`, `turret_fire_rate_mult`, `multi_shot_count`,
`turret_crit_chance`, `mana_regen_mult`, `mana_cap_mult`,
`mana_on_kill_add`, `gold_per_kill_add`, `catapult_range_add`,
`catapult_hp_mult`, `catapult_rebuild_mult`.

**Agrégation** : mults se multiplient, additifs s'additionnent,
unlock s'unissent.

### 6.8 `progress.rs` — Persistance

```rust
pub struct Progress {
    pub stars: HashMap<String, u8>,   // niveau → étoiles (0-3)
    pub tree: UpgradeTree,             // gold + nœuds achetés
}
```

- `record_win(level_id, stars, gold_earned)` : meilleur score +
  crédite la bank.
- `is_level_unlocked(level_id)` : premier niveau, ou précédent
  complété.
- `is_chapter_unlocked(chapter_idx)` : tous les niveaux du précédent
  complétés (≥1⭐).
- `stars_for_victory(tower_hp_fraction)` : 1⭐ win, 2⭐ (≥50%),
  3⭐ (≥90%).
- Sauvegarde dans `progress.ron` (gitignoré).

### 6.9 `camera.rs` — Caméra locale

```rust
pub struct Camera2D { pub x: f32, pub viewport_w: f32, pub map_w: f32 }
```

- `scroll(dt, dir)` : contrôle manuel (flèches ← / →).
  `CAMERA_SCROLL_SPEED = 900.0` px/s.
- `clamp()` : borne à `[0, max_x]`.
- `world_to_screen` / `screen_to_world`.

### 6.10 `layout.rs` — Layout runtime

- `scr_w()`, `scr_h()`, `cx()`, `cy()` runtime.
- `SPAWN_BAR_H = 90.0`.
- **Sol responsive** : `ground_y = viewport_h * GROUND_Y_RATIO`.

### 6.11 `textures.rs` — Décor procédural

Toutes les fonctions de dessin sont dans ce module. Aucun asset
externe. Utilise une `Palette` et un `visual_seed`.

**Fonctions publiques** :
- `draw_sky(palette, ground_y)` — dégradé + étoiles (Urban).
- `draw_far_layer(palette, cam_x, ground_y, seed)` — skyline
  lointaine ou montagnes, parallax 0.25.
- `draw_mid_layer(palette, cam_x, ground_y, seed)` — skyline proche
  ou collines, parallax 0.55.
- `draw_ground(palette, cam_x, ground_y, vh)` — sol + pavés / herbe
  + néons au sol (Urban).
- `draw_mist(palette, ground_y)` — bande de brume (Mountain).
- `draw_tower(player, palette, world_x, cam_x, ground_y, shake)`.

**Hash déterministe** : `hash_u32(seed, x)` (murmur-inspired).

### 6.12 `juice.rs` — Feedback visuel

```rust
pub struct Juice {
    pub particles: Vec<Particle>,
    pub floating_texts: Vec<FloatingText>,
    pub screen_shake: ScreenShake,
    pub flashes: Vec<Flash>,
}
```

- `tick(dt)` : particules (gravité + drag), floating texts (montée
  + fade), shake (décroissance), flashes (fade).
- `shake(intensity, duration)` : remplace si plus fort.
- `flash(color, duration)` : plein écran semi-transparent.
- `spawn_hit_spark(pos, color)` : petit particle au hit.
- `spawn_death_burst(pos, color, count)` : explosion de particles.
- `spawn_floating_text(pos, text, color)` : texte qui monte.
- `draw_particles`, `draw_floating_texts`, `draw_flashes`.

### 6.13 `fonts.rs` — Police Unicode

- `load_default_font().await -> Option<Font>` : charge DejaVu Sans
  Mono depuis `assets/DejaVuSansMono.ttf` et le définit comme
  police par défaut.
- `draw_text_regular` / `draw_text_bold` (double-draw pour simuler
  le gras) / `measure_regular` / `measure_bold`.

### 6.14 `menu.rs` — Campagne

- Disposition **linéaire verticale** : 5 chapitres empilés, chacun
  avec header + séparateur + 3 niveaux en dessous.
- **Scrollable** : molette, flèches ↑↓. Header (titre + gold) et
  footer (hints) restent fixes — **fond opaque** pour que le scroll
  passe derrière.
- **Scrollbar verticale** discrète entre les cartes et le panneau
  d'info (apparaît seulement si le contenu déborde).
- **Panneau d'info** à droite : chapter, nom, objectif, étoiles
  colorées, reward. Si verrouillé → raison précise.
- Étoiles : 4 paliers de couleur (gris/bronze/argent/or), pulse
  subtil sur les étoiles gagnées.

### 6.15 `ui/spawn_bar.rs` — Barre de spawn

- 6 slots en bas de l'écran.
- Chaque slot : nom, coût Mana, état (locked / ready / no mana /
  cooldown / max).
- Cooldown overlay (voile noir proportionnel).

### 6.16 `ui/upgrade_modal.rs` — Modal arbre

- Ouverte par Tab depuis le **menu** (pas en jeu).
- 28 nœuds disposés en grille (col -4..+4, row 0..8).
- Connexions entre nœuds.
- **Scrollable** : molette, Page Up/Down, Home/End.
- **Auto-scroll** vers le nœud focus quand on navigue au clavier.
- **Scrollbar verticale** à droite du tree area si nécessaire.
- Description en bas (nom, effet, état, coût, requires).
- Navigation : flèches (find_neighbor), Entrée (achat), souris.
- **Header + description dessinés en dernier** pour masquer le
  débordement vertical du tree.

### 6.17 `systems.rs` — State struct `Game`

```rust
pub struct Game {
    pub phase: Phase,
    pub player_tower: Tower,
    pub enemy_tower: Tower,
    pub units: Vec<Unit>,
    pub projectiles: Vec<Projectile>,
    pub player_mana: ManaPool,
    pub enemy_mana: ManaPool,
    pub player_upgrades: UpgradeTree,   // snapshot au démarrage
    pub catapult: Catapult,
    pub camera: Camera2D,
    pub player_cooldowns: HashMap<String, f32>,
    pub enemy_cooldowns: HashMap<String, f32>,
    pub id_gen: UnitIdGen,
    pub rng: Rng,
    pub level: &'static LevelConfig,
    pub palette: &'static Palette,
    pub visual_seed: u32,
    pub balance: &'static Balance,
    pub stats: GameStats,
    pub ground_y: f32,
    pub juice: Juice,
}

pub enum Phase { Playing, Won, Lost }
```

**Méthodes clés** :
- `new(ctx, level_id, seed, progress) -> Result<Self, String>` —
  configure la catapulte avec les upgrades du progress.
- `tick(dt, mouse_world, mouse_left_pressed, camera_scroll)`.
- `try_player_spawn(kind) -> bool`.
- `survive_remaining() -> Option<f32>`.
- `tower_hp_fraction() -> f32`.
- `check_end_condition()`.

### 6.18 `main.rs` — Boucle

- `AppScreen { Menu, Game }`.
- `window_conf()` : 1600×900, redimensionnable.
- Charge la police DejaVu au démarrage.
- Charge `Progress` depuis `progress.ron`.
- Dispatch menu / game.
- Touches globales : Échap, Tab, R, Entrée, 1/2/3 (ShotKind).

---

## 7. State struct & Phases

### 7.1 Machine à états

```
AppScreen::Menu
    ↓ clic sur niveau débloqué
AppScreen::Game
    Phase::Playing
        ↓ objectif atteint
    Phase::Won
        ↓ Enter / R
AppScreen::Menu

AppScreen::Menu
    ↓ Tab
Modal Upgrade Tree (jeu pas lancé)
    ↓ Tab / Échap
AppScreen::Menu
```

### 7.2 Timers

- `attack_cd`, `heal_cd` : f32 qui décrémente.
- `pending_attack.time_to_hit` : f32 qui décrémente.
- `cooldowns[kind]`, `catapult.cooldown`, `catapult.rebuild_timer` :
  f32 qui décrémente.

**Note** : le pattern « cooldown f32 qui décrémente » est utilisé
dans **4 jeux** (Asteroids, Bullet Hell, Zhuo Ji, ember-wars).
Extraction `Cooldown` dans `ember_core::time` planifiée.

---

## 8. Structure des fichiers

```
games/ember-wars/
├── DESIGN.md
├── .env
├── .gitignore (progress.ron ignoré)
├── Cargo.toml
├── assets/
│   ├── chapters.ron         # 5 chapitres × 3 niveaux
│   ├── palettes.ron         # 5 palettes
│   ├── balance.ron
│   ├── units.ron            # 6 unités
│   ├── upgrade_tree.ron     # 28 nœuds, 4 branches
│   └── DejaVuSansMono.ttf
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config.rs
│   ├── components.rs
│   ├── units.rs
│   ├── stickman.rs
│   ├── weapons.rs
│   ├── combat.rs
│   ├── mana.rs
│   ├── catapult.rs          # remplace tower.rs
│   ├── ai.rs
│   ├── upgrades.rs
│   ├── progress.rs
│   ├── camera.rs
│   ├── layout.rs
│   ├── textures.rs
│   ├── juice.rs
│   ├── fonts.rs
│   ├── render.rs
│   ├── menu.rs
│   ├── systems.rs
│   └── ui/
│       ├── mod.rs
│       ├── spawn_bar.rs
│       └── upgrade_modal.rs
└── tests/
    └── game_scenarios.rs
```

---

## 9. Extractions moteur anticipées

| Primitive | Occurrences | Action |
|---|---|---|
| `Cooldown` (f32 qui décrémente) | 4 (Asteroids, BH, Zhuo Ji, EW) | **Prêt à extraire** |
| `Camera2D` | 1 (EW) | Local |
| `AutoCombat` | 1 (EW) | Local |
| `Stickman` | 1 (EW) | Local |
| `Weapons` procédurales | 1 (EW) | Local |
| `PendingAttack` | 1 (EW) | Local |
| `Catapult` (artillerie balistique) | 1 (EW) | Local |
| `Spawner` | 1 (EW) | Local |
| `UpgradeTree` | 1 (EW) | Local |
| `Juice` | 2 (BH, EW) | Attendre 3e |
| `Parallax` | 1 (EW) | Local |
| Police Unicode via `set_default_font` | 2 (FreeCell, EW) | Attendre 3e |
| Menu campagne scrollable | 1 (EW) | Local |
| **Floating damage texts** | 1 (EW) | Local |

---

## 10. Stratégie de tests

Objectif actuel : **~295 tests unit + ~20 integration = ~315 tests**
dans `ember-wars`.

### 10.1 Couverture par module

| Module | Tests |
|---|---|
| `components.rs` | 5 |
| `units.rs` | 18 |
| `stickman.rs` | 22 |
| `weapons.rs` | 9 |
| `catapult.rs` | 27 |
| `combat.rs` | 44 |
| `mana.rs` | 4 |
| `ai.rs` | 12 |
| `upgrades.rs` | 35 |
| `progress.rs` | 15 |
| `camera.rs` | 17 |
| `textures.rs` | 13 |
| `juice.rs` | 15 |
| `systems.rs` | 25 |
| `menu.rs` | 10 |
| `ui/spawn_bar.rs` | 3 |
| `ui/upgrade_modal.rs` | 13 |
| **Total** | **~295** |

### 10.2 Tests d'intégration (`tests/game_scenarios.rs`)

1. `player_wins_by_destroying_enemy_tower`
2. `player_wins_by_surviving`
3. `player_loses_when_tower_destroyed`
4. `phase_stays_playing_while_both_towers_alive`
5. `turret_kill_credits_bank_gold`
6. `progress_tree_grants_unlock_at_start`
7. `bomber_explodes_and_damages_neighbors`
8. `levels_load_and_run` (15 niveaux)
9. `progress_unlocks_next_level_after_completion`
10. `chapter_2_unlocked_only_after_chapter_1_cleared`
11. `chapter_3_unlocked_only_after_chapter_2_cleared`
12. `chapter_4_unlocked_only_after_chapter_3_cleared`
13. `chapter_5_unlocked_only_after_chapter_4_cleared`
14. `stars_scale_with_tower_hp`
15. `chapters_ron_loads` (5 chapitres)
16. `catapult_never_reaches_enemy_tower` (invariant clé)
17. `upgrade_tree_ron_loads` (28 nœuds)
18. `units_ron_loads`

### 10.3 Ce qu'on ne teste PAS

- Rendu visuel (pas de snapshot).
- Le ressenti (playtest manuel).

---

## 11. Pièges connus / décisions délicates

### 11.1 Hérités du workspace

- `Input::from_macroquad_with_keys` peuple les 3 vecteurs.
- Clavier AZERTY vs QWERTY : scancodes QWERTY.
- `manual_range_contains` : `(a..=b).contains(&x)`.
- `manual_is_multiple_of` : `r.is_multiple_of(3)`.
- `needless_borrows_for_generic_args` : `draw_text(format!(...))`.
- `clippy::collapsible_if` : let-chains stables.
- `clippy::too_many_arguments` : bundler en tuple.
- `draw_right` / `draw_left` : `font_size: u16`.
- `gen` réservé en édition 2024.
- `should_implement_trait` sur `next()` → `next_id()`.
- `vec_init_then_push` → `vec![...]`.
- E0502 sur `a[i].field = a[i].other * x`.

### 11.2 Spécifiques à ember-wars

- **UnitId stable** : `target` pointe vers un `UnitId`, jamais un
  index.
- **Catapulte ≠ sol** : launch_origin à `pos.y - LAUNCH_HEIGHT`.
- **Comparer les difficultés sur les HP déployés**, pas le nombre.
- **Sol responsive** : `ground_y = viewport_h * GROUND_Y_RATIO`.
- **`palettes.ron` doit commencer par `[`**.
- **Strike différé** : ne JAMAIS clear `pending_attack` dans
  `tick_unit_timers` — c'est la Phase 1 qui le consomme.
- **`attack_cd` minimum = `ATTACK_DURATION`**.
- **`strike_delay = ATTACK_DURATION * WINDUP_END`**.
- **`REST_ARM_R` / `REST_ARM_L` partagés**.
- **Phase d'anim = `attack_cd_max - attack_cd`**.
- **Bomber** : `detonate_bomber` prend l'index de l'**attaquant**.
- **Healer midline** : `HEALER_MIDLINE_MARGIN = 300.0`.
- **Mouvement** : le cooldown n'empêche pas l'avance si pas de
  cible à portée.
- **Catapulte ne peut jamais toucher la tour ennemie** — invariant
  testé dans `catapult_never_reaches_enemy_tower`.
- **Modal rect** : `modal_rect_for(vw, vh)` testable, `modal_rect()`
  wrapper runtime.
- **Menu draw order** : chapitres → scrollbar → panneau info →
  header → footer. Header/footer ont un fond opaque pour masquer le
  scroll.
- **Upgrade modal draw order** : tree (connexions + nœuds) →
  scrollbar → header → description. Header/description masquent le
  débordement.
- **Floating texts** : spawn dans les fonctions `apply_damage_*`,
  pas au call site. Comme ça toutes les attaques en bénéficient
  automatiquement.

### 11.3 Décisions validées

- ✅ Nom : `ember-wars`.
- ✅ Campagne 5 chapitres × 3 niveaux.
- ✅ Objectifs : Destroy / Survive(N).
- ✅ **Catapulte balistique** (remplace la tourelle).
- ✅ **Catapulte = structure destructible** (300 HP, rebuild 10s).
- ✅ **Multi-tir resserré** (20px entre arcs).
- ✅ Mana = régén passive + bonus kill.
- ✅ IA = waves temporelles + agressivité data-driven.
- ✅ **Upgrades dans le menu** (Tab).
- ✅ **Or persistant** dans `progress.ron`.
- ✅ **Étoiles** (1-3) par niveau.
- ✅ **Chapitres débloqués séquentiellement**.
- ✅ **Menu linéaire scrollable** + scrollbar.
- ✅ **Modal upgrade scrollable** (molette + clavier).
- ✅ **Décor procédural** avec palettes.
- ✅ **Police DejaVu Sans Mono** (Unicode).
- ✅ **UnitId(u32) monotone**.
- ✅ **Stick men animés** (3 phases).
- ✅ **6 armes procédurales**.
- ✅ **Strike différé**.
- ✅ **Floating damage/heal texts partout**.
- ✅ **Fenêtre 1600×900** au lancement.

---

## 12. Hors-scope / Open questions

### 12.1 V2

- **Stick figures ragdoll** à la mort.
- **Cadavres persistants**.
- **Sons** (`AudioClip` existe, mais rien).
- **Fortress effect** (2e barre HP) — présent dans l'arbre mais pas
  appliqué.
- **Catapulte ennemie** (symétrie).
- **Nouveaux ShotKind** (Slow, Poison).
- **Upgrades mutuellement exclusifs** (arbres de choix).
- **Nouvelles unités** (7e, 8e...).
- **Plus de chapitres**.

### 12.2 Open questions

1. **Chapitre 6** : Nanjing, Xi'an, Suzhou ?
2. **Équilibre Beijing bj_03** : 6 waves, aggression 1.65 — trop
   dur ?
3. **Reconstruire la catapulte** : 10s c'est trop long / trop court ?
4. **Catapulte ennemie** : est-ce qu'on l'ajoute pour symétrie ?

---

## 13. Prochaines étapes

### Session playtest — Tuning
- Vérifier la difficulté des 5 Survive (triviaux avec catapulte ?).
- Vérifier que bj_03 reste gagnable.
- Tester le rebuild 10s en conditions réelles.

### Session G — Catapulte ennemie (symétrie)
- L'IA aurait sa propre catapulte.
- Le joueur doit gérer 2 menaces.

### Session H — Upgrades mutuellement exclusifs
- 2 choix par branche.
- Décision meaningful à chaque palier.

### Session I — Nouveaux ShotKind
- Slow, Poison.
- Plus de variété dans le choix 1/2/3.

### Session refacto
- Extraction `Cooldown` (4e occurrence).
- DESIGN.md à rafraîchir au fil de l'eau.

---

## 14. Changelog

### Phase 25 (session actuelle)
- Catapulte balistique remplace la tourelle.
- 2 chapitres (Chengdu, Beijing).
- Tree 18 → 28 nœuds, 6 nouveaux effets.
- Floating damage/heal texts partout.
- Modal upgrade scrollable.
- Fenêtre 1600×900.
- Menu : scrollbar + fix superposition header.

### Phase 23 (session précédente)
- Refonte UI menu (Session A).
- Chapitre Hong Kong.
- Stick men 3-phases + 6 armes.
- Strike différé.
- Police DejaVu Sans Mono.
- Sol responsive.
- Juice (particles, shake, flashes).
- Caméra manuelle.
- Upgrade tree (18 nœuds originel).