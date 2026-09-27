# ember-wars — Design Document

> Inspiré de **Cartoon Wars** (Gamevil, 2009) : un hybride
> Tower Defense / RTS side-scrolling. Deux tours s'affrontent,
> chaque camp spawn des unités qui avancent automatiquement,
> le joueur gère une ressource et vise manuellement avec une
> tourelle. Objectif : détruire la tour ennemie ou survivre.

Dernière mise à jour : Phase 23, après Session F (stick men + armes + strike différé).
Statut : **V1 jouable avec animations complètes. Prêt pour polish UI + chapitre 3.**

---

## 1. Pitch

**ember-wars** est un jeu d'affrontement en temps réel à deux camps,
vu de côté. Le joueur incarne le défenseur d'une tour et doit
accomplir l'objectif du niveau en :

1. **Spawnant des unités** depuis sa tour, qui avancent
   automatiquement vers l'ennemi et se battent toutes seules.
2. **Visant manuellement** avec une tourelle montée sur sa tour
   (souris → angle, clic → tir), pour soutenir ses unités.
3. **Gérant une ressource (Mana)** qui régénère passivement et
   gagne un bonus à chaque kill ennemi.
4. **Achetant des upgrades** dans l'arbre de progression, **entre
   les niveaux** (voir §3), avec l'or gagné en jeu.

La partie se termine selon l'objectif du niveau :
- **Destroy** — détruire la tour ennemie.
- **Survive(N)** — tenir N secondes sans que la tour joueur tombe.

**Thème visuel** : deux chapitres, chacun avec une identité propre.

- **Chapitre 1 : Shanghai** — nuit urbaine, skyline de gratte-ciels
  à fenêtres jaunes, enseignes néon, ciel violet/rose.
- **Chapitre 2 : Guiyang** — aube brumeuse sur les montagnes
  karstiques, pagodes, rizières.

Tout le décor est **procédural** (aucun asset externe), dessiné en
code avec des palettes par chapitre.

**Unités** : stick men procéduraux avec animation 3-phases
(windup / strike / recovery), 6 armes distinctes (épée, gourdin,
arc, bâton doré, bombe, épée longue). Les dégâts sont appliqués au
moment du strike, pas au début de l'animation.

**Plateforme** : desktop, clavier + souris.

---

## 2. Inspirations

- **Cartoon Wars** (Gamevil, 2009) — modèle principal : spawn
  d'unités, tourelle manuelle, Mana, upgrades.
- **Age of War** (Louissi, 2007) — vue de côté, tours à HP,
  vagues d'unités.
- **Plants vs Zombies** — lisibilité des coûts / cooldowns.
- **Auto-battlers** — composition d'unités, mais pas le tour par tour.
- **Blade Runner / Ghost in the Shell** — pour l'ambiance nocturne
  du chapitre Shanghai.

---

## 3. Scope V1

| Élément | Implémenté | V2 (hors-scope) |
|---|---|---|
| **Campagne** | 2 chapitres × 3 niveaux (Shanghai, Guiyang) | +chapitres |
| **Objectifs** | Destroy / Survive(N) | +Escort / +Boss |
| **Tours** | HP data-driven par niveau | +attaques de tour, +passifs |
| **Unités** | 6 types : Grunt, Brute, Archer, Healer, Bomber, Hero | +volants, +soigneurs avancés |
| **Stick men** | ✅ 3 phases, 6 armes, strike différé | +ragdoll, +cadavres persistants |
| **Tourelle** | 3 tirs : Basic, Piercing, Explosive | +tirs élémentaires, +charge |
| **Mana** | Régén passive + bonus kill | +upgrades actifs |
| **Or** | Gagné sur kill, **persistant entre niveaux** | — |
| **Upgrade tree** | **18 nœuds, 4 branches**, achetés dans le menu | +arbre arborescent |
| **Déblocage d'unités** | Via l'arbre (Archer, Healer, Bomber, Hero) | — |
| **Progression** | Étoiles (1-3) par niveau, chapitres verrouillés | +achievements |
| **IA** | Waves temporelles, agressivité data-driven | +IA adaptative |
| **Art** | Formes procédurales + palettes par chapitre | +textures, +stick figures détaillées |
| **Police** | DejaVu Sans Mono (Unicode complet) | +Noto Sans CJK |
| **Écrans** | Menu campagne scrollable + modal upgrade | +écran de fin enrichi |

**Tests actuels** : **251 unit + 14 integration = 265 tests** dans `ember-wars`.

---

## 4. Core loop

### 4.1 Frame par frame (en jeu)

1. Lire Input (souris → tourelle, clavier → caméra / tir).
2. Tick Mana regen (dt × regen_rate, clampé au cap).
3. Tick Tower regen (si upgrade acheté).
4. Tick cooldowns des unités (chaque type, chaque camp).
5. Tick cooldown de la tourelle.
6. Tick Juice (particules, floating numbers, screen shake).
7. Scroll caméra manuel (flèches ← / →).
8. Si clic gauche hors UI → tir de tourelle (selon type
   sélectionné), consomme le cooldown.
9. IA : tick Mana, waves, décide spawn selon `AiConfig`.
10. **Combat** :
    - **Phase 1** : appliquer les pending attacks dont le strike est
      arrivé (fin du windup), appliquer dégâts / projectiles / bomber.
    - **Phase 2** : par unité — acquérir cible, avancer si pas de
      cible à portée, décider d'une nouvelle attaque si cooldown prêt.
11. Résoudre projectiles.
12. Résoudre morts → gains d'or + Mana.
13. Détecter fin de niveau (objectif atteint).

### 4.2 Partie par partie

```
Menu (campagne) → clic sur un niveau débloqué → Playing
     → objectif atteint → Won → Écran de fin (étoiles + gold)
     → Enter → retour menu (progression sauvegardée)

Depuis le menu :
  Tab → modal Upgrade Tree (achats persistés)
```

Pas de sauvegarde mid-partie. Chaque niveau repart de zéro (mêmes
stats, mêmes coûts). L'**or** et les **upgrades achetés** sont les
seuls éléments persistants.

---

## 5. Modèle de données (.ron)

### 5.1 `assets/chapters.ron`

Deux chapitres, 3 niveaux chacun. Chaque niveau référence une
palette et un `AiConfig` avec des waves temporelles.

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
            (
                id: "sh_02",
                name: "Le Bund",
                palette: "shanghai",
                width: 1800.0,
                tower_hp: 500.0,
                objective: Destroy,
                reward_gold: 250.0,
                ai: (
                    aggression: 1.0,
                    mana_regen_mult: 1.0,
                    waves: [
                        (start_at: 0.0, priority: ["grunt"]),
                        (start_at: 15.0, priority: ["grunt", "brute"]),
                        (start_at: 40.0, priority: ["grunt", "brute", "archer"]),
                    ],
                ),
            ),
            (
                id: "sh_03",
                name: "Pudong",
                palette: "shanghai",
                width: 2000.0,
                tower_hp: 600.0,
                objective: Destroy,
                reward_gold: 400.0,
                ai: (
                    aggression: 1.15,
                    mana_regen_mult: 1.20,
                    waves: [
                        (start_at: 0.0, priority: ["grunt"]),
                        (start_at: 10.0, priority: ["grunt", "brute"]),
                        (start_at: 30.0, priority: ["grunt", "brute", "archer"]),
                        (start_at: 60.0, priority: ["brute", "archer", "bomber"]),
                        (start_at: 90.0, priority: ["hero", "brute", "archer", "bomber"]),
                    ],
                ),
            ),
        ],
    ),
    (
        id: "guiyang",
        name: "Guiyang",
        subtitle: "Le cœur du Guizhou",
        levels: [
            (
                id: "gy_01",
                name: "Les rizières",
                palette: "guiyang",
                width: 1700.0,
                tower_hp: 500.0,
                objective: Survive(60.0),
                reward_gold: 220.0,
                ai: (
                    aggression: 0.95,
                    mana_regen_mult: 1.0,
                    waves: [
                        (start_at: 0.0, priority: ["grunt", "brute"]),
                        (start_at: 25.0, priority: ["grunt", "brute", "archer"]),
                        (start_at: 45.0, priority: ["brute", "archer", "bomber"]),
                    ],
                ),
            ),
            (
                id: "gy_02",
                name: "La montagne",
                palette: "guiyang",
                width: 1900.0,
                tower_hp: 700.0,
                objective: Destroy,
                reward_gold: 350.0,
                ai: (
                    aggression: 1.10,
                    mana_regen_mult: 1.15,
                    waves: [
                        (start_at: 0.0, priority: ["grunt", "brute"]),
                        (start_at: 20.0, priority: ["brute", "archer"]),
                        (start_at: 50.0, priority: ["brute", "archer", "bomber"]),
                        (start_at: 80.0, priority: ["hero", "brute", "archer"]),
                    ],
                ),
            ),
            (
                id: "gy_03",
                name: "La citadelle",
                palette: "guiyang",
                width: 2100.0,
                tower_hp: 800.0,
                objective: Destroy,
                reward_gold: 600.0,
                ai: (
                    aggression: 1.30,
                    mana_regen_mult: 1.30,
                    waves: [
                        (start_at: 0.0, priority: ["brute", "grunt"]),
                        (start_at: 15.0, priority: ["brute", "archer"]),
                        (start_at: 40.0, priority: ["brute", "archer", "bomber"]),
                        (start_at: 70.0, priority: ["hero", "brute", "archer", "bomber"]),
                        (start_at: 110.0, priority: ["hero", "brute", "archer", "bomber", "healer"]),
                    ],
                ),
            ),
        ],
    ),
]
```

**Notes** :
- `objective` : `Destroy` (détruire) ou `Survive(f32)` (tenir N secondes).
- `ai.waves[*].start_at` : temps de début en secondes.
- `ai.waves[*].priority` : ordre de préférence des unités.
- `ai.aggression` : divise le seuil de Mana (Hard = spawn plus tôt).
- `reward_gold` : or crédité à la victoire.
- `palette` : id dans `palettes.ron`.

### 5.2 `assets/palettes.ron`

Une palette par chapitre. Chaque palette définit les couleurs pour
le ciel, les couches de parallax, le sol, les tours et les néons.

```ron
[
    (
        id: "shanghai",
        style: Urban,
        sky_top: (0.03, 0.04, 0.12, 1.0),
        sky_bottom: (0.35, 0.12, 0.28, 1.0),
        far_layer: (0.16, 0.18, 0.32, 1.0),
        mid_layer: (0.08, 0.10, 0.20, 1.0),
        ground_base: (0.06, 0.07, 0.12, 1.0),
        ground_accent: (0.95, 0.72, 0.35, 1.0),
        tower_player: (0.28, 0.60, 1.00, 1.0),
        tower_enemy: (1.00, 0.30, 0.35, 1.0),
        window_color: (1.00, 0.85, 0.40, 1.0),
        accent_light: (0.95, 0.42, 0.62, 1.0),
        neon_1: (0.95, 0.35, 0.55, 1.0),
        neon_2: (0.35, 0.85, 1.00, 1.0),
        mist_color: (0.0, 0.0, 0.0, 0.0),
    ),
    (
        id: "guiyang",
        style: Mountain,
        sky_top: (0.32, 0.42, 0.58, 1.0),
        sky_bottom: (0.92, 0.76, 0.55, 1.0),
        far_layer: (0.42, 0.50, 0.56, 1.0),
        mid_layer: (0.18, 0.28, 0.24, 1.0),
        ground_base: (0.14, 0.18, 0.14, 1.0),
        ground_accent: (0.65, 0.75, 0.45, 1.0),
        tower_player: (0.35, 0.65, 0.45, 1.0),
        tower_enemy: (0.75, 0.30, 0.25, 1.0),
        window_color: (1.00, 0.85, 0.40, 1.0),
        accent_light: (0.95, 0.70, 0.40, 1.0),
        neon_1: (0.95, 0.70, 0.40, 1.0),
        neon_2: (0.70, 0.85, 0.95, 1.0),
        mist_color: (0.90, 0.88, 0.82, 0.28),
    ),
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

```ron
[
    (
        id: "grunt",
        name: "Grunt",
        cost: 15.0,
        cooldown: 1.5,
        hp: 30.0,
        speed: 90.0,
        damage: 5.0,
        attack_range: 28.0,
        attack_cooldown: 0.8,
        shape: Rect,
        color: (0.90, 0.90, 0.90, 1.0),
        size: (14.0, 22.0),
    ),
    (
        id: "brute",
        name: "Brute",
        cost: 40.0,
        cooldown: 4.0,
        hp: 120.0,
        speed: 55.0,
        damage: 18.0,
        attack_range: 32.0,
        attack_cooldown: 1.4,
        shape: Rect,
        color: (0.85, 0.35, 0.25, 1.0),
        size: (22.0, 30.0),
    ),
    (
        id: "archer",
        name: "Archer",
        cost: 30.0,
        cooldown: 2.5,
        hp: 40.0,
        speed: 75.0,
        damage: 12.0,
        attack_range: 180.0,
        attack_cooldown: 1.6,
        shape: Triangle,
        color: (0.40, 0.85, 0.40, 1.0),
        size: (16.0, 22.0),
        projectile: Some((
            speed: 420.0,
            damage: 12.0,
            radius: 4.0,
            color: (0.40, 0.85, 0.40, 1.0),
        )),
    ),
    (
        id: "healer",
        name: "Healer",
        cost: 45.0,
        cooldown: 5.0,
        hp: 50.0,
        speed: 70.0,
        damage: 0.0,
        attack_range: 0.0,
        attack_cooldown: 0.0,
        shape: Circle,
        color: (0.55, 0.85, 1.00, 1.0),
        size: (18.0, 18.0),
        heal: Some(8.0),
        heal_range: Some(90.0),
        heal_cooldown: Some(1.2),
    ),
    (
        id: "bomber",
        name: "Bomber",
        cost: 35.0,
        cooldown: 4.5,
        hp: 45.0,
        speed: 130.0,
        damage: 45.0,
        attack_range: 22.0,
        attack_cooldown: 1.0,
        shape: Circle,
        color: (0.95, 0.80, 0.20, 1.0),
        size: (16.0, 16.0),
        suicide: true,
        explosion_radius: Some(70.0),
    ),
    (
        id: "hero",
        name: "Hero",
        cost: 90.0,
        cooldown: 18.0,
        hp: 250.0,
        speed: 80.0,
        damage: 35.0,
        attack_range: 40.0,
        attack_cooldown: 0.9,
        shape: Rect,
        color: (1.00, 0.85, 0.20, 1.0),
        size: (26.0, 36.0),
        max_alive: Some(1),
    ),
]
```

**Unités** :
- **Grunt** : épée courte, melee basique, pas cher.
- **Brute** : gourdin, melee lourd, tank.
- **Archer** : arc, ranged avec projectile, débloqué par l'arbre.
- **Healer** : bâton doré, soigne l'allié le plus blessé, débloqué par l'arbre.
- **Bomber** : bombe, suicide AoE (70 px), débloqué par l'arbre.
- **Hero** : épée longue, melee fort, max 1 vivant, débloqué par l'arbre.

### 5.5 `assets/upgrade_tree.ron`

**18 nœuds, 4 branches.** Achetés dans le menu via Tab.

```
                         [Awakening]  (racine, gratuit)
                              |
       +----------+-----------+-----------+----------+
       |          |                       |          |
   DÉFENSE     ARMÉE                   TOURELLE   ÉCONOMIE
   (col -3)   (col -1)                (col +1)   (col +3)
       |          |                       |          |
     HP I      Unit HP I                Dmg I      Mana I
       |          |                       |          |
     HP II     Unit HP II               Dmg II     Gold I
       |          |                       |          |
     Regen   Unlock Archer             Fire rate  Mana cap
       |          |                       |          |
     Fortress Unlock Healer            Multi-shot
                   |
                Unlock Bomber
                   |
                Unlock Hero
```

**Effets disponibles** :
```rust
pub enum UpgradeEffect {
    None,
    TowerHpMult(f32),
    TowerRegen(f32),
    Fortress,           // 2e barre HP (V2 — pas encore appliqué)
    UnitHpMult(f32),
    UnitDamageMult(f32),
    UnlockUnit(String),
    TurretDamageMult(f32),
    TurretFireRateMult(f32),   // <1 = plus rapide
    MultiShot(u32),            // >=1
    ManaRegenMult(f32),
    ManaCapMult(f32),
    GoldPerKillAdd(f32),
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
    pub id: UnitId,             // stable, monotone
    pub kind: String,           // id dans units.ron
    pub team: Team,
    pub pos: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub damage: f32,            // baké au spawn
    pub attack_cd: f32,
    pub attack_cd_max: f32,
    pub heal_cd: f32,
    pub hit_flash: f32,         // juice
    pub pose_phase: f32,        // temps cumulé pour l'anim
    pub target: Option<UnitId>,
    pub pending_attack: Option<PendingAttack>,
}

/// Attaque en cours d'anim, dont le coup doit être appliqué plus tard
/// (à la fin du windup). Permet à la cible de rester vivante pendant
/// l'animation.
pub struct PendingAttack {
    pub target: AttackTarget,
    pub damage: f32,
    pub time_to_hit: f32,       // temps restant avant le strike
}

pub enum AttackTarget {
    Unit(UnitId),
    Tower(Team),
}

pub struct Projectile { ... }
pub struct Tower { ... }
```

### 6.2 `units.rs` — Stats et spawn

- Charge `units.ron` via `OnceLock`.
- `spawn_unit(gen, kind, team, pos, hp_mult, damage_mult) -> Option<Unit>`.
- Initialise `attack_cd_max` à `stats.attack_cooldown`, `pose_phase` à `0.0`,
  `pending_attack` à `None`.
- `can_spawn(kind, team, units)` respecte `max_alive`.
- `alive_count(kind, team, units)`.

### 6.3 `stickman.rs` — Animation (Phase 23, F1)

Dessine chaque unité comme un stick man : tête (cercle), torse,
2 bras, 2 jambes.

**Poses** :
- **Idle** : balancement léger (`IDLE_FREQ = 1.2` Hz).
- **Walking** : jambes alternées (`WALK_FREQ = 2.2` Hz).
- **Attacking** : 3 phases (voir ci-dessous).
- **Dying** : ragdoll simple, les membres s'écartent sur 0.6 s.

**Animation d'attaque — 3 phases** :
- **Windup** (0 → `WINDUP_END = 0.30`) : le bras monte derrière (`REST_ARM_R` → `-1.80` rad).
- **Strike** (`WINDUP_END` → `STRIKE_END = 0.65`) : le bras descend en
  smoothstep (`-1.80` → `+1.60` rad).
- **Recovery** (`STRIKE_END` → 1.0) : retour progressif vers `REST_ARM_R = -0.15`.

**Durée totale** : `ATTACK_DURATION = 0.55` s.

**Continuité** : les constantes `REST_ARM_L = 0.15` et `REST_ARM_R = -0.15`
sont partagées entre Idle, Walking et les bornes d'attaque → aucune
discontinuité visuelle.

**Helpers publics** :
- `WINDUP_END`, `STRIKE_END`, `ATTACK_DURATION` (utilisés par
  `combat::strike_delay`).
- `auto_pose(unit, is_attacking) -> Pose`.
- `draw(unit, pos, size, facing, pose, phase, color)`.

### 6.4 `weapons.rs` — Armes procédurales (Phase 23, F2)

Dessine l'arme tenue par la main droite de la stick man.

```rust
pub enum Weapon {
    Sword,      // grunt
    Club,       // brute
    Bow,        // archer
    Staff,      // healer
    Bomb,       // bomber
    Longsword,  // hero
}
```

Détection via `Weapon::for_kind(&unit.kind)`.

Chaque arme a : `length`, `thickness`, `wrist_offset`,
`swing_amplitude`, `color(camp_color)`.

**`WeaponAnchor { hand, facing, arm_angle, attack_t }`** : ancre au
bout du bras droit, angle `arm_angle + wrist_offset + attack_t *
swing_amplitude`.

**Dessins spécifiques** :
- Épée / Longsword / Staff / Club : ligne + pointe optionnelle +
  orbe (staff).
- Bow : double ligne en V, corde tendue pendant le strike.
- Bomb : cercle + reflet.

### 6.5 `combat.rs` — Targeting, attaque, projectiles, juice

**`strike_delay() -> f32`** : `ATTACK_DURATION * WINDUP_END` = 0.165 s.
Délai entre le début de l'anim et l'application des dégâts.

**`acquire_target`** : unité ennemie la plus proche à portée > tour
ennemie à portée.

**`resolve_attacks`** — boucle à deux phases :

**Phase 1 — appliquer les pending attacks dont `time_to_hit <= 0`** :
- Collecte les `(index, pending, pos, team)` pour tous les pending
  expirés.
- Pour chacun : si l'attaquant est mort, annuler. Sinon,
  `apply_pending_attack` :
  - Ranged : spawn projectile.
  - Bomber : `detonate_bomber(units, ctx.attacker_idx,
    ctx.attacker_pos, ...)`.
  - Melee : `apply_damage_to_unit` sur la cible.
- Reset `pending_attack = None`.

**Phase 2 — décision d'attaque + mouvement par unité** :
- **Healer** : `resolve_healer`. S'il n'a rien à soigner ET n'a pas
  atteint la midline (`enemy_tower.x - HEALER_MIDLINE_MARGIN`), il
  avance.
- **Autres** : `acquire_target`.
  - **Pas de cible** → avance (`advance_toward_tower`), **même si
    `attack_cd > 0`** (fix Phase 23 : plus de figement après attaque).
  - **Cible** → si `pending_attack.is_none()` ET `attack_cd == 0` →
    créer `PendingAttack { target, damage, time_to_hit:
    strike_delay() }`, armer `attack_cd =
    max(stats.attack_cooldown, ATTACK_DURATION)`.

**`tick_unit_timers`** : décrémente `attack_cd`, `heal_cd`,
`hit_flash`, incrémente `pose_phase`. **Ne consomme PAS
`pending_attack`** (c'est la Phase 1 qui le fait, sinon le strike
est perdu — bug classique).

**`resolve_projectiles`** : avance, collision, AoE explosive, pierce.

**`detonate_bomber(units, bomber_idx, pos, team, damage, radius,
juice)`** : AoE sur les ennemis à portée depuis `pos`, tue le
bomber, screen shake fort.

**Cap** : `MAX_PROJECTILES = 200`.

### 6.6 `mana.rs` — Ressource

```rust
pub struct ManaPool { pub current: f32, pub max: f32, pub regen: f32 }
```

- `tick(dt)`, `try_spend(amount)`, `gain(amount)`, `fraction()`.
- Un pool par camp (joueur + IA).

### 6.7 `tower.rs` — Turret

```rust
pub struct Turret {
    pub pos: Vec2,
    pub angle: f32,             // clampé ±80°
    pub cooldown: f32,
    pub shot_kind: ShotKind,    // Basic | Piercing | Explosive
    pub fire_rate_mult: f32,
    pub multi_shot: u32,
}
```

- `aim(mouse_world)`, `try_fire_multi(team, damage_mult)`.
- Multi-shot : éventail de 15° centré.

### 6.8 `ai.rs` — IA adverse

`decide_spawn(ai_mana, ai_units, player_units, ai_config, elapsed,
cooldowns)`.

Détermine la wave active = dernière avec `start_at <= elapsed`.
Ordre défensif si `ai_alive + 2 < player_alive`. Prend le premier
type abordable, hors cooldown, `can_spawn`.

### 6.9 `upgrades.rs` — UpgradeTree

`UpgradeTreeDefs` (18 nœuds, 4 branches) +
`UpgradeTree { gold, purchased }` sérialisable.

**Méthodes** : `is_purchased`, `is_accessible`, `can_buy`, `buy`,
`grant` (tests), et agrégations : `tower_hp_mult`, `tower_regen`,
`has_fortress`, `unit_hp_mult`, `unit_damage_mult`,
`unlocked_units(defaults)`, `turret_damage_mult`,
`turret_fire_rate_mult`, `multi_shot_count`, `mana_regen_mult`,
`mana_cap_mult`, `gold_per_kill_add`.

**Agrégation** : les mults se multiplient, les additifs
s'additionnent, les unlock s'unissent.

### 6.10 `progress.rs` — Persistance

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

### 6.11 `camera.rs` — Caméra locale

```rust
pub struct Camera2D { pub x: f32, pub viewport_w: f32, pub map_w: f32 }
```

- `scroll(dt, dir)` : contrôle manuel (flèches ← / →).
  `CAMERA_SCROLL_SPEED = 900.0` px/s.
- `clamp()` : borne à `[0, max_x]`.
- `follow_front` : **désactivé** en V1, conservé pour V2.
- `world_to_screen` / `screen_to_world`.

### 6.12 `layout.rs` — Layout runtime

- `scr_w()`, `scr_h()`, `cx()`, `cy()` runtime.
- `SPAWN_BAR_H = 90.0`, `UPGRADE_PANEL_W = 220.0`.
- **Sol responsive** : `ground_y_for(viewport_h) = viewport_h -
  SPAWN_BAR_H - BOTTOM_MARGIN - GROUND_PADDING` (constante
  `GROUND_PADDING_ABOVE_SPAWN_BAR = 30.0`).
- Recalculé à chaque frame via `Game::update_viewport`.

### 6.13 `textures.rs` — Décor procédural

Toutes les fonctions de dessin sont dans ce module. Aucun asset
externe. Utilise une `Palette` et un `visual_seed` pour la variation.

**Fonctions publiques** :
- `draw_sky(palette, ground_y)` — dégradé + étoiles (Urban).
- `draw_far_layer(palette, cam_x, ground_y, seed)` — skyline lointaine
  ou montagnes, parallax 0.25.
- `draw_mid_layer(palette, cam_x, ground_y, seed)` — skyline proche
  ou collines, parallax 0.55.
- `draw_ground(palette, cam_x, ground_y, vh)` — sol + pavés / herbe
  + néons au sol (Urban).
- `draw_mist(palette, ground_y)` — bande de brume (Mountain uniquement).
- `draw_tower(player, palette, world_x, cam_x, ground_y, shake)` —
  tour stylisée avec fenêtres, créneaux, porte, antenne.

**Hash déterministe** : `hash_u32(seed, x)` (murmur-inspired).

### 6.14 `juice.rs` — Feedback visuel

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
- `spawn_floating_text(pos, text, color)` : "-10" qui monte.
- `draw_particles`, `draw_floating_texts`, `draw_flashes`.

### 6.15 `fonts.rs` — Police Unicode

- `load_default_font().await -> Option<Font>` : charge DejaVu Sans
  Mono depuis `assets/DejaVuSansMono.ttf` et le définit comme police
  par défaut via `set_default_font`.
- À appeler une fois dans `main`.
- Fallback silencieux vers la police macroquad si absent.
- **Résout le problème des symboles Unicode** (★☆, accents).

### 6.16 `menu.rs` — Campagne

- Disposition **linéaire verticale** : chapitres empilés, chacun
  avec header + séparateur + niveaux en dessous.
- **Scrollable** : molette, flèches ↑↓. Header (titre + gold) et
  footer (hints) restent fixes.
- Chaque niveau affiche : numéro + nom, objectif, étoiles (★☆),
  reward gold.
- Chapitres verrouillés en gris, niveaux non débloqués grisés.

### 6.17 `ui/spawn_bar.rs` — Barre de spawn

- 6 slots en bas de l'écran.
- Chaque slot : nom, coût Mana, état (locked / ready / no mana /
  cooldown / max).
- Cooldown overlay (voile noir proportionnel).

### 6.18 `ui/upgrade_modal.rs` — Modal arbre

- Ouverte par Tab depuis le **menu** (pas en jeu).
- 18 nœuds disposés en grille (col, row).
- Connexions entre nœuds et prérequis.
- Description en bas (nom, effet, état, coût).
- Navigation : flèches (find_neighbor), Entrée (achat), souris.
- **Layout responsive** : `row_spacing` calculé à partir de la
  hauteur disponible.

### 6.19 `systems.rs` — State struct `Game`

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
    pub turret: Turret,
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
- `new(ctx, level_id, seed, progress) -> Result<Self, String>`.
- `update_viewport(viewport_w, viewport_h)` : recalcule `ground_y`,
  repositionne la turret, décale les unités existantes.
- `tick(dt, mouse_world, mouse_left_pressed, camera_scroll)`.
- `try_player_spawn(kind) -> bool`.
- `survive_remaining() -> Option<f32>`.
- `tower_hp_fraction() -> f32`.
- `check_end_condition()`.

### 6.20 `main.rs` — Boucle

- `AppScreen { Menu, Game }`.
- Charge la police DejaVu au démarrage.
- Charge `Progress` depuis `progress.ron`.
- Dispatch menu / game.
- En Game : tick + draw + résolution de fin de niveau (une seule fois).
- Résolution : crédite bank + reward gold, calcule étoiles, sauvegarde.
- Touches globales : Échap, Tab, R, Entrée, 1/2/3.

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
- `cooldowns[kind]`, `turret.cooldown` : f32 qui décrémente.

**Note** : le pattern « cooldown f32 qui décrémente » est utilisé
dans **4 jeux** maintenant (Asteroids, Bullet Hell, Zhuo Ji,
ember-wars). L'extraction d'un type `Cooldown` dans
`ember_stdlib` (ou `ember_core::time`) est **planifiée** pour une
session de refacto dédiée.

---

## 8. Structure des fichiers

```
games/ember-wars/
├── DESIGN.md
├── .env
├── .gitignore (progress.ron ignoré)
├── Cargo.toml
├── assets/
│   ├── chapters.ron         # 2 chapitres × 3 niveaux
│   ├── palettes.ron         # Shanghai + Guiyang
│   ├── balance.ron          # mana, or, coûts
│   ├── units.ron            # 6 unités
│   ├── upgrade_tree.ron     # 18 nœuds, 4 branches
│   └── DejaVuSansMono.ttf   # police Unicode
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config.rs            # GameContext + loaders
│   ├── components.rs        # Unit, PendingAttack, AttackTarget, Tower, Projectile
│   ├── units.rs             # units.ron, spawn, can_spawn
│   ├── stickman.rs          # animation stick men (Phase 23, F1)
│   ├── weapons.rs           # 6 armes procédurales (Phase 23, F2)
│   ├── combat.rs            # targeting, attaques, strike différé
│   ├── mana.rs              # ManaPool
│   ├── tower.rs             # Turret, ShotKind
│   ├── ai.rs                # decide_spawn (waves)
│   ├── upgrades.rs          # UpgradeTree, UpgradeTreeDefs
│   ├── progress.rs          # Progress, étoiles, chapitres
│   ├── camera.rs            # Camera2D (scroll manuel)
│   ├── layout.rs            # scr_w/h, ground_y responsive
│   ├── textures.rs          # décor procédural
│   ├── juice.rs             # particles, shake, floating texts, flashes
│   ├── fonts.rs             # DejaVu Sans Mono
│   ├── render.rs            # dessin jeu + HUD + end overlay
│   ├── menu.rs              # campagne scrollable
│   ├── systems.rs           # Game (state struct), Phase
│   └── ui/
│       ├── mod.rs
│       ├── spawn_bar.rs
│       └── upgrade_modal.rs
└── tests/
    └── game_scenarios.rs    # 14 tests d'intégration
```

---

## 9. Extractions moteur anticipées

| Primitive | Occurrences | Action |
|---|---|---|
| `Cooldown` (f32 qui décrémente) | 4 (Asteroids, BH, Zhuo Ji, EW) | ✅ **Prêt à extraire** — session refacto dédiée |
| `Camera2D` | 1 (EW) | Local — attendre 2e jeu à scroll |
| `AutoCombat` (targeting + attaque) | 1 (EW) | Local |
| `Stickman` / skeletal animation | 1 (EW) | Local |
| `Weapons` procédurales | 1 (EW) | Local |
| `PendingAttack` / strike différé | 1 (EW) | Local |
| `Tourelle` (angle + tir) | 1 (EW) | Local |
| `Spawner` (coût + cooldown + régén) | 1 (EW) | Local |
| `UpgradeTree` | 1 (EW) | Local |
| `Juice` (particles + shake) | 2 (Bullet Hell, EW) | Attendre 3e |
| `Parallax / skybox procédural` | 1 (EW) | Local — attendre 2e |
| Police Unicode via `set_default_font` | 2 (FreeCell, EW) | Attendre 3e |
| Menu campagne scrollable | 1 (EW) | Local |

---

## 10. Stratégie de tests

Objectif atteint : **251 tests unit + 14 integration = 265 tests** dans
`ember-wars`.

### 10.1 Tests unitaires par module

| Module | Tests |
|---|---|
| `components.rs` | 4 |
| `units.rs` | 18 |
| `stickman.rs` | 22 |
| `weapons.rs` | 9 |
| `combat.rs` | 42 |
| `mana.rs` | 4 |
| `tower.rs` | 15 |
| `ai.rs` | 12 |
| `upgrades.rs` | 25 |
| `progress.rs` | 15 |
| `camera.rs` | 17 |
| `textures.rs` | 13 |
| `juice.rs` | 15 |
| `systems.rs` | 26 |
| `menu.rs` | 3 |
| `ui/spawn_bar.rs` | 3 |
| `ui/upgrade_modal.rs` | 6 |
| **Total** | **251** |

### 10.2 Tests d'intégration (`tests/game_scenarios.rs`)

1. `player_wins_by_destroying_enemy_tower`
2. `player_wins_by_surviving`
3. `player_loses_when_tower_destroyed`
4. `phase_stays_playing_while_both_towers_alive`
5. `turret_kill_credits_bank_gold`
6. `progress_tree_grants_unlock_at_start`
7. `bomber_explodes_and_damages_neighbors`
8. `levels_load_and_run`
9. `progress_unlocks_next_level_after_completion`
10. `chapter_2_unlocked_only_after_chapter_1_cleared`
11. `stars_scale_with_tower_hp`
12. `chapters_ron_loads`
13. `upgrade_tree_ron_loads`
14. `units_ron_loads`

### 10.3 Ce qu'on ne teste PAS

- Rendu visuel (pas de snapshot).
- Le ressenti (playtest manuel).

---

## 11. Pièges connus / décisions délicates

### 11.1 Hérités du workspace

- **`Input::from_macroquad_with_keys`** peuple les 3 vecteurs.
- **Clavier AZERTY vs QWERTY** : scancodes QWERTY. Préférer `Tab`,
  `Escape`, `Space`, `Enter`, `1/2/3`.
- **`manual_range_contains`** : `(a..=b).contains(&x)`.
- **`manual_is_multiple_of`** : `r.is_multiple_of(3)`.
- **`needless_borrows_for_generic_args`** : `draw_text(format!(...))`
  sans `&`.
- **`clippy::collapsible_if`** : let-chains stables.
- **`clippy::too_many_arguments`** : bundler en tuple.
- **`ui::hit`** : `contains(Rect, Vec2)` etc.
- **`draw_right` / `draw_left`** : `font_size: u16`.
- **`gen` réservé en édition 2024**.
- **`should_implement_trait`** sur `next()` → `next_id()`.
- **`vec_init_then_push`** → `vec![...]`.
- **E0502** sur `a[i].field = a[i].other * x`.

### 11.2 Spécifiques à ember-wars

- **UnitId stable** : `target` pointe vers un `UnitId`, jamais un
  index.
- **Tourelle ≠ sol** : la tourelle est au-dessus (`GROUND_Y -
  TURRET_HEIGHT`). Pour tester un tir direct, placer l'ennemi à la
  hauteur de la tourelle.
- **Comparer les difficultés sur les HP déployés**, pas le nombre.
- **Viewport dynamique** : `game.update_viewport()` à chaque frame.
- **Sol responsive** : `ground_y_for(viewport_h)`.
- **`palettes.ron` doit commencer par `[`** — sinon RON `Expected
  opening [`. Vérifier fichier vide / BOM.
- **Strike différé** : ne JAMAIS clear `pending_attack` dans
  `tick_unit_timers` — c'est la Phase 1 qui le consomme. Sinon le
  strike est perdu.
- **`attack_cd` minimum = `ATTACK_DURATION`** : sinon l'unité
  réattaque avant la fin de son anim.
- **`strike_delay = ATTACK_DURATION * WINDUP_END`** : doit être
  cohérent avec `stickman.rs`. Si on change l'un, il faut changer
  l'autre.
- **`REST_ARM_R` / `REST_ARM_L` partagés** : Idle, Walking, début/fin
  d'attaque doivent utiliser les mêmes constantes, sinon saut au repos.
- **Phase d'anim = `attack_cd_max - attack_cd`** : c'est le temps
  écoulé depuis le début de l'attaque, pas `pose_phase`.
- **Bomber** : `detonate_bomber` prend l'index **de l'attaquant**,
  pas de la cible.
- **Healer midline** : `HEALER_MIDLINE_MARGIN = 300.0` empêche la
  fuite hors écran.
- **Mouvement** : le cooldown n'empêche pas l'avance si pas de cible
  à portée.

### 11.3 Décisions validées

- ✅ Nom : `ember-wars`.
- ✅ Campagne à 2 chapitres × 3 niveaux (Shanghai, Guiyang).
- ✅ Objectifs : Destroy / Survive(N).
- ✅ Contrôle = spawn + tourelle manuelle.
- ✅ Mana = régén passive + bonus kill.
- ✅ IA = waves temporelles + agressivité data-driven.
- ✅ **Upgrades dans le menu** (Tab).
- ✅ **Or persistant** dans `progress.ron`.
- ✅ **Étoiles** (1-3) par niveau.
- ✅ **Chapitres débloqués séquentiellement**.
- ✅ **Menu linéaire scrollable**.
- ✅ **Décor procédural** avec palettes.
- ✅ **Police DejaVu Sans Mono** (Unicode).
- ✅ **Identité des unités** : `UnitId(u32)` monotone.
- ✅ **Sol responsive** (`ground_y_for(viewport_h)`).
- ✅ **Stick men animés** (3 phases).
- ✅ **6 armes procédurales**.
- ✅ **Strike différé**.

---

## 12. Hors-scope / Open questions

### 12.1 V2

- **Stick figures ragdoll** à la mort (actuellement Dying simple).
- **Cadavres persistants** (au lieu de disparaître après 0.6 s).
- **Sons** (`AudioClip` existe, mais rien).
- **Fortress effect** (2e barre HP) — implémenté dans l'arbre mais
  pas encore appliqué.
- **Plus de chapitres** (3e, 4e...).
- **Arbre d'upgrades arborescent** (au lieu de linéaire).
- **Noto Sans CJK** pour les vrais glyphes (si un jour un jeu en a
  besoin).

### 12.2 Open questions

1. **Chapitre 3** : Hong Kong, Beijing, Xi'an ?
2. **Upgrade tree** : plus de nœuds ? Effets mutuellement exclusifs ?
3. **Animations de mort** : cadavres persistants ?
4. **Balance** : à tuner en playtest.

---

## 13. Prochaines étapes

### Session G — Refonte UI
- Menu plus aéré, transitions, panneau d'info au survol.
- Étoiles animées, couleurs par palier.
- Boutons de navigation.

### Session H — Chapitre 3
- Nouvelle région (Hong Kong / Beijing / Xi'an).
- Palette dédiée, 3 niveaux, difficulté plus dure.
- Objectifs variés.

### Session I — Upgrade tree amélioré
- 24-30 nœuds, choix significatifs.
- Effets nouveaux (Unit Speed, Chain Lightning, Poison).

### Session refacto
- Extraction `Cooldown` (4e occurrence).
