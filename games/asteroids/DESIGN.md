# asteroids — Design Document

> Inspiré du classique **Asteroids** (Atari, 1979) — un shoot'em up
> vectoriel en espace fermé par wrap-around. Le joueur pilote un
> vaisseau à inertie, tire sur des astéroïdes qui se scindent, et
> doit survivre à N vagues.

Dernière mise à jour : DESIGN.md rétroactif (Phase 25+, après le refacto
`Cooldown`). Statut : **V1 terminé**, 2e jeu du workspace, 29 tests verts.

---

## 1. Pitch

**asteroids** est un shoot'em up arcade minimaliste. Le joueur pilote
un vaisseau triangulaire dans un espace toroïdal (wrap-around sur les
4 bords), tire sur des astéroïdes qui se scindent en deux quand
touchés, et doit survivre à 5 vagues. Il dispose de 3 vies et d'une
invincibilité temporaire après chaque respawn.

**Mécaniques clés** :
- **Inertie** : le vaisseau accélère quand on pousse, garde sa vitesse
  sinon. Pas de friction. Freiner = se retourner et pousser.
- **Wrap-around** : tout objet qui sort d'un bord réapparaît du bord
  opposé. Les astéroïdes, les balles et le vaisseau suivent la même
  règle.
- **Split hiérarchique** : astéroïde taille 3 → deux taille 2 ;
  taille 2 → deux taille 1 ; taille 1 → disparaît. Chaque split
  donne +10 points, indépendamment de la taille.
- **Invincibilité brève** : 1,5 s après un respawn ou au démarrage.
  Le vaisseau clignote pendant cette fenêtre.

**Thème visuel** : noir profond, étoiles fixes (60 points), lignes
vectorielles. Aucun asset — tout est dessiné en primitives macroquad.

**Plateforme** : desktop, clavier. Fenêtre 900×700 fixe (pas
redimensionnable).

---

## 2. Inspirations

- **Asteroids** (Atari, 1979) — modèle principal : wrap-around,
  inertie, split hiérarchique, vagues progressives.
- **Spacewar!** (MIT, 1962) — vaisseau à inertie, torpilles.
- **Geometry Wars** (Bizarre Creations, 2003) — esthétique vectorielle
  moderne, feedback visuel.
- **Ember Pong** — conventions du workspace (State struct, GameState,
  config via `.env`).

---

## 3. Scope V1

| Élément | Implémenté | V2 (hors-scope) |
|---|---|---|
| **Vagues** | 5 vagues, 4 → 12 astéroïdes | + waves infinies |
| **Astéroïdes** | 3 tailles, split hiérarchique | + astéroïdes spéciaux |
| **Vaisseau** | Inertie, rotation, thrust, tir | + bouclier, + dash |
| **Armes** | 1 seule (balle simple) | + missiles, + laser |
| **Vies** | 3 vies, invincibilité 1,5 s | + bonus vie |
| **Score** | +10 / astéroïde détruit | + combo, + multiplicateur |
| **Wrap-around** | ✅ sur tous les objets | — |
| **High score** | Persistant (`highscore.ron`) | + top 10 |
| **Sons** | Aucun | + AudioClip (existait pas à l'époque) |
| **Rendu** | Primitives macroquad (triangles, cercles, polygones) | + sprites |
| **Config** | `.env` (f32/u32) | — |

**Tests actuels** : **19 unit + 10 integration = 29 tests**.

---

## 4. Core loop

### 4.1 Frame par frame (en jeu)

1. Lire l'input (`ShipInput` : rotate_left/right, thrust, shoot).
2. **Si `Start`** : attendre `Space` → `start_new_game`.
3. **Si `Playing`** :
   - `update_ship` (rotation, thrust, clamp vitesse, wrap).
   - Tick `shoot_cooldown` ; si `shoot` et cooldown prêt → `try_shoot`.
   - `update_bullets` (avance, wrap, tick lifetime, cull morts).
   - `update_asteroids` (avance, wrap, rotation).
   - `resolve_bullet_asteroid_collisions` → score += 10 × détruits.
   - `step_ship_asteroid_collision` (tick invincibilité, puis
     collision si non-invincible).
   - Si plus d'astéroïdes → `on_wave_cleared`.
4. **Si `LevelCleared`** : tick `level_cleared_timer`, avance à la
   vague suivante quand expiré (ou `Win` si c'était la 5e).
5. **Si `GameOver` / `Win`** : `R` → `reset_to_start`.

### 4.2 Partie par partie

```
Start ── Space ──▶ Playing ── 5 vagues ──▶ Win
                     │
                     └── 0 vies ──▶ GameOver

Win / GameOver ── R ──▶ Start (high score préservé)
```

Pas de sauvegarde mid-partie. Le **high score** est le seul élément
persistant.

---

## 5. Configuration (.env)

**Note** : contrairement aux jeux plus récents (ember-wars, zhuo-ji),
Asteroids est configuré via `.env` — pas de `.ron` pour le gameplay.
Seul le high score utilise `Persistence<i32>` → `highscore.ron`.

### 5.1 `.env` — clés reconnues

```
# Fenêtre
SCREEN_WIDTH=900
SCREEN_HEIGHT=700

# Vaisseau
SHIP_RADIUS=14.0
SHIP_ROTATION_SPEED=4.0          # rad/s
SHIP_ACCELERATION=400.0          # px/s²
SHIP_MAX_SPEED=400.0             # px/s
INVINCIBILITY_MS=1500

# Balles
BULLET_RADIUS=3.0
BULLET_SPEED=600.0               # px/s
BULLET_LIFETIME=1.0              # s
BULLET_COOLDOWN_MS=250

# Astéroïdes
ASTEROID_COUNT_START=4           # vague 1
ASTEROID_SPEED_MIN=50.0
ASTEROID_SPEED_MAX=150.0

# Progression
LIVES_START=3
MAX_WAVES=5
WAVE_TRANSITION_MS=1500
```

**Couleurs** : hardcodées dans `config.rs::load_config`, pas lues
depuis `.env`. C'est une déviation par rapport à la convention
`COLOR_<ROLE>_*` d'ember-wars — acceptable pour un jeu de 2012 qui
précède cette convention.

**Toutes les clés ont un défaut** dans `GameContext::default()` →
lancer le jeu sans `.env` fonctionne.

### 5.2 `assets/highscore.ron`

Fichier contenant un seul entier :

```ron
570
```

`Persistence<i32>::in_manifest_dir(manifest_dir, "highscore.ron")`.

### 5.3 Structure `GameContext`

```rust
pub struct GameContext {
    pub screen_w: f32,
    pub screen_h: f32,
    pub ship_radius: f32,
    pub ship_rotation_speed: f32,
    pub ship_acceleration: f32,
    pub ship_max_speed: f32,
    pub invincibility_secs: f32,
    pub bullet_radius: f32,
    pub bullet_speed: f32,
    pub bullet_lifetime: f32,
    pub bullet_cooldown_ms: u32,
    pub asteroid_count_start: u32,
    pub asteroid_speed_min: f32,
    pub asteroid_speed_max: f32,
    pub lives_start: u32,
    pub max_waves: u32,
    pub wave_transition_secs: f32,
    pub ship_color: Color,
    pub bullet_color: Color,
    pub asteroid_color: Color,
    pub bg_color: Color,
    pub star_color: Color,
    pub ui_text_color: Color,
}
```

Méthode utilitaire : `asteroids_for_wave(wave) = asteroid_count_start + (wave - 1) * 2`.
Vague 1 → 4, vague 5 → 12.

---

## 6. Systèmes

Chaque système est un module de `src/`. Ordre d'exécution = §4.1.

### 6.1 `components.rs` — Entités

```rust
pub struct Ship {
    pub transform: Transform,   // position + rotation + scale(=radius)
    pub sprite: Sprite,         // couleur
    pub collider: Collider,     // Circle
    pub velocity: Vec2,         // inertie
}

pub struct Bullet {
    pub transform: Transform,
    pub sprite: Sprite,
    pub collider: Collider,
    pub velocity: Vec2,
    pub lifetime: Cooldown,     // refacto Phase 25
}

pub type AsteroidSize = u8;     // 3 = grand, 2 = moyen, 1 = petit

pub struct Asteroid {
    pub transform: Transform,
    pub sprite: Sprite,
    pub collider: Collider,
    pub velocity: Vec2,
    pub spin: f32,              // rad/s, pur visuel
    pub size: AsteroidSize,
}
```

**Helpers** :
- `Ship::forward()` → `Vec2::new(rotation.cos(), rotation.sin())`.
- `Ship::nose()` → `position + forward * radius` (là où spawn la balle).
- `Ship::radius()`, `Bullet::radius()`, `Asteroid::radius()` — wrappers
  sur `transform.scale.x`.
- `Bullet::is_alive()` → `lifetime.is_active()`.

### 6.2 `config.rs` — `GameContext`

Voir §5. Méthode utilitaire `asteroids_for_wave`.

### 6.3 `systems.rs` — State struct + logique

**`GameWorld`** (voir §7) + les fonctions libres :

- `wrap_position(p, w, h)` : modulo positif (gère les négatifs).
- `asteroid_color_for_size(size, base)` : assombrit 3 → 2 → 1
  (facteurs 1.0, 0.85, 0.7).
- `asteroid_radius(size)` : 3 → 40, 2 → 20, 1 → 10.
- `update_ship(ship, input, ctx, dt)` : rotation, thrust, clamp
  vitesse, wrap.
- `update_bullets(bullets, ctx, dt)` : avance, wrap, tick lifetime,
  `retain(is_alive)`.
- `try_shoot(ship, bullets, ctx)` : spawn une balle devant le nez,
  vitesse = `forward * bullet_speed + ship.velocity * 0.5` (héritage
  partiel de l'inertie).
- `update_asteroids(asteroids, ctx, dt)` : avance, wrap, rotation.
- `spawn_asteroid_wave(asteroids, count, ctx)` : spawn sur les bords
  (4 côtés équiprobables), à 50 px à l'extérieur, avec angle et
  vitesse aléatoires.
- `resolve_bullet_asteroid_collisions(bullets, asteroids, ctx)` :
  collecte les collisions, split hiérarchique, retourne le nombre
  d'astéroïdes détruits.
- `resolve_ship_asteroid_collision(ship, asteroids)` → `bool`.

**Ordre des splits** : `sort_unstable().dedup()` + itération inverse
avant `remove` pour ne pas décaler les indices.

### 6.4 `persistence.rs` — High score

```rust
pub type HighScore = Persistence<i32>;
pub fn default() -> HighScore { ... }   // highscore.ron
```

`GameWorld::new` charge le high score au démarrage. `record_high_score_if_needed`
persiste quand le score courant dépasse.

### 6.5 `main.rs` — Boucle

- `window_conf()` : lit `ctx.screen_w/h` **avant** `#[macroquad::main]`.
- Match sur `world.state` :
  - `Start` : attend `Space`.
  - `Playing` : joue une frame complète (input → update → collisions).
  - `LevelCleared` : tick timer.
  - `GameOver | Win` : attend `R`.
- Rendu **toujours actif** (draw stars + asteroids + bullets + ship).
- HUD (score, wave, lives, best).
- Overlays spécifiques par état (`Start`, `LevelCleared`, `GameOver`,
  `Win`) via `draw_centered`.

---

## 7. State struct & Phases

### 7.1 `GameWorld`

```rust
pub struct GameWorld {
    pub ship: Ship,
    pub bullets: Vec<Bullet>,
    pub asteroids: Vec<Asteroid>,
    pub score: i32,
    pub lives: u32,
    pub wave: u32,
    pub invincibility: Cooldown,      // refacto Phase 25
    pub shoot_cooldown: Cooldown,     // refacto Phase 25
    pub state: GameState,             // ember_core::app::GameState
    pub level_cleared_timer: f32,     // timer one-shot de phase
    pub high_score: i32,
    pub high_score_handle: HighScore,
}
```

### 7.2 Machine à états

Réutilise `ember_core::app::GameState` (comme Snake, Breakout, Pong) :

```
Start ── Space ──▶ Playing
                     │
        ┌────────────┼────────────┐
        │            │            │
        │   (5 vagues clears)    │
        │            │            │
        ▼            ▼            ▼
   LevelCleared    Win         GameOver
        │
        └── timer ──▶ Playing (vague+1)
```

### 7.3 Timers après refacto Phase 25

| Timer | Type | Sémantique |
|---|---|---|
| `Bullet::lifetime` | `Cooldown` | `is_active()` = balle vivante |
| `GameWorld::shoot_cooldown` | `Cooldown` | `is_ready()` = peut tirer |
| `GameWorld::invincibility` | `Cooldown` | `is_active()` = invincible |
| `GameWorld::level_cleared_timer` | `f32` brut | timer de phase (one-shot, pas de reset) |

**Pourquoi `level_cleared_timer` reste en f32** : c'est un timer de
transition de phase, décrémenté une seule fois puis jeté. Il ne
bénéficie pas de l'abstraction `Cooldown` (pas de `reset`, pas de
`trigger`, pas de `fraction`).

**Pourquoi `invincible_until` a été renommé `invincibility`** : le
nom d'origine suggérait un timestamp absolu, mais c'était un
countdown. Après migration, `Cooldown` clarifie la sémantique.

---

## 8. Structure des fichiers

```
games/asteroids/
├── DESIGN.md                  # ce document
├── .env                        # clés de config (voir §5.1)
├── Cargo.toml
├── assets/
│   └── highscore.ron           # i32 persistant
├── src/
│   ├── lib.rs                  # réexports
│   ├── config.rs               # GameContext + load_config + default
│   ├── components.rs           # Ship, Bullet, Asteroid
│   ├── systems.rs              # GameWorld, ShipInput, updates, collisions
│   ├── persistence.rs          # HighScore = Persistence<i32>
│   └── main.rs                 # window_conf + boucle + rendu
└── tests/
    └── game_scenarios.rs       # 10 tests d'intégration
```

---

## 9. Extractions moteur

| Primitive | Statut |
|---|---|
| `Transform`, `Sprite`, `Collider`, `Shape` | Déjà dans `ember_stdlib` |
| `GameState` | Déjà dans `ember_core::app` |
| `Persistence<T>` | Déjà dans `ember_stdlib` |
| `load_dotenv_once`, `env_f32/u32` | Déjà dans `ember_stdlib::config` |
| `Cooldown` | **Extrait en Phase 25** depuis ce jeu (parmi 4) |
| `wrap_position` | Local (aussi utilisé par Snake ?) — candidat si 2e occurrence |
| `random_angle` | Local (utilise `macroquad::rand`) — pas de généralisation |

**Aucune extraction en attente pour Asteroids.** Le jeu est un
consommateur de la stdlib, pas un producteur.

---

## 10. Stratégie de tests

Objectif : **29 tests verts** (19 unit + 10 integration).

### 10.1 Couverture par module

| Module | Tests |
|---|---|
| `components.rs` | 6 |
| `systems.rs` | 13 |
| `tests/game_scenarios.rs` | 10 |
| **Total** | **29** |

### 10.2 Tests d'intégration (`tests/game_scenarios.rs`)

1. `test_bullet_destroys_big_asteroid_and_splits_in_two`
2. `test_small_asteroid_destroyed_without_split`
3. `test_clearing_all_asteroids_leaves_asteroids_empty`
4. `test_ship_hit_by_asteroid_is_detected`
5. `test_ship_away_from_asteroid_is_safe`
6. `test_ship_hit_loses_one_life`
7. `test_invincibility_blocks_life_loss`
8. `test_third_hit_triggers_game_over`
9. `test_shoot_input_creates_bullet_ahead_of_ship`
10. `test_ship_crosses_screen_edge_and_wraps`

### 10.3 Ce qu'on ne teste PAS

- Rendu visuel (pas de snapshot).
- Comportement de la boucle `main.rs` (pas de harnais macroquad).
- Générateur aléatoire (macroquad::rand non seedable pour les tests).

---

## 11. Pièges connus / décisions délicates

### 11.1 Hérités du workspace

- `Transform::position` = top-left par défaut (ici c'est le **centre**
  pour Ship/Bullet/Asteroid — déviation cohérente avec Bullet Hell).
- Pas de fix d'échelle : `transform.scale.x` = rayon.
- Collision circle-circle via `ember_stdlib::collider::collides`.

### 11.2 Spécifiques à Asteroids

- **Wrap positif** : `((p.x % w) + w) % w` — le modulo Rust peut être
  négatif, il faut la double opération.
- **Split avec `remove`** : parcourir en **ordre décroissant**
  (`sort_unstable().dedup()` puis `.iter().rev()`) pour ne pas invalider
  les indices des astéroïdes restants.
- **Invincibilité avant collision** : `step_ship_asteroid_collision`
  **décrémente** l'invincibilité d'abord, puis teste la collision. Un
  vaisseau dont l'invincibilité expire cette frame **peut** être touché
  la même frame. Ordre délibéré.
- **Héritage de vélocité** : la balle part avec `ship.velocity * 0.5`
  en plus de sa vitesse propre. Un vaisseau qui thrust vers l'avant
  tire plus loin.
- **`window_conf()` lit `.env`** : il faut `load_config()` avant
  `#[macroquad::main]`. La macroquad appelle la closure `window_conf`
  avant d'ouvrir la fenêtre.

### 11.3 Décisions validées

- ✅ Wrap-around toroïdal (4 bords).
- ✅ Split hiérarchique 3 → 2 → 1 → disparaît.
- ✅ +10 points par astéroïde détruit, quelle que soit la taille.
- ✅ 5 vagues, 3 vies, invincibilité 1,5 s.
- ✅ Config `.env` (héritage Pong/Breakout/Snake).
- ✅ High score persistant (`Persistence<i32>`).
- ✅ Rendu vectoriel pur (aucun asset).
- ✅ **`GameWorld`** comme state struct unique (convention Ember).
- ✅ Migration `Cooldown` en Phase 25.

---

## 12. Hors-scope / Open questions

### 12.1 V2 potentielles

- **Sons** (`AudioClip` dans ember_stdlib aujourd'hui).
- **Top 10 high scores** (actuellement un seul i32).
- **Astéroïdes spéciaux** (plus gros, plus fragiles, en mouvement).
- **Armes multiples** (missiles à tête chercheuse, laser perforant).
- **Bonus drops** (invincibilité, vitesse temporaire).
- **Bouclier** (absorb une collision).
- **Décor** : nébuleuses, planètes en parallax.

### 12.2 Open questions

1. **Friction** : actuellement nulle. Ajouter une friction légère
   (1-2 %/s) améliorerait-elle le feeling ?
2. **Vagues infinies** : mode "survie" après la 5e ?
3. **Coop local** : 2e vaisseau sur les touches WASD ?

---

## 13. Changelog

### Phase 1-6 — Fondations moteur
- Création du jeu (2e après Pong).
- Aabb/Circle dans `ember_core::math`.
- Collider enrichi (Phase 3).
- `GameState` (Phase 5).
- `GameWorld` state struct (Phase 6).

### Phase 12 — Fondations moteur
- `load_dotenv_once`, `Rng`, `Input::from_macroquad_with_keys`.

### Phase 14 — Architecture
- `GameWorld` déplacé dans `systems.rs` (uniformisation).

### Phase 15-16 — Persistance et tests
- `Persistence<T>` créé (Phase 15).
- Migration `.txt` → `.ron` (Phase 15).
- `in_manifest_dir` fixé (Phase 16).
- `step_ship_asteroid_collision` extrait de `main.rs` (Phase 16).

### Phase 25 — Refacto `Cooldown`
- `Bullet::lifetime: f32` → `Cooldown`.
- `GameWorld::shoot_cooldown: f32` → `Cooldown`.
- `GameWorld::invincible_until: f32` → renommé `invincibility: Cooldown`.
- `level_cleared_timer: f32` **inchangé** (timer de phase).

### DESIGN.md rétroactif (Phase 25+)
- Création du document (ce fichier).

---

## 14. Prochaines étapes

Aucune. Le jeu est stable et n'est plus en développement actif.
Les évolutions éventuelles (§12.1) viendront d'une V2.

**Priorité workspace** : migrations `Cooldown` sur Bullet Hell et
Zhuo Ji, puis `DESIGN.md` rétroactifs pour Pong, Breakout, Snake,
Bullet Hell, Minesweeper, Simon, Air Hockey.