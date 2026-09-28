# bullet-hell — Design Document

> Inspiré des **danmaku** japonais (Touhou, DoDonPachi) : un shoot'em
> up vertical où le joueur esquive des motifs de projectiles denses,
> avec un **système de graze** qui récompense le frôlement des balles
> par un multiplicateur de score.

Dernière mise à jour : DESIGN.md rétroactif (Phase 25+, après le
refacto `Cooldown`). Statut : **V1 terminé**, 5e jeu du workspace,
45 tests verts.

---

## 1. Pitch

**bullet-hell** est un shoot'em up vertical en arène fermée. Le joueur
pilote un petit vaisseau dans les 2/3 bas de l'écran, tire vers le
haut, et doit survivre à 6 vagues d'ennemis qui émettent des motifs
de projectiles variés (radial, visée, spirale, mur). Il dispose de
3 vies et de 2 secondes d'invincibilité après chaque respawn.

**Mécaniques clés** :
- **Hitbox minuscule** : 2,5 px de rayon. Le sprite fait 6 px. La
  différence est cruciale pour esquiver — c'est le cœur du genre.
- **Graze** : frôler une balle ennemie (entrer dans un rayon de
  12 px sans la toucher) incrémente le compteur et le multiplicateur.
- **Multiplicateur** : +0,1 par graze, plafonné à ×8. Décroît de
  0,5/s si le joueur ne graze plus. Reset à ×1 à chaque mort.
- **Focus mode** (Shift) : ralentit le vaisseau de 55 % pour
  permettre un micro-positionnement précis. Affiche les hitboxes.
- **Hitstop** : 80 ms de freeze à chaque mort, feedback juteux.

**Thème visuel** : fond sombre bleu-nuit, projectiles colorés (jaune
joueur, rose ennemis), particules à chaque collision. Aucun asset
externe — tout est dessiné en primitives macroquad.

**Plateforme** : desktop, clavier. Fenêtre 800×600 fixe.

---

## 2. Inspirations

- **Touhou Project** (ZUN, 1996–) — modèle principal : danmaku
  vertical, hitbox minuscule, graze, focus mode, patterns.
- **DoDonPachi** (Cave, 1997) — multiplicateur de score, ennemis
  data-driven, patterns variés.
- **Ikaruga** (Treasure, 2001) — lisibilité des projectiles.
- **Ember Asteroids** — conventions du workspace (State struct,
  GameState, config `.env`, `Cooldown` post-refacto).

---

## 3. Scope V1

| Élément | Implémenté | V2 (hors-scope) |
|---|---|---|
| **Vagues** | 6 vagues data-driven (`waves.ron`) + fallback | + éditeur de niveaux |
| **Ennemis** | 3 kinds visuels : grunt, tank, boss | + mini-boss, + patterns combinés |
| **Entry motions** | 3 : Static, Drift, Sine | + courbes de Bézier, + formations |
| **Émetteurs** | 4 : Radial, Aimed, Spiral, Wall | + Laser, + Wave, + rings multiples |
| **Armes** | 1 seule (balle verticale) | + missiles, + laser, + bombes |
| **Graze** | ✅ compteur + multiplicateur (×8 max) | + slow-motion, + bullet time |
| **Multiplicateur** | ×1 → ×8, décroît 0,5/s | + paliers, + freeze sur graze |
| **Vies** | 3 vies, invincibilité 2 s | + extends (bonus vie) |
| **Focus** | ✅ (Shift, −55 % vitesse) | — |
| **Juice** | Particules, screen shake, hitstop | + floating numbers, + flash |
| **Sons** | Aucun | + AudioClip (existe dans stdlib) |
| **High score** | Aucun (pas de persistance) | + top 10 local |
| **Rendu** | Primitives macroquad (triangles, cercles) | + sprites, + trail bullets |

**Tests actuels** : **34 unit + 11 integration = 45 tests**.

---

## 4. Core loop

### 4.1 Frame par frame (en jeu)

1. Lire l'input (`ShipInput` : dx/dy, focus, fire).
2. **Si hitstop actif** (`now < hitstop_until`) : tick `now`, decay
   shake, `return` — la simulation est gelée.
3. Tick `now`, decay shake, decay multiplier.
4. **Si `Start` / `GameOver` / `Win`** : pas de simulation.
5. **Si `Playing`** :
   - `update_player` : mouvement normalisé, clamp aux bords, tick
     `cooldown`.
   - `try_fire_player` : si `fire` et cooldown prêt → spawn balle.
   - `update_bullets` : avance, tick `ttl`, cull hors bornes + morts.
   - `update_enemies` : entry motion (Static/Drift/Sine), tick
     `fire_cooldown` + `flash`, émet des balles si prêt.
   - `update_particles` : avance, drag, tick `ttl`.
   - `resolve_player_bullet_vs_enemies` : damage + score + particle
     burst à la mort.
   - `resolve_enemy_bullet_vs_player` : détecte hit (hitbox) et graze
     (couronne), applique mort ou bonus de multiplier.
   - Incrémente `wave_elapsed`.
   - **Si plus d'ennemis et `wave_elapsed ≥ 1.5`** → `LevelCleared`
     (ou `Win` si dernière vague).
6. **Si `LevelCleared`** : joueur bouge, particules/balles tick, pas
   d'ennemis. Après 1,5 s → `spawn_wave(wave+1)`, `Playing`.
7. **Si `GameOver` / `Win`** : `R` → `reset_to_start`.

### 4.2 Partie par partie

```
Start ── Space/Enter ──▶ Playing (wave 1/6)
                          │
                          │ (ennemis tués)
                          ▼
                      LevelCleared (1.5 s)
                          │
                          ├─ wave < 6 ──▶ Playing (wave+1)
                          │
                          └─ wave = 6 ──▶ Win

Playing ── 0 vies ──▶ GameOver

GameOver / Win ── R ──▶ Start
```

Pas de sauvegarde mid-partie. **Aucune persistance** — le score est
jeté à chaque partie. C'est un choix arcade : on ne sauvegarde que
le high score, et encore, pas ici.

---

## 5. Configuration (.env)

**Note** : comme Asteroids, Bullet Hell utilise `.env` (pas de `.ron`
pour la config — les *waves* sont en `.ron` séparément, voir §6.5).

### 5.1 `.env` — clés reconnues

```
# Fenêtre
WINDOW_W=800
WINDOW_H=600
HUD_H=20

# Joueur
PLAYER_SPEED=280.0
PLAYER_FOCUS_MULT=0.45
PLAYER_RADIUS=6.0
PLAYER_HITBOX_RADIUS=2.5
PLAYER_GRAZE_RADIUS=12.0
PLAYER_FIRE_RATE=0.12

# Balles
BULLET_PLAYER_SPEED=520.0
BULLET_PLAYER_RADIUS=3.0
BULLET_ENEMY_SPEED=90.0
BULLET_ENEMY_RADIUS=5.0
BULLET_TTL=12.0

# Couleurs (format COLOR_<ROLE>_R/_G/_B/_A)
COLOR_BG
COLOR_PLAYER
COLOR_PLAYER_FOCUS
COLOR_HITBOX
COLOR_HUD
COLOR_BULLET_PLAYER
COLOR_BULLET_ENEMY
COLOR_ENEMY
```

**Toutes les clés ont un défaut** dans `load_config()` → lancer le
jeu sans `.env` fonctionne.

### 5.2 Structure `GameContext`

```rust
pub struct GameContext {
    pub window_w: f32,
    pub window_h: f32,
    pub hud_h: f32,

    pub player_speed: f32,
    pub player_focus_mult: f32,
    pub player_radius: f32,
    pub player_hitbox_radius: f32,
    pub player_graze_radius: f32,
    pub player_fire_rate: f32,

    pub bullet_player_speed: f32,
    pub bullet_player_radius: f32,
    pub bullet_enemy_speed: f32,
    pub bullet_enemy_radius: f32,
    pub bullet_ttl: f32,

    pub color_bg: Color,
    pub color_player: Color,
    pub color_player_focus: Color,
    pub color_hitbox: Color,
    pub color_hud: Color,
    pub color_bullet_player: Color,
    pub color_bullet_enemy: Color,
    pub color_enemy: Color,
}
```

**Méthodes utilitaires** :
- `playfield_w() = window_w`
- `playfield_h() = window_h - hud_h`
- `to_window(p) = Vec2::new(p.x, p.y + hud_h)` — convertit un point
  playfield-space en window-space.

**Espace de coordonnées** : tout le gameplay vit en **playfield-space**
(origine en haut à gauche du playfield, sous le HUD). Seul le rendu
convertit via `to_window`. C'est la clé pour que les tests tournent
sans fenêtre.

### 5.3 `waves.ron`

Fichier à la racine du crate (pas dans `assets/`), chargé via
`ember_core::io::load_from_file::<Vec<WaveData>>`.

**6 vagues progressives** :

| # | Durée | Ennemis | Pattern dominant |
|---|---|---|---|
| 1 | 18 s | 1 grunt static | Radial(8) — apprendre à esquiver |
| 2 | 22 s | 2 grunts sine | Aimed(3) — apprendre à bouger |
| 3 | 24 s | 2 grunts drift | Spiral(2) — apprendre le rythme |
| 4 | 26 s | 1 tank + 1 grunt | Wall + Radial — combiner |
| 5 | 28 s | 3 grunts static | Aimed + Radial dense |
| 6 | 45 s | 1 boss (200 HP) | Spiral(5) rapide — final |

**Fallback** : si `waves.ron` est absent ou corrompu, `load_waves()`
retourne une vague unique (grunt radial). Le jeu reste jouable.

---

## 6. Systèmes

Chaque système est un module de `src/`. Ordre d'exécution = §4.1.

### 6.1 `components.rs` — Entités

```rust
pub struct Player {
    pub transform: Transform,       // position = CENTRE
    pub hitbox: Collider,            // Circle(2.5)
    pub graze: Collider,             // Circle(12)
    pub cooldown: Cooldown,          // tir
    pub focus: bool,
    pub invincible_until: f32,       // timestamp absolu
    pub alive: bool,
}

pub struct Bullet {
    pub pos: Vec2,                   // pas de Transform (voir §11.2)
    pub vel: Vec2,
    pub radius: f32,
    pub ttl: Cooldown,               // cull quand ready
    pub from_player: bool,
    pub color: Color,
}

pub struct Enemy {
    pub center: Vec2,
    pub base_center: Vec2,           // pour Sine (oscille autour)
    pub radius: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub emitter: Emitter,            // clone à chaque frame (voir §11.2)
    pub fire_cooldown: Cooldown,
    pub age: f32,                    // pour Sine phase
    pub phase: f32,                  // accumulation pour Spiral
    pub entry: EntryMotion,
    pub color: Color,
    pub kind: String,                // "grunt", "tank", "boss"
    pub flash: Cooldown,             // 80 ms au hit
}

pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub ttl: f32,                    // pas de Cooldown (juice pur)
    pub max_ttl: f32,
    pub radius: f32,
    pub color: Color,
}
```

**Helpers** :
- `Player::speed(ctx)` : `player_speed` × `focus_mult` si focus.
- `Player::is_invincible(now)` : `now < invincible_until`.
- `Player::color(ctx, now)` : blink (visible 50 % du temps) si
  invincible, sinon couleur normale ou focus.
- `Bullet::player(pos, vel, ctx)` / `Bullet::enemy(...)` : constructeurs.
- `Bullet::is_alive()` : `ttl.is_active()`.
- `Enemy::from_data(data, ctx)` : construit depuis `EnemyData` +
  grace period de +0,6 s sur le premier tir.
- `Enemy::render_color()` : blanc si `flash` actif, sinon assombri
  par `hp / max_hp`.
- `Particle::burst(pos, count, color, rng)` : explosion radiale avec
  vitesse et TTL aléatoires.

### 6.2 `config.rs` — `GameContext`

Voir §5.2. Chargé une fois au démarrage et une fois dans chaque test
via `load_config()`.

### 6.3 `systems.rs` — State struct + logique

**`World`** (voir §7) + fonctions libres :

- `update(world, input, ctx, dt)` — point d'entrée unique.
- `update_player(world, input, ctx, dt)` — mouvement normalisé, clamp,
  tick cooldown.
- `try_fire_player(world, input, ctx)` — spawn balle si prêt.
- `update_bullets(world, ctx, dt)` — avance + tick ttl + cull.
- `update_enemies(world, ctx, dt)` — entry motion + tick cooldowns +
  émission via `Emitter`.
- `resolve_player_bullet_vs_enemies(world, ctx)` — damage, score,
  particules à la mort.
- `resolve_enemy_bullet_vs_player(world, ctx)` — détecte hit/graze.
- `on_player_hit(world, ctx)` — perte vie, respawn, reset multiplier.
- `update_particles(world, dt)` — avance, drag, tick.
- `spawn_wave(world, ctx, wave_idx)` — spawn les ennemis de la vague.
- `reset(world, ctx)` — retour à l'écran titre.

**Pattern important** : `update_enemies` collecte les demandes de spawn
dans un `Vec<(Vec2, Vec2)>` (`to_spawn`) **avant** de les pousser dans
`world.enemy_bullets`. C'est un **emprunt en deux temps** : on ne peut
pas itérer `world.enemies.iter_mut()` et pousser dans
`world.enemy_bullets` simultanément (borrow checker).

### 6.4 `main.rs` — Boucle

- `window_conf()` lit `WINDOW_W/H` **avant** `#[macroquad::main]`.
- `read_input()` traduit les touches en `ShipInput`.
- Match sur `world.state` :
  - `Start` : attend `Space`/`Enter`.
  - `GameOver | Win` : attend `R` (géré hors match).
- `update` appelé à chaque frame (le match ne conditionne pas l'appel,
  c'est `update` qui gère l'état en interne).
- Rendu : HUD, playfield, overlays.

**Rendu par couche** (ordre d'affichage) :
1. Bullets (joueur + ennemi chaînées).
2. Particules.
3. Ennemis (cercle + noyau sombre).
4. Joueur (triangle, seulement si `alive`).
5. En focus : hitbox + graze ring.

### 6.5 `waves.rs` — Chargement et style

- `load_waves() -> Vec<WaveData>` — charge `waves.ron` avec fallback.
- `enemy_style(kind) -> (radius, color)` — "boss" = 36/rg, "tank" =
  26/ld, "grunt" = 18/rouge.
- `pos_vec((x, y)) -> Vec2` — helper tuple → Vec2.

**Types data** :
- `EnemyData { kind, entry, pos, hp, emitter }`.
- `WaveData { duration, enemies }`.

### 6.6 `lib.rs` — Réexports

```rust
pub mod components;
pub mod config;
pub mod systems;
pub mod waves;

pub use ember_core::app::GameState;
pub use systems::{World, reset, update};
```

---

## 7. State struct & Phases

### 7.1 `World`

```rust
pub struct World {
    pub player: Player,
    pub enemies: Vec<Enemy>,
    pub player_bullets: Vec<Bullet>,
    pub enemy_bullets: Vec<Bullet>,
    pub particles: Vec<Particle>,
    pub score: u32,
    pub lives: i32,
    pub wave: u32,
    pub waves: Vec<WaveData>,
    pub state: GameState,
    pub now: f32,                    // temps global
    pub wave_elapsed: f32,
    pub level_cleared_until: f32,    // timestamp absolu
    pub shake_magnitude: f32,
    pub shake_decay: f32,
    pub hitstop_until: f32,          // timestamp absolu
    pub rng: Rng,                    // seed 0xDEAD_BEEF
    pub graze_count: u32,
    pub multiplier: f32,             // 1.0 → 8.0
}
```

### 7.2 Machine à états

Réutilise `ember_core::app::GameState` (comme Snake, Breakout, Pong,
Asteroids).

```
Start ── Space/Enter ──▶ Playing
                          │
            ┌─────────────┼─────────────┐
            │             │             │
       (tous morts)   (0 vies)     (6e vague)
            │             │             │
            ▼             ▼             ▼
      LevelCleared    GameOver        Win
            │
       (1.5 s timer)
            │
            ▼
        Playing (wave+1)
```

### 7.3 Timers après refacto Phase 25

| Timer | Type | Sémantique |
|---|---|---|
| `Bullet::ttl` | `Cooldown` | `is_active()` = balle vivante |
| `Player::cooldown` | `Cooldown` | `is_ready()` = peut tirer |
| `Enemy::fire_cooldown` | `Cooldown` | `is_ready()` = peut émettre |
| `Enemy::flash` | `Cooldown` | `is_active()` = clignote blanc |
| `Player::invincible_until` | `f32` | timestamp absolu (vs `now`) |
| `World::level_cleared_until` | `f32` | timestamp absolu |
| `World::hitstop_until` | `f32` | timestamp absolu |
| `World::now` | `f32` | temps global (jamais reset en cours de partie) |
| `Particle::ttl` | `f32` | juice pur — `max_ttl` sert pour alpha |

**Pourquoi 3 timestamps restent en `f32`** : ce sont des **timestamps
absolus** comparés à `world.now`, pas des countdowns qui tick dt. Les
convertir en `Cooldown` demanderait de ticker explicitement chaque
frame en miroir de `now`, ce qui est plus fragile qu'un simple
`now < X`.

**Pourquoi `Particle::ttl` reste en `f32`** : c'est du juice, et
`max_ttl` sert à calculer l'alpha au rendu (`ttl / max_ttl`). Un
`Cooldown` offrirait `remaining_fraction()` qui rendrait `max_ttl`
redondant — mais on ne l'a pas fait car les particules ne sont pas
concernées par les resets ou triggers. Candidat à une future
migration cosmétique.

---

## 8. Structure des fichiers

```
games/bullet-hell/
├── DESIGN.md                   # ce document
├── .env                         # clés de config (voir §5.1)
├── Cargo.toml
├── waves.ron                    # 6 vagues data-driven
├── src/
│   ├── lib.rs                   # réexports
│   ├── config.rs                # GameContext + load_config
│   ├── components.rs            # Player, Bullet, Enemy, Particle, Emitter, EntryMotion
│   ├── waves.rs                 # WaveData, EnemyData, load_waves, enemy_style
│   ├── systems.rs               # World, ShipInput, update + sous-systèmes
│   └── main.rs                  # window_conf + boucle + rendu
└── tests/
    └── game_scenarios.rs        # 11 tests d'intégration
```

**Note** : contrairement aux jeux plus récents, pas de module
`persistence.rs` (aucune persistance), pas de `assets/` (pas de
fichiers binaires, pas de police custom).

---

## 9. Extractions moteur

| Primitive | Statut |
|---|---|
| `Transform`, `Collider`, `Shape` | Déjà dans `ember_stdlib` |
| `GameState` | Déjà dans `ember_core::app` |
| `Rng` | Déjà dans `ember_core::rng` |
| `load_dotenv_once`, `env_f32`, `env_color` | Déjà dans `ember_stdlib::config` |
| `Cooldown` | **Extrait en Phase 25** depuis ce jeu (parmi 4) |
| `Emitter` (4 patterns) | Local — spécifique au genre danmaku |
| `EntryMotion` | Local — idem |
| `Particle` | Local — dupliqué avec ember-wars (candidat si 3e) |
| `ScreenShake` | Local — dupliqué avec ember-wars |
| `Hitstop` | Local — propre à ce jeu |
| `Graze` | Local — propre à ce jeu |

**Aucune extraction en attente pour Bullet Hell.** Le jeu consomme la
stdlib, il ne produit pas (à part `Cooldown`, maintenant extrait).

---

## 10. Stratégie de tests

Objectif : **45 tests verts** (34 unit + 11 integration).

### 10.1 Couverture par module

| Module | Tests |
|---|---|
| `components.rs` | 5 |
| `config.rs` | 2 |
| `waves.rs` | 4 |
| `systems.rs` | 23 |
| `tests/game_scenarios.rs` | 11 |
| **Total** | **45** |

### 10.2 Tests d'intégration (`tests/game_scenarios.rs`)

1. `test_world_starts_in_start_state`
2. `test_start_run_spawns_wave_1`
3. `test_fire_input_creates_player_bullet`
4. `test_bullet_hits_enemy_and_damages`
5. `test_killing_last_enemy_transitions_to_level_cleared`
6. `test_level_cleared_advances_to_next_wave`
7. `test_player_hit_costs_life_and_grants_invincibility`
8. `test_zero_lives_triggers_game_over`
9. `test_graze_builds_multiplier_over_multiple_frames`
10. `test_reset_clears_everything`
11. `test_player_cannot_leave_playfield_via_full_loop`

### 10.3 Ce qu'on ne teste PAS

- Rendu visuel (pas de snapshot).
- Boucle `main.rs` (pas de harnais macroquad).
- Interaction entre `focus` et rendu (testé sur la vitesse seulement).

---

## 11. Pièges connus / décisions délicates

### 11.1 Hérités du workspace

- `Transform::position` = top-left par défaut.
- `env_color` lit `{PREFIX}_R/_G/_B/_A`, pas une chaîne.
- Clippy `collapsible_if` → let-chains.
- `clippy::too_many_arguments` → bundle en tuple si > 7 args.
- `should_implement_trait` sur `next()` custom → `next_id()`.

### 11.2 Spécifiques à Bullet Hell

- **`transform.position = CENTRE`** pour Player / Enemy / Particle.
  C'est **la seule déviation documentée** du workspace sur la
  convention top-left. Justifiée par le genre : esquiver un danmaku
  demande de raisonner en centre de hitbox, pas en coin.

- **`Bullet` n'a pas de `Transform`** : position + velocité +
  radius suffisent. Pas de rotation (projectile circulaire), pas
  d'échelle (rayon fixe). Éviter le cérémonial.

- **Hitbox ≠ sprite** : `player_radius = 6.0` (sprite) vs
  `player_hitbox_radius = 2.5` (dégâts). C'est **le** game design
  du genre. Ne jamais les fusionner.

- **Graze ≠ hitbox** : la couronne de graze est un 3e cercle
  (`player_graze_radius = 12.0`) strictement plus grand que la
  hitbox. Un graze n'inflige aucun dégât mais incrémente le
  multiplicateur.

- **Borrow en deux temps dans `update_enemies`** : on collecte les
  spawns dans `to_spawn: Vec<(Vec2, Vec2)>` avant de pousser dans
  `world.enemy_bullets`. Impossible d'itérer `enemies.iter_mut()`
  et pousser dans un autre Vec du même `World` en une passe.

- **`Emitter` est cloné à chaque frame** : `match e.emitter.clone()`
  pour éviter de tenir un borrow mutable sur `e` pendant que
  `e.phase += ...` ou `e.fire_cooldown.trigger(...)`. Clone peu
  coûteux (4 variants, champs primitifs).

- **Hitstop AVANT simulation** : `update` teste `now < hitstop_until`
  en tout premier. Si vrai, tick `now`, decay shake, `return`. La
  simulation est **gelée** mais le temps continue — sinon les
  invincibilités et cooldowns resteraient bloqués pendant le freeze.

- **Multiplier decay 0,5/s** : assez lent pour qu'un graze
  intermittent ne casse pas une chaîne, assez rapide pour punir
  l'inactivité. Ne pas l'accélérer sans playtest.

- **Reset multiplier à la mort** : même si les autres stats
  (score, graze) persistent. C'est le nerf central du jeu : graze
  = risque, mourir = tout perdre.

- **`Rng` seed hardcodé `0xDEAD_BEEF`** : les bursts de particules
  sont donc identiques entre parties. Acceptable pour l'instant
  (pur juice). Passer un vrai seed quand on ajoutera de la
  persistance.

- **Pas de `window_conf` dynamique** : `window_conf()` lit `.env`
  avant `#[macroquad::main]`. Si le `.env` est modifié à chaud, la
  fenêtre ne change pas de taille sans relance.

### 11.3 Décisions validées

- ✅ Danmaku vertical, hitbox 2,5 px.
- ✅ Graze → multiplicateur ×8 max, decay 0,5/s.
- ✅ Focus mode (Shift, −55 % vitesse).
- ✅ 6 vagues data-driven (`waves.ron`) + fallback.
- ✅ 4 émetteurs, 3 entry motions.
- ✅ 3 vies, invincibilité 2 s.
- ✅ Hitstop 80 ms sur mort.
- ✅ Juice : particules, screen shake.
- ✅ Config `.env`.
- ✅ Position = centre (déviation assumée).
- ✅ `GameWorld`/`World` comme state struct unique.
- ✅ Migration `Cooldown` en Phase 25.

---

## 12. Hors-scope / Open questions

### 12.1 V2 potentielles

- **Sons** (`AudioClip` existe maintenant).
- **High score persistant** (`Persistence<u32>`).
- **Trail sur les balles** (particules TTL longues derrière).
- **Nouvelles armes** : missiles à tête chercheuse, laser
  perforant, bombes à zone.
- **Ennemis destructibles** (drop des power-ups au lieu de
  disparaître).
- **Bombes** (clear screen avec pénalité de score).
- **Boss multi-phases**.
- **Bullet time** sur les grazes rapprochés.

### 12.2 Open questions

1. **Équilibre vague 6** : le boss fait 200 HP avec Spiral(5) à
   0,14 s de cooldown. Trop dur ? À playtester.
2. **`Rng` seed fixe** : changer pour un seed tiré de l'horloge
   quand on ajoutera du contenu variable ?
3. **Focus mode par défaut** : inverser (focus ON, Shift OFF) ? La
   convention Touhou est Shift pour focus, on la suit.
4. **Headless testing** : la boucle `main.rs` n'est pas testable
   directement. Extraire un `run_frame(world, input, ctx, dt)` pour
   pouvoir tester le hitstop + l'input en integration ?

---

## 13. Changelog

### Sessions A-F — Construction initiale
- `World` + `ShipInput` + `update`.
- `Player` (hitbox + graze + focus).
- `Bullet` (joueur + ennemi, TTL).
- `Enemy` + 4 `Emitter` + 3 `EntryMotion`.
- `Particle::burst` + `Rng`.
- `waves.ron` + fallback.
- Juice : hitstop, screen shake, flash.
- 45 tests.

### Phase 12 — Fondations moteur
- `Rng` extrait dans `ember_core::rng` (utilisé ici en local avant).

### Phase 14 — Architecture
- `World` déplacé dans `systems.rs` (uniformisation workspace).

### Phase 15-16 — Persistance et tests
- Aucune (Bullet Hell ne persiste rien).

### Phase 25 — Refacto `Cooldown`
- `Bullet::ttl: f32` → `Cooldown`.
- `Player::cooldown: f32` → `Cooldown`.
- `Enemy::fire_cooldown: f32` → `Cooldown`.
- `Enemy::flash: f32` → `Cooldown`.
- `invincible_until`, `level_cleared_until`, `hitstop_until`,
  `Particle::ttl` **inchangés** (timestamps absolus + juice).

### DESIGN.md rétroactif (Phase 25+)
- Création du document (ce fichier).

---

## 14. Prochaines étapes

Le jeu est stable, aucun développement actif prévu. Les évolutions
éventuelles (§12.1) viendront d'une V2.

**Priorité workspace** : migrations `Cooldown` sur Zhuo Ji (dernier
jeu), puis DESIGN.md rétroactifs pour Pong, Breakout, Snake,
Minesweeper, Simon, Air Hockey.