# ember-wars — Design Document

> Inspiré de **Cartoon Wars** (Gamevil, 2009) : un hybride
> Tower Defense / RTS side-scrolling. Deux tours s'affrontent,
> chaque camp spawn des unités qui avancent automatiquement,
> le joueur gère une ressource et vise manuellement avec une
> tourelle. Objectif : détruire la tour ennemie ou survivre.

Dernière mise à jour : Phase 23, après Session C (textures procédurales).
Statut : **V1 jouable + polish graphique. Stick men = prochaine étape.**

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
**Chapitre 1 : Shanghai** — nuit urbaine, skyline de gratte-ciels à
fenêtres jaunes, enseignes néon, ciel violet/rose.
**Chapitre 2 : Guiyang** — aube brumeuse sur les montagnes
karstiques, pagodes, rizières.

Tout le décor est **procédural** (aucun asset externe), dessiné en
code avec des palettes par chapitre.

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
| **Tours** | HP data-driven par niveau | +attaques de tour |
| **Unités** | 6 types : Grunt, Brute, Archer, Healer, Bomber, Hero | +volants, +soigneurs avancés |
| **Tourelle** | 3 tirs : Basic, Piercing, Explosive | +tirs élémentaires |
| **Mana** | Régén passive + bonus kill | +upgrades actifs |
| **Or** | Gagné sur kill, **persistant entre niveaux** | — |
| **Upgrade tree** | **18 nœuds, 4 branches**, achetés dans le menu | +arbre arborescent |
| **Déblocage d'unités** | Via l'arbre (Archer, Healer, Bomber, Hero) | — |
| **Progression** | Étoiles (1-3) par niveau, chapitres verrouillés | +achievements |
| **IA** | Waves temporelles, agressivité data-driven | +IA adaptative |
| **Art** | Formes procédurales + palettes par chapitre | +stick men animés (Session F) |
| **Police** | DejaVu Sans Mono (Unicode complet) | — |
| **Écrans** | Menu campagne scrollable + modal upgrade | — |

**Tests actuels** : ~250 dans `ember-wars`.

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
10. Tick unités : target acquisition, advance, attaque.
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
            // sh_02 "Le Bund" — Destroy, 1800px, 500 HP
            // sh_03 "Pudong" — Destroy, 2000px, 600 HP, 5 waves
        ],
    ),
    (
        id: "guiyang",
        name: "Guiyang",
        subtitle: "Le cœur du Guizhou",
        levels: [
            // gy_01 "Les rizières" — Survive(60)
            // gy_02 "La montagne" — Destroy, 1900px
            // gy_03 "La citadelle" — Destroy, 2100px, 5 waves
        ],
    ),
]
```

**Notes** :
- `objective` : `Destroy` (détruire) ou `Survive(f32)` (tenir N
  secondes).
- `ai.waves[*].start_at` : temps de début en secondes.
- `ai.waves[*].priority` : ordre de préférence des unités. L'IA
  prend le premier type disponible ET abordable.
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
        // ... (voir fichier réel)
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

6 unités. Chacune a ses stats, sa forme (utilisée par le render
actuel, sera remplacée par un stick man en Session F), et
éventuellement des comportements spéciaux (`projectile`, `heal`,
`suicide`, `max_alive`).

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
    // brute, archer, healer, bomber, hero
]
```

**Unités** :
- **Grunt** : melee basique, pas cher.
- **Brute** : melee lourd, tank.
- **Archer** : ranged avec projectile, débloqué par l'arbre.
- **Healer** : soigne l'allié le plus blessé, débloqué par l'arbre.
- **Bomber** : suicide AoE, débloqué par l'arbre.
- **Hero** : melee fort, max 1 vivant, débloqué par l'arbre.

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
    Fortress,           // 2e barre HP (V2 — pas encore utilisé)
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
    pub heal_cd: f32,
    pub hit_flash: f32,         // juice
    pub target: Option<UnitId>,
}

pub struct Projectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub damage: f32,
    pub radius: f32,
    pub team: Team,
    pub kind: ProjectileKind,
    pub pierce_remaining: u8,
    pub color: Color,
}

pub struct Tower {
    pub team: Team,
    pub hp: f32,
    pub max_hp: f32,
    pub x: f32,
}
```

### 6.2 `units.rs` — Stats et spawn

- Charge `units.ron` via `OnceLock`.
- `spawn_unit(gen, kind, team, pos, hp_mult, damage_mult) -> Option<Unit>`.
- `can_spawn(kind, team, units)` : respecte `max_alive`.
- `alive_count(kind, team, units)`.

### 6.3 `combat.rs` — Targeting, attaque, juice

- `acquire_target` : unité ennemie la plus proche à portée >
  tour ennemie à portée.
- `resolve_attacks` : tick cooldowns, détermine target, attaque
  (melee, ranged, healer, bomber).
- `resolve_projectiles` : avance, collision, AoE explosive,
  pierce.
- `apply_damage_to_unit` : dégâts + hit flash + spark + death burst.
- `apply_damage_to_tower` : dégâts + screen shake proportionnel.
- `detonate_bomber` : AoE + suicide.
- Cap : `MAX_PROJECTILES = 200`.

### 6.4 `mana.rs` — Ressource

```rust
pub struct ManaPool { pub current: f32, pub max: f32, pub regen: f32 }
```
- `tick(dt)`, `try_spend(amount)`, `gain(amount)`, `fraction()`.
- Un pool par camp (joueur + IA).

### 6.5 `tower.rs` — Turret

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
- `aim(mouse_world)`, `try_fire_multi(team, damage_mult) -> Option<Vec<Projectile>>`.
- Le `fire_rate_mult` et le `multi_shot` sont lus depuis l'arbre
  d'upgrades au moment du tick.
- Multi-shot : éventail de 15° centré.

### 6.6 `ai.rs` — IA adverse

```rust
pub fn decide_spawn(
    ai_mana: &ManaPool,
    ai_units: &[Unit],
    player_units: &[Unit],
    ai_config: &AiConfig,
    elapsed: f32,
    cooldowns: &HashMap<String, f32>,
) -> Option<String>;
```
- Détermine la **wave active** = dernière wave avec `start_at <= elapsed`.
- Si l'IA est en infériorité (`ai_alive + 2 < player_alive`) :
  ordre défensif filtré sur `DEFENSIVE_ORDER` = `[archer, grunt, brute]`.
- Sinon, prend l'ordre de la wave.
- Prend le premier type abordable, hors cooldown, `can_spawn`.

### 6.7 `upgrades.rs` — UpgradeTree

- `UpgradeTreeDefs` : structure désérialisée depuis `upgrade_tree.ron`,
  singleton statique chargé une fois.
- `UpgradeTree` : `{ gold: f32, purchased: HashSet<String> }`,
  **sérialisable** pour la persistance dans `Progress`.
- Méthodes : `is_purchased`, `is_accessible`, `can_buy`, `buy`,
  `grant` (tests).
- **Agrégation** : `tower_hp_mult()`, `tower_regen()`,
  `has_fortress()`, `unit_hp_mult()`, `unit_damage_mult()`,
  `unlocked_units(defaults)`, `turret_damage_mult()`,
  `turret_fire_rate_mult()`, `multi_shot_count()`,
  `mana_regen_mult()`, `mana_cap_mult()`, `gold_per_kill_add()`.
- Les effets multiplicatifs se **multiplient** entre eux.
- Les effets additifs s'**additionnent**.
- `unlocked_units(defaults)` : union des defaults + tous les
  `UnlockUnit` achetés.

### 6.8 `camera.rs` — Caméra locale

```rust
pub struct Camera2D { pub x: f32, pub viewport_w: f32, pub map_w: f32 }
```
- `scroll(dt, dir)` : **contrôle manuel** par le joueur (flèches
  ← / →). `CAMERA_SCROLL_SPEED = 900.0` px/s.
- `clamp()` : borne à `[0, max_x]`.
- `follow_front` : **désactivé** en V1, conservé pour V2.
- `world_to_screen` / `screen_to_world`.

### 6.9 `layout.rs` — Layout runtime

- `scr_w()`, `scr_h()`, `cx()`, `cy()` runtime.
- `SPAWN_BAR_H = 90.0`, `UPGRADE_PANEL_W = 220.0`.
- **Sol responsive** : `ground_y_for(viewport_h) = viewport_h - spawn_bar_h - margin - padding`
  (constante `GROUND_PADDING_ABOVE_SPAWN_BAR = 30.0`). Le sol est
  recalculé à chaque frame via `Game::update_viewport`.

### 6.10 `textures.rs` — Décor procédural

Toutes les fonctions de dessin sont dans ce module. Aucun asset
externe. Utilise une `Palette` et un `visual_seed` pour la variation.

**Fonctions publiques** :
- `draw_sky(palette, ground_y)` — dégradé + étoiles (Urban).
- `draw_far_layer(palette, cam_x, ground_y, seed)` — skyline lointaine
  ou montagnes, parallax 0.25.
- `draw_mid_layer(palette, cam_x, ground_y, seed)` — skyline proche
  ou collines, parallax 0.55.
- `draw_ground(palette, cam_x, ground_y, vh)` — sol + pavés / herbe +
  néons au sol (Urban).
- `draw_mist(palette, ground_y)` — bande de brume (Mountain uniquement).
- `draw_tower(player, palette, world_x, cam_x, ground_y, shake)` —
  tour stylisée avec fenêtres, créneaux, porte, antenne.

**Hash déterministe** : `hash_u32(seed, x)` (murmur-inspired).
Générateurs de hauteur : `far_urban_height`, `mid_urban_height`,
`far_mountain_height`, `mid_mountain_height` — chacun retourne une
valeur dans une plage fixe.

### 6.11 `juice.rs` — Feedback visuel

```rust
pub struct Juice {
    pub particles: Vec<Particle>,
    pub floating_texts: Vec<FloatingText>,
    pub screen_shake: ScreenShake,
    pub flashes: Vec<Flash>,
}
```
- `tick(dt)` : met à jour particules (gravité + drag), floating texts
  (montée + fade), shake (décroissance), flashes (fade).
- `shake(intensity, duration)` : remplace si plus fort.
- `flash(color, duration)` : plein écran semi-transparent.
- `spawn_hit_spark(pos, color)` : petit particle au hit.
- `spawn_death_burst(pos, color, count)` : explosion de particles.
- `spawn_floating_text(pos, text, color)` : "-10" qui monte.
- `draw_particles`, `draw_floating_texts`, `draw_flashes`.

### 6.12 `fonts.rs` — Police Unicode

- `load_default_font().await -> Option<Font>` : charge DejaVu Sans
  Mono depuis `assets/DejaVuSansMono.ttf` et le définit comme police
  par défaut via `set_default_font`.
- À appeler une fois dans `main`.
- Fallback silencieux vers la police macroquad si absent.
- **Résout le problème des symboles Unicode** (★☆, accents) que la
  police par défaut ne couvre pas.

### 6.13 `progress.rs` — Persistance

```rust
pub struct Progress {
    pub stars: HashMap<String, u8>,   // niveau → étoiles (0-3)
    pub tree: UpgradeTree,             // gold + nœuds achetés
}
```
- `record_win(level_id, stars, gold_earned)` : met à jour les étoiles
  (garde le meilleur), crédite la bank d'or.
- `is_level_unlocked(level_id)` : premier niveau, ou précédent
  complété dans l'ordre linéaire.
- `is_chapter_unlocked(chapter_idx)` : chapitre 0 toujours débloqué,
  sinon tous les niveaux du précédent doivent être complétés (≥1⭐).
- `stars_for_victory(tower_hp_fraction)` : 1⭐ / 2⭐ (≥50%) / 3⭐ (≥90%).
- Sauvegarde dans `progress.ron` (gitignoré).

### 6.14 `menu.rs` — Menu campagne

- Disposition **linéaire verticale** : chapitres empilés, chacun avec
  header + séparateur + niveaux en dessous.
- **Scrollable** : molette, flèches ↑↓. Header (titre + gold) et
  footer (hints) restent fixes, masqués par des bandeaux de fond.
- Chaque niveau affiche : numéro + nom, objectif (Destroy/Survive),
  étoiles (★☆), reward gold.
- Chapitres verrouillés en gris, niveaux non débloqués grisés.

### 6.15 `ui/spawn_bar.rs` — Barre de spawn

- 6 slots en bas de l'écran.
- Chaque slot : nom, coût Mana, état (locked / ready / no mana /
  cooldown / max).
- Les unités non débloquées sont grisées avec "locked".
- Cooldown overlay (voile noir proportionnel).

### 6.16 `ui/upgrade_modal.rs` — Modal arbre

- Ouverte par Tab depuis le **menu** (pas en jeu).
- 18 nœuds disposés en grille (col, row).
- Connexions entre nœuds et prérequis, couleur verte si les deux
  sont achetés.
- Nœuds : couleur selon état (acheté / achetable / accessible /
  inconnu).
- Description en bas (nom, effet, état, coût).
- Navigation : flèches (find_neighbor), Entrée (achat), souris
  (hover + clic).
- **Layout responsive** : `row_spacing` calculé à partir de la
  hauteur disponible.
- Panneau de description avec fond opaque.

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
    pub stats: GameStats,               // kills, gold_earned, elapsed
    pub ground_y: f32,                  // responsive, recalculé
    pub juice: Juice,
}

pub enum Phase { Playing, Won, Lost }
```

**Méthodes clés** :
- `new(ctx, level_id, seed, progress) -> Result<Self, String>` :
  snapshot du tree, initialisation des tours / mana / turret.
- `update_viewport(viewport_w, viewport_h)` : recalcule `ground_y`,
  repositionne la turret, décale les unités existantes.
- `tick(dt, mouse_world, mouse_left_pressed, camera_scroll)` :
  boucle complète (voir §4.1).
- `try_player_spawn(kind) -> bool` : vérifie unlock + mana + cooldown
  + `max_alive`, spawn l'unité.
- `survive_remaining() -> Option<f32>`.
- `tower_hp_fraction() -> f32`.
- `check_end_condition()`.

### 6.18 `main.rs` — Boucle

- `AppScreen { Menu, Game }`.
- Charge la police DejaVu au démarrage (`fonts::load_default_font().await`).
- Charge `Progress` depuis `progress.ron`.
- Dispatch menu / game.
- En Game : tick + draw + résolution de fin de niveau (une seule fois).
- Résolution de fin : crédite bank + reward gold, calcule étoiles,
  sauvegarde.
- Touches globales : Échap (retour menu / quit), Tab (modal en menu),
  R (retry niveau), Entrée (retour menu après fin), 1/2/3 (type de tir).

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

- `attack_cd`, `heal_cd` : f32 qui décrémente (convention Asteroids /
  Bullet Hell / Zhuo Ji / ember-wars).
- `cooldowns[kind]` : f32 qui décrémente, un par camp.
- `turret.cooldown` : f32 qui décrémente.

**Note** : le pattern « cooldown f32 qui décrémente » est utilisé
dans **4 jeux** maintenant. L'extraction d'un type `Cooldown` dans
`ember_stdlib` (ou `ember_core::time`) est **planifiée** pour une
session de refacto dédiée.

---

## 8. Structure des fichiers

```
games/ember-wars/
├── DESIGN.md
├── .env
├── Cargo.toml
├── assets/
│   ├── chapters.ron         # 2 chapitres × 3 niveaux
│   ├── palettes.ron         # Shanghai + Guiyang
│   ├── balance.ron          # mana, or, coûts
│   ├── units.ron            # 6 unités
│   ├── upgrade_tree.ron     # 18 nœuds
│   └── DejaVuSansMono.ttf   # police Unicode
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config.rs            # GameContext + loaders .ron (chapters, balance)
│   ├── components.rs        # Unit, Projectile, Tower, Team, Shape, UnitId
│   ├── units.rs             # units.ron, spawn, can_spawn
│   ├── combat.rs            # targeting, attacks, projectiles, juice hooks
│   ├── mana.rs              # ManaPool
│   ├── tower.rs             # Turret, ShotKind
│   ├── ai.rs                # decide_spawn (waves)
│   ├── upgrades.rs          # UpgradeTree, UpgradeTreeDefs
│   ├── progress.rs          # Progress, stars, chapter unlock
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
    └── game_scenarios.rs    # ~15 tests d'intégration
```

---

## 9. Extractions moteur anticipées

| Primitive | Occurrences | Après ce jeu | Action |
|---|---|---|---|
| `Cooldown` (f32 décrémentant) | 4 (Asteroids, BH, Zhuo Ji, EW) | 4 | ✅ **Prêt à extraire** — session refacto dédiée |
| `Camera2D` | 1 (EW) | 1 | Local — attendre 2e jeu à scroll |
| `AutoCombat` (targeting + attaque) | 1 (EW) | 1 | Local |
| `Tourelle` (angle + tir) | 1 (EW) | 1 | Local |
| `Spawner` (coût + cooldown + régén) | 1 (EW) | 1 | Local |
| `UpgradeTree` | 1 (EW) | 1 | Local |
| `Parallax / skybox procédural` | 1 (EW) | 1 | Local — attendre 2e jeu |
| `Juice` (particles + shake) | 2 (Bullet Hell, EW) | 2 | Attendre 3e |
| Police Unicode via `set_default_font` | 2 (FreeCell, EW) | 2 | Attendre 3e |
| Menu campagne scrollable | 1 (EW) | 1 | Local |

---

## 10. Stratégie de tests

Objectif atteint : **~250 tests** dans `ember-wars`.

### 10.1 Tests unitaires par module

| Module | Tests | Cible |
|---|---|---|
| `components.rs` | 5 | Constructeurs, Team::opponent, Tower fraction |
| `units.rs` | 18 | Chargement .ron, spawn, mults, can_spawn |
| `combat.rs` | 25+ | Targeting, attaques, projectiles, bomber |
| `mana.rs` | 4 | Tick, spend, gain, clamp |
| `tower.rs` | 15 | Visée, tir, fire rate, multi-shot |
| `ai.rs` | 15 | Décisions par wave, priorité, defensive |
| `upgrades.rs` | 25 | Validation, achat, agrégation, unlocked |
| `progress.rs` | 15 | Étoiles, déblocage, chapitres |
| `camera.rs` | 15 | Scroll, clamp, world↔screen |
| `textures.rs` | 15 | Palettes, hash, height ranges |
| `juice.rs` | 15 | Particles, shake, flashes |
| `systems.rs` | 25 | Game::new, tick, viewport, survive, buy |
| `menu.rs` | 5 | Layout niveaux, chapitres |
| `ui/spawn_bar.rs` | 3 | Rect des slots |
| `ui/upgrade_modal.rs` | 8 | Layout, focus, description |
| **Total** | **~205** | |

### 10.2 Tests d'intégration (`tests/game_scenarios.rs`)

1. Player wins by destroying enemy tower.
2. Player wins by surviving.
3. Player loses when tower destroyed.
4. Phase stays Playing while both alive.
5. Turret kill credits bank gold.
6. Buying unlock archer enables spawn.
7. Bomber explodes and damages neighbors.
8. Levels load and run (6 niveaux).
9. Chapter 2 levels wider than chapter 1.
10. Progress unlocks next level.
11. Chapter 2 unlocked only after chapter 1 cleared.
12. Stars scale with tower HP.
13. chapters.ron / upgrade_tree.ron / units.ron load.
14. Progress tree grants unlock at start.

### 10.3 Ce qu'on ne teste PAS

- Rendu visuel (pas de snapshot).
- Le ressenti (playtest manuel).

---

## 11. Pièges connus / décisions délicates

### 11.1 Pièges hérités du workspace

- **`Input::from_macroquad_with_keys`** peuple les 3 vecteurs.
- **Clavier AZERTY vs QWERTY** : scancodes QWERTY. Préférer
  `Tab`, `Escape`, `Space`, `Enter`, `1/2/3`.
- **`manual_range_contains`** : utiliser `(a..=b).contains(&x)`.
- **`manual_is_multiple_of`** : `r.is_multiple_of(3)`.
- **`needless_borrows_for_generic_args`** : `draw_text(format!(...))`
  sans `&`.
- **`clippy::collapsible_if`** : let-chains stables.
- **`clippy::too_many_arguments`** : bundler en tuple.
- **`ui::hit`** : `contains(Rect, Vec2)` etc.
- **`draw_right` / `draw_left`** : `font_size: u16`.
- **`gen` est un mot réservé en édition 2024** : ne pas l'utiliser
  comme identifiant. Préférer `id_gen`, `generator`.
- **`clippy::should_implement_trait`** sur une méthode `next()`
  custom → renommer `next_id()`.
- **`clippy::vec_init_then_push`** → préférer `vec![...]`.
- **E0502 sur `a[i].field = a[i].other * x`** → extraire `other`
  dans un `let`.

### 11.2 Pièges spécifiques à ember-wars

- **UnitId stable** : les unités portent un `UnitId(u32)` monotone.
  Le `target` pointe vers un `UnitId`. Lookup par ID.
- **Tourelle ≠ sol** : la tourelle est au-dessus du sol
  (`GROUND_Y - TURRET_HEIGHT`). Pour tester un tir direct, placer
  l'ennemi à la hauteur de la tourelle.
- **Comparer les difficultés sur les HP déployés**, pas le nombre
  d'unités.
- **Viewport dynamique** : `game.update_viewport()` à chaque frame.
- **Sol responsive** : `ground_y_for(viewport_h)` recalcule à partir
  de la hauteur de spawn bar + padding.
- **Bomber** : dégâts AoE à l'impact, puis suicide.
- **Mana cap** : `mana_regen` clampe à `mana_max`.
- **IA déterministe en test** : `Rng::from_state(seed)`.
- **Explosion** : appliquer les dégâts **une seule fois** par unité.
- **Ordre de résolution** : cooldowns → spawn → attaques →
  projectiles → morts.
- **`palettes.ron` doit commencer par `[`** — sinon RON
  `Expected opening [`. Vérifier que le fichier n'est pas vide /
  avec BOM.

### 11.3 Décisions délicates

1. **Or et Mana séparés** — l'or achète les upgrades dans le menu,
   le Mana spawn en jeu.
2. **Upgrades dans le menu, pas en jeu** — la modal n'est
   accessible que depuis le menu.
3. **Or persistant** — crédité à la fin du niveau, conservé dans
   `progress.ron`.
4. **Étoiles** : 1⭐ win, 2⭐ tower HP ≥ 50%, 3⭐ tower HP ≥ 90%.
5. **Chapitres verrouillés** tant que le précédent n'a pas tous ses
   niveaux complétés (≥1⭐).

---

## 12. Hors-scope / Open questions

### 12.1 Hors-scope V1

- **Stick men animés** — remplacement des formes géométriques.
- **CJK** — glyphs 万/条/筒 pour une future référence Zhuo Ji-like.
- **Sons** — `AudioClip` existe, mais rien.
- **Fortress** — implémenté dans l'arbre mais effet pas encore appliqué.
- **Progression méta plus large** — achievements, statistiques.
- **Plus de chapitres** — la structure .ron le permet.

### 12.2 Open questions

1. **Fortress** : l'effet (2e barre HP) doit être implémenté.
2. **Bu Gang** : n'existe pas (c'est Zhuo Ji).
3. **Camera scroll speed** : 900 px/s. À tuner.
4. **Wave balance** : à tuner en playtest.

### 12.3 Décisions validées

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

---

## 13. Prochaines étapes

### Session F — Stick men (haute priorité visuelle)

- F1 — Module `stickman.rs` avec 6 points (tête, torse, 2 bras,
  2 jambes).
- F2 — Animation procédurale : idle, walking, attacking, dying.
- F3 — Arme par type d'unité (épée, gourdin, arc, bâton, bombe).
- Détection d'état automatique dans `render.rs`.

### Session G — Polish gameplay

- Balance (coûts, dégâts, vitesses).
- Ajustement de la courbe de difficulté par chapitre.
- Playtest des 6 niveaux.

### Session refacto moteur (reportée)

- Extraire `Cooldown` dans `ember_core::time` (4e occurrence).
- Extraire `Juice` générique si un autre jeu en a besoin.

### Docs finaux

- Design docs rétroactifs pour Pong, Breakout, Snake, Asteroids,
  Bullet Hell, Minesweeper, Simon, Memory.
- Mise à jour du Récapitulatif Complet.
