# jade-garden — Design Document

> Inspiré de **Bejeweled** (PopCap, 2001) et **Puzzle & Dragons** (GungHo,
> 2012) pour la mécanique de match-3, de **Monument Valley** (ustwo, 2014)
> pour le rythme contemplatif, et de l'esthétique **laque sombre + or
> impérial + cinabre** pour la palette visuelle. Un puzzle contemplatif
> où chaque niveau restaure un fragment d'un jardin de jade abandonné,
> et révèle un caractère d'un poème des Tang.

Dernière mise à jour : Session 7 (release prep).
Statut : **release candidate**. Voir [`RELEASE.md`](RELEASE.md) pour
la checklist et l'état des tests.

---

## 1. Pitch

**jade-garden** est un puzzle match-3 à ambiance poétique. Le joueur
incarne **Xiao Lin (小林)**, un jeune jardinier qui découvre les ruines
d'un jardin impérial abandonné près de Chang'an. Une stèle de pierre
porte un poème à demi effacé, et les jades qui ornaient le jardin sont
dispersés. En alignant les jades, le joueur restaure le jardin pierre
par pierre, caractère par caractère.

**Mécaniques clés** :
- **Match-3 classique** : aligner 3+ jades identiques, les faire
  disparaître, laisser tomber la grille, combler par le haut.
- **Cascade** : un match peut en déclencher un autre, avec un
  multiplicateur croissant. Le joueur regarde, savoure, et parfois
  provoque.
- **Objectifs variables** : atteindre un score, éliminer N jades d'un
  type précis, ou **déclencher N cascades** (les réactions en chaîne
  qui suivent un swap ; le match initial est gratuit).
- **Contemplation** : pas de timer stressant. Chaque niveau a un
  nombre limité de **coups** (moves), pas de secondes. Le joueur peut
  réfléchir, observer, planifier. Un timer *indicatif* s'affiche en
  haut à droite, mais il ne joue aucun rôle dans les conditions de
  fin.

**Thème visuel** : boîte laquée chinoise vue sous une lampe chaude.
Papier presque noir-brun, accents en feuille d'or vieilli, reflets
cinabre. Les jades sont dessinés en procédural avec **six silhouettes
distinctes** (rond, hexagone, diamant, octogone, pentagone, carré)
pour rester lisibles sans couleur. Le fond est un **rouleau peint**
(shan shui) en parallax lent : montagnes, brume, pins, dérive
automatique vers la gauche.

**Narration** : 12 niveaux répartis en 4 chapitres. Chaque niveau
révèle un caractère du poème **《春晓》** de **Meng Haoran** (唐, 689–740) :
« *Aurore de printemps* ». Le poème complet compte 20 caractères,
révélés dans l'ordre. Après le dernier niveau : un **épilogue
cinématique** (le poème s'illumine caractère par caractère, avec
pinyin et prose finale) puis, si la date système tombe entre
septembre et novembre, un **teaser Halloween** (fausse alerte, alien
procédural, jumpscare).

**Plateforme** : desktop, clavier + souris. Fenêtre 1200×900 par
défaut, redimensionnable.

---

## 2. Inspirations

- **Bejeweled** (PopCap, 2001) — modèle principal : match-3, cascade,
  multiplicateur.
- **Candy Crush Saga** (King, 2012) — objectifs variés, moves plutôt
  que timer, feedback visuel très travaillé.
- **Puzzle & Dragons** (GungHo, 2012) — esthétique asiatique, sons
  satisfaisants.
- **Monument Valley** (ustwo, 2014) — approche contemplative du puzzle.
- **Journey** (thatgamecompany, 2012) — rythme lent, narration
  environnementale, pas de dialogue.
- **Peintures Song** (Li Cheng, Fan Kuan, Guo Xi, XIe siècle) —
  composition en trois plans (proche, moyen, lointain), brume
  intercalée.
- **Poésie Tang** (Li Bai, Du Fu, Wang Wei, Meng Haoran) — narration
  fragmentaire, silencieuse.
- **Zachtronics** — pattern de hint *shape-matched* (anneau doré qui
  épouse la silhouette du jade suggéré).

---

## 3. Scope — état actuel

| Élément | Livré | Post-release (bonus) |
|---|---|---|
| **Grille** | 8×8, 6 types de jade, 6 silhouettes distinctes | — |
| **Mécanique** | Swap adjacent, match-3+, cascade | + bombes, + lignes spéciales |
| **Cascade** | Multiplicateur 1 + 0.5 N cap ×5, **tiers 0-4** pour le juice, **slow-mo** au tier 3+ | — |
| **Objectifs** | Score / ClearJade / FillPoem (compte les cascades) | + obstacles, + cage, + glace |
| **Coups** | Limités (18-35 selon niveau) | + moves bonus |
| **Narration** | 12 niveaux, 4 chapitres, poème 20 caractères, épilogue | + saisons, + autres poèmes |
| **Progression** | Étoiles (1-3), **meilleurs temps par palier**, persistance RON | + achievements, + mode zen |
| **Juice** | Tween, particules polygonales, floating text, screen shake, **shockwave, radial burst, vignette, flash, slow-mo** | — |
| **Animations** | Fall, bounce, shrink, spawn | + trail, + wave |
| **Art** | Jades procéduraux, board en bois laqué, cadre doré | + textures |
| **Police** | DejaVu Sans Mono (défaut) + subset Noto Sans SC (poème) | — |
| **Écrans** | Menu, intro chapitre, jeu, victoire, défaite, aide, épilogue, teaser Halloween | + boutique, + carte |
| **Sons** | Playlist séquentielle 3 pistes (soloud → afplay/ffplay/mpv) + SFX tease jumpscare | + slider volume |
| **Cinématiques** | Épilogue, teaser Halloween, jumpscare final | — |
| **Aide** | Modal in-game (Tab/F1) : 6 jades, gameplay, objectifs, contrôles | — |
| **Hints** | 3 par partie, shape-matched, 3 s, `[H]` | — |
| **Replay** | Continue au-delà du 1er seuil si le niveau est déjà complété | — |

**Tests livrés** : 265 unit (lib) + 10 (main) + 1 balance + 22
intégration + 1 objectives (+1 ignoré) = **299 tests verts**.

---

## 4. Core loop

### 4.1 Frame par frame (en jeu)

1. Lire l'input (souris → hover + clic, clavier → navigation).
2. Tick du juice et du timer de hint (indépendants du slow-mo).
3. **Si animation en cours** (swap, résolution, chute, cascade) :
   tick les tweens et retour.
4. **Si idle** : lire le clic.
   - Premier clic sur une tuile → sélection.
   - Deuxième clic sur tuile adjacente → tentative de swap.
   - Clic sur tuile non adjacente → change la sélection.
   - Clic hors grille → désélectionne.
   - `[H]` → demande un hint (décrémente le compteur, sauf si aucun
     swap objectif n'est trouvable).
5. **Swap attempt** :
   - Animer le swap (0.15 s).
   - Si le swap crée un match → valider, décrémenter `moves_left`.
   - Sinon → animer le swap inverse (0.15 s).
6. **Résolution de match** :
   - Identifier toutes les tuiles alignées (dédupliquées).
   - Calculer le gain, mettre à jour score et objectif.
   - Émettre shockwave, floating text, shake, flash, rayons selon le
     **tier** de cascade.
   - Retirer les tuiles après un shrink de 0.25 s.
7. **Chute** :
   - Les tuiles au-dessus tombent (interpolation exponentielle, `K =
     15.0`), les nouvelles spawnent au-dessus de l'écran.
8. **Cascade** : si la chute crée de nouveaux matchs, retour à
   l'étape 6 avec `cascade_level += 1`, capé à 15.
9. **Fin de niveau** :
   - Si objectif atteint et `stop_on_objective` → victoire.
   - Si `moves_left == 0` → victoire ou défaite selon que l'objectif
     est atteint. En mode replay (niveau déjà complété),
     `stop_on_objective = false` : le joueur va jusqu'au bout des
     coups.

### 4.2 Partie par partie

```
Menu (carte des chapitres) → clic sur un niveau débloqué
    → Intro (si premier niveau du chapitre)
    → Jeu → objectif atteint → Victoire → révélation du poème
    → Entrée → Menu (progression sauvegardée)

    → moves épuisés sans objectif → Défaite → Entrée → Menu
                                            → R → Rejouer
```

Après le dernier niveau :

```
Jeu → Victoire → Épilogue (poème complet, pinyin, prose)
    → Entrée → Teaser Halloween (si sept-nov et non vu) → Menu
    → Entrée → Menu (sinon)
```

Pas de sauvegarde mid-partie. Score, étoiles, temps, fragments de
poème et flags cinématiques persistent dans `progress.ron`.

---

## 5. Modèle de données

### 5.1 Les 6 jades

Chaque type a une couleur, un nom chinois, une signification, et
surtout une **silhouette distincte** :

| ID | Nom | Chinois | Silhouette | Couleur | Signification |
|---|---|---|---|---|---|
| `bai` | Jade Blanc | 白玉 | Rond | Blanc ivoire `#F5EFE0` | Pureté, deuil |
| `bi` | Jade Vert | 碧玉 | Hexagone | Vert émeraude `#3D9B6A` | Harmonie, croissance |
| `qing` | Jade Bleu | 青玉 | Diamant | Bleu céleste `#4A7BC4` | Esprit, immortalité |
| `hong` | Jade Rouge | 红玉 | Octogone | Rouge cinabre `#B23A38` | Joie, célébration |
| `huang` | Jade Jaune | 黄玉 | Pentagone | Jaune ambré `#D4A54E` | Terre, prospérité |
| `mo` | Jade Noir | 墨玉 | Carré | Noir encre `#2A2A2E` | Mystère, encre |

**Rendu procédural** en 11 passes par tuile :
1. Ombre multi-couches (3 copies décalées).
2. Rim profond (copie quasi-noire).
3. Corps en 7 polygones imbriqués (dégradé vertical + bevel bas).
4. Sheen supérieur.
5. Inner glow.
6. Core glow pulsé (`get_time`).
7. Détails spécifiques à la forme : veines Bézier pour le rond,
   facettes pour l'hexagone, rayons pour le diamant, anneau + accents
   cardinaux pour l'octogone, fleur pour le pentagone, striures
   d'encre pour le carré.
8. Rim intérieur clair.
9. Rim extérieur brillant.
10. Specular principal (haut-gauche, halo + cœur).
11. Specular secondaire (bas-droite).

### 5.2 Structure de la grille

```rust
pub const GRID_W: usize = 8;
pub const GRID_H: usize = 8;
pub const JADE_TYPES: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Jade { Bai, Bi, Qing, Hong, Huang, Mo }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tile {
    pub jade: Jade,
    pub row: usize,       // logique, stable
    pub col: usize,
    pub visual_row: f32,  // animable (swap, chute)
    pub visual_col: f32,
    pub vein_seed: u32,   // seed déterministe pour les détails
}

pub struct Grid {
    /// Toujours plein (64 tuiles). Pas d'Option.
    pub tiles: [[Tile; GRID_W]; GRID_H],
}
```

**Différence par rapport à la Session 1** : plus de `TileState` ni
d'`Option<Tile>` — la grille est logiquement toujours pleine, et
l'animation se contente d'interpoler `visual_row`/`visual_col` vers
`row`/`col`. La logique de swap et de résolution passe par des
structures éphémères dans `systems.rs` (`SwapAnim`, `Resolution`).

### 5.3 Configuration des niveaux

Charge `assets/levels.ron` via `paths::asset()`.

```ron
[
    (id: "l01", name: "Arrival at the Garden", chapter: 0, moves: 20,
     objective: Score(2100), star_target: 2100, poem_reveal: 2, seed: 5),
    // ...
]
```

**Objectifs** :

```rust
pub enum Objective {
    /// Atteindre `target` points.
    Score(u32),
    /// Éliminer `count` jades du type `jade`.
    ClearJade(Jade, u32),
    /// Déclencher `count` cascades (le match initial est gratuit).
    FillPoem(u8),
}
```

`star_target` définit le seuil du 1er palier d'étoiles, indépendamment
de l'objectif principal (utile pour `ClearJade` et `FillPoem`).

### 5.4 Persistance

```rust
pub struct Progress {
    pub best_scores: HashMap<String, u32>,
    pub stars: HashMap<String, u8>,
    pub poem_fragments: u8,        // 0..20
    pub epilogue_seen: bool,
    pub halloween_seen: bool,
    pub best_times: HashMap<String, LevelTimes>,
}

pub struct LevelTimes {
    pub time_1_star: Option<f32>,
    pub time_2_star: Option<f32>,
    pub time_3_star: Option<f32>,
    pub best_score_time: Option<f32>,
}
```

Le fichier est écrit dans le dossier utilisateur de la plateforme
(voir §6.13 `paths.rs`). Il est créé à la première sauvegarde.

### 5.5 Étoiles

- **1 ⭐** : objectif atteint.
- **2 ⭐** : score ≥ `star_target` × 1.5.
- **3 ⭐** : score ≥ `star_target` × 2.0.

Les paliers sont calculés dynamiquement ; seuls les temps de première
atteinte par palier sont mémorisés dans `LevelTimes`.

---

## 6. Systèmes

Chaque système est un module de `src/`. Ordre d'exécution ≈ §4.1.

### 6.1 `components.rs` — Types de base

`Jade`, `JadeShape`, `Tile`, constantes de grille. Module **pur**,
sans `macroquad`. Testable headless.

### 6.2 `config.rs` — `GameContext`

Palette (dark lacquer + gold + cinnabar), layout, helpers de hit-test.
Chargé depuis `.env` en dev (`COLOR_*`), avec valeurs par défaut
codées en dur.

### 6.3 `grid.rs` — Logique de match-3

Fonctions pures sur `Grid` :
- `find_matches() -> Vec<(usize, usize)>` — dédupliqué via `HashSet`.
- `has_match_at(row, col) -> bool`.
- `has_any_valid_move() -> bool`.
- `swap(a, b)` — met aussi à jour `visual_row`/`visual_col`.
- `remove_and_collapse(coords, rng)` — compaction + refill en un seul
  passage, en gardant l'ancienne `visual_row` pour l'animation de
  chute et en spawnant les nouvelles tuiles en négatif.
- `reshuffle(rng)` — Fisher-Yates sur le multiset, vérification
  match + coup valide, fallback régénération.
- `pick_safe_jade(tiles, row, col, rng)` — évite les auto-matchs à la
  génération.

### 6.4 `scoring.rs` — Score et multiplicateur

**Règles réelles** (post-rebalance) :
- 3 jades → **60** points de base.
- Chaque tuile supplémentaire : **+20** (4 → 80, 5 → 100, 6 → 120…).
- **Multiplicateur de cascade** : × (1 + 0.5 × cascade_level), cap ×5.
- **Tiers de présentation** 0-4 pour le juice (`cascade_tier`) :
  - tier 0 : pas de combo.
  - tier 1 : premier combo.
  - tier 2 : rayons radiaux, vignette légère.
  - tier 3 : flash plein écran, **slow-mo**.
  - tier 4 : intensité maximale.

### 6.5 `animation.rs` — Tweens locaux

`Tween` + `Easing` (`Linear`, `EaseIn`, `EaseOut`, `EaseInOut`,
`Bounce`, `Elastic`). Local au jeu — candidat extraction
`ember_stdlib::animation` à la 2e occurrence.

### 6.6 `juice.rs` — Feedback visuel

Primitives :
- `Particle` polygonal (3 ou 4 côtés, jamais des cercles — plus
  organique pour le jade).
- `FloatingText`.
- `Shockwave` — anneau expansif à chaque match.
- `RadialRay` — rayons fins (burst) à partir du tier 2.
- `Vignette` — assombrissement des bords, un seul à la fois.
- `ScreenShake` — RNG déterministe, force remplace si plus forte.
- `Flash` — overlay plein écran, alpha qui fade.

Candidat extraction `ember_stdlib::juice` (3e occurrence confirmée
après Bullet Hell et ember-wars).

### 6.7 `scroll_painting.rs` — Fond parallax

Trois couches de montagnes stylisées (triangles), brume intercalée,
dérive automatique à `DRIFT_SPEED = 2.5 px/s`. Hash déterministe
inspiré de `ember-wars`. Les invariants de couleur (alpha croissant
vers l'avant, near < 1.0) sont vérifiés à la compilation via des
`const _: () = assert!(...)`.

### 6.8 `tile_render.rs` — Dessin des jades

Voir §5.1. `jade_shape_params(jade)` retourne `(sides, rotation)`
pour chaque silhouette. `generate_veins(seed, size)` produit 3
courbes de Bézier déterministes.

### 6.9 `menu.rs` — Menu / carte

Palette locale (indépendante du `.env`), header/footer opaques
dessinés en dernier (les cartes ne peuvent jamais baver dessus).
Chapitres scrollables, info panel à droite, boutons debug cachés
derrière Ctrl en build dev.

### 6.10 `systems.rs` — State struct `Game`

```rust
pub struct Game {
    pub phase: Phase,
    pub grid: Grid,
    pub level: LevelConfig,
    pub score: u32,
    pub moves_left: u32,
    pub cascade_level: u32,
    pub target_score: u32,
    pub selected: Option<(usize, usize)>,
    pub hover: Option<(usize, usize)>,
    pub swap_anim: Option<SwapAnim>,
    pub resolution: Option<Resolution>,
    pub juice: Juice,
    pub rng: Rng,
    pub time: f32,             // horloge visuelle
    pub elapsed: f32,          // horloge de partie (best_times)
    pub objective_progress: u32,
    pub slowmo_timer: f32,
    pub slowmo_strength: f32,
    pub hint: Option<HintState>,
    pub hint_uses_left: u32,
    pub stop_on_objective: bool,
    pub crossed_1_star: Option<f32>,
    pub crossed_2_star: Option<f32>,
    pub crossed_3_star: Option<f32>,
    pub peak_score: u32,
    pub peak_score_time: f32,
}
```

`Phase` : `Playing`, `Won`, `Lost`.

`ResolveStep` : `Shrinking`, `Falling`, `Checking`.

**Timers** :
- `time` et `elapsed` **ne sont pas ralentis** par le slow-mo (le
  timer affiché et les best_times sont en temps réel — un joueur ne
  peut pas « farmer » un record en déclenchant du slow-mo).
- Le slow-mo s'applique uniquement au `dt` de gameplay (swap, chute,
  résolution).

### 6.11 `hud.rs` — Affichage in-game

- Top bar : titre, nom du niveau, timer, score, target.
- Panneau objectif : label, progression, barre, moves restants,
  disponibilité du hint, `[Tab] Help`.
- Indicateur de cascade pulsé au centre-bas pendant les chaînes.
- Overlay de défaite (dessiné ici, pas dans `main.rs`).

### 6.12 `help.rs` — Modal d'aide

Accessible via Tab/F1 depuis le menu, l'intro ou le jeu. Détaille les
6 jades (icône, nom chinois, latin, signification), le gameplay, les
3 types d'objectifs, et les contrôles. Fond assombri, double cadre
doré.

### 6.13 `paths.rs` — Résolution des chemins runtime

Résolution en cascade, sans dépendance :

**Assets (lecture)** :
1. `$JADE_GARDEN_ASSETS`
2. `<exe_dir>/assets` (layout release)
3. `CARGO_MANIFEST_DIR/assets` (dev)

**`progress.ron` (lecture/écriture)** :
1. `$JADE_GARDEN_PROGRESS`
2. Dossier utilisateur de la plateforme
   (`~/Library/Application Support/jade-garden/` sur macOS, etc.)
3. `CARGO_MANIFEST_DIR/progress.ron` (dev)

`ensure_parent_dir` crée le dossier cible à la première sauvegarde.

### 6.14 `fonts.rs` — Chargement des polices

- **DejaVu Sans Mono** : police par défaut, contient les accents
  français, les étoiles ★☆, la ponctuation Unicode.
- **Noto Sans SC (subset)** : uniquement les glyphes du poème, du
  titre, de l'auteur et des noms de jades. Chargée séparément via
  `PoemFont`. Fallback gracieux si absente (tofu, pas de crash).

### 6.15 `audio.rs` — Musique + SFX

Backend hybride :
1. **soloud** (bundled) si disponible.
2. Sinon **sous-processus** : `afplay` (macOS), `ffplay`/`mpv`
   (Linux, Windows).

Playlist séquentielle : intro → boucle sur les pistes non-intro. Si
une piste manque, essaie les suivantes en round-robin. Si aucune ne
marche, musique désactivée, jeu continue.

SFX : `play_scare()` / `play_scream()` pour le teaser Halloween,
avec override `assets/scare.wav` / `assets/scream.wav`, fallback
sur les sons système macOS (`Glass.aiff` / `Sosumi.aiff`).

### 6.16 `poem.rs` — Poème《春晓》

20 caractères, 4 lignes × 5, pinyin avec tons, position/index,
titre, auteur.

### 6.17 `level.rs` — Chargement des niveaux et chapitres

Charge `levels.ron` et `chapitres.ron` via `paths::asset`.
`objective_label` produit les libellés anglais pour le HUD et le menu.

### 6.18 `main.rs` — Boucle

Machine à états à 5 écrans : `Menu`, `Intro(idx)`, `Game(idx)`,
`Epilogue`, `Halloween`. Deux cinématiques implémentées en dur
(épilogue = poème animé ; teaser = zoom alien + jumpscare final avec
`play_scream` à 0.7 s). `is_spooky_season()` utilise une conversion
Unix → mois **exacte** (règles grégoriennes complètes).

---

## 7. State struct & Phases

### 7.1 Machine à états

```
AppScreen::Menu
    ↓ clic niveau débloqué
AppScreen::Intro(idx)  (1er niveau du chapitre uniquement)
    ↓ clic / Entrée / Espace
AppScreen::Game(idx)
    Phase::Playing
        ↓ objectif atteint (et stop_on_objective)
    Phase::Won → overlay poème
        ↓ Entrée → go_to_next_reveal(idx)
            → Menu (niveau normal)
            → Epilogue (dernier niveau + épilogue non vu)
            → Halloween (épilogue vu + sept-nov + non vu)
            → Menu (sinon)

    Phase::Lost (moves épuisés, objectif non atteint)
        ↓ Entrée / Échap → Menu
        ↓ R → Rejouer
```

### 7.2 Timers

- Swap : 0.15 s.
- Shrink avant disparition : 0.25 s.
- Chute : interpolation exponentielle, pas de durée fixe (converge en
  quelques frames).
- Hint TTL : 3 s.
- Slow-mo : 0.25 s, strength 0.30–0.55 selon le tier.
- Cinématiques : constantes nommées en tête de `main.rs`.
- **Pas de timer de partie**. Le timer affiché est purement
  indicatif.

---

## 8. Structure des fichiers

```
games/jade-garden/
├── DESIGN.md                (ce fichier)
├── RELEASE.md               checklist release
├── README.md                contrôles, install, dev
├── CREDITS.md               musique, polices, poème
├── Cargo.toml
├── .env                     overrides couleur (dev)
├── .gitignore               (progress.ron, .env)
├── subset_font.sh           génère NotoSansSC-JadeGarden.otf
├── assets/
│   ├── levels.ron           12 niveaux
│   ├── chapitres.ron        4 chapitres × 3 niveaux
│   ├── DejaVuSansMono.ttf
│   ├── NotoSansSC-JadeGarden.otf   (subset CJK)
│   └── sonican-*.mp3        3 pistes musicales
├── src/
│   ├── main.rs              boucle + cinématiques
│   ├── lib.rs               exports publics
│   ├── paths.rs             résolution runtime
│   ├── config.rs            GameContext + palette
│   ├── components.rs        Jade, Tile, constantes
│   ├── grid.rs              logique match-3
│   ├── scoring.rs           score + tiers de cascade
│   ├── animation.rs         Tween + Easing
│   ├── juice.rs             particles, shockwave, rays, vignette…
│   ├── scroll_painting.rs   fond shan shui
│   ├── tile_render.rs       dessin des jades
│   ├── menu.rs              carte des chapitres
│   ├── hud.rs               affichage in-game
│   ├── help.rs              modal d'aide
│   ├── level.rs             chargement niveaux + chapitres
│   ├── poem.rs             《春晓》 + pinyin
│   ├── progress.rs          persistance RON + temps
│   ├── systems.rs           Game state + tick
│   ├── fonts.rs             DejaVu + subset CJK
│   └── audio.rs             playlist + SFX
└── tests/
    ├── game_scenarios.rs    22 tests d'intégration
    ├── balance.rs           rapport d'équilibrage (greedy AI)
    └── objectives.rs        faisabilité des objectifs
```

---

## 9. Extractions moteur anticipées

| Primitive | Occurrences | Action |
|---|---|---|
| `Juice` (particles + text + shake + flash + vignette) | 3 (Bullet Hell, ember-wars, jade-garden) | **Prêt à extraire** |
| Police Unicode + subset | 3 (FreeCell, ember-wars, jade-garden) | **Prêt à extraire** |
| `Tween` / `Easing` | 1 | Attendre 2e |
| Fond parallax shan shui | 2 (ember-wars → jade-garden) | Attendre 3e |
| Menu campagne scrollable | 2 (ember-wars → jade-garden) | Attendre 3e |
| `Persistence<T>` | déjà extrait | — |
| `Cooldown` | déjà extrait | — |

**Note stratégique** : après jade-garden, `Juice` et la police Unicode
sont mûrs pour `ember_stdlib`. À traiter en phase refacto dédiée,
post-release.

---

## 10. Stratégie de tests

**Livrés** : 265 unit (lib) + 10 (main) + 1 balance + 22 intégration
+ 1 objectives (+1 ignoré) = **299 verts**.

### 10.1 Couverture par module

| Module | Tests |
|---|---|
| `animation.rs` | 24 |
| `audio.rs` | 6 |
| `components.rs` | 12 |
| `config.rs` | 10 |
| `fonts.rs` | 1 |
| `grid.rs` | 32 |
| `help.rs` | 3 |
| `juice.rs` | 26 |
| `level.rs` | 9 |
| `menu.rs` | 22 |
| `paths.rs` | 5 |
| `poem.rs` | 9 |
| `progress.rs` | 16 |
| `scoring.rs` | 32 |
| `scroll_painting.rs` | 11 |
| `systems.rs` | 33 |
| `tile_render.rs` | 17 |
| **Total unit (lib)** | **265** |
| `main.rs` (reveal + dates) | 10 |
| `tests/game_scenarios.rs` | 22 |
| `tests/balance.rs` | 1 |
| `tests/objectives.rs` | 1 (+1 ignored) |

### 10.2 Tests prioritaires

**`grid.rs`** — la partie la plus critique : détection de lignes
horizontales/verticales de 3, 4, croix (dédup), gravity sur N cases,
multiset préservé au reshuffle, déterminisme du seed.

**`scoring.rs`** : 60 pour 3, +20 par tuile, multiplicateur 1 + 0.5N
cap ×5, monotonie, tiers de cascade monotones et capés à 4,
`compute_stars` (0/1/2/3) avec ses bornes exactes.

**`systems.rs`** : sélection/désélection, adjacence 4-voisinage
(diagonales exclues), `request_hint` sur les 3 types d'objectifs,
expiration du hint après 3 s, limite de 3 hints, slow-mo réduit
`current_time_scale` sans le mettre à zéro.

**`balance.rs`** : invariant — aucun niveau ne peut être `IMPOSSIBLE`
(AI greedy ne peut pas atteindre `star_target`). Panique le test si
une régression casse l'équilibrage.

**`objectives.rs`** : rapport par niveau + scanner de seeds (ignoré
par défaut, `--ignored`).

### 10.3 Ce qu'on ne teste PAS

- Rendu visuel (pas de snapshot).
- Ressenti des easings (validé en jeu).
- Les cinématiques (animées en temps réel).

---

## 11. Narration — Les 12 niveaux

### Chapitre 1 : The Arrival

> Xiao Lin has walked for three days. At dusk, a wall of moss-covered
> stone appears. Behind it: an abandoned garden. At its centre, a
> stele carved with a poem whose characters are almost worn away.

- **L01 — Arrival at the Garden** — Score 2100, 20 coups.
- **L02 — The Broken Stele** — Clear 15 碧玉 (Bi), 20 coups.
- **L03 — First Steps** — Score 2200, 18 coups.

**Caractères révélés** : 春眠不 (3).

### Chapitre 2 : The Silence of Stones

> The garden is silent, but the stones carry memory. Xiao Lin finds
> shards of pottery, fossilised petals. He understands that he must
> awaken the jades one by one.

- **L04 — The Pottery** — Score 3500, 20 coups.
- **L05 — Fallen Petals** — Clear 18 红玉 (Hong), 18 coups.
- **L06 — The First Awakening** — Score 2800, 22 coups.

**Caractères révélés** : 觉晓处 (3, total 6).

### Chapitre 3 : The Breath of Spring

> One morning, Xiao Lin hears a bird. It is the first sound since his
> arrival. The jades fall into place more easily, as if the garden
> were answering his patience.

- **L07 — The First Bird** — Score 2700, 22 coups.
- **L08 — Wind in the Pines** — Clear 22 青玉 (Qing), 20 coups.
- **L09 — The Fragrance of Flowers** — FillPoem 4 cascades, 25 coups.

**Caractères révélés** : 处闻啼鸟 (4, total 10).

### Chapitre 4 : The Awakening

> The poem comes together. Xiao Lin reads the restored characters
> aloud. The garden shivers. A soft green light rises from the ground.

- **L10 — The Missing Characters** — FillPoem 6 cascades, 28 coups.
- **L11 — The Imperial Seal** — Clear 30 墨玉 (Mo), 24 coups.
- **L12 — The Garden Awakens** — FillPoem 10 cascades, 35 coups.

**Caractères révélés** : 夜来风雨声花落知多少 (10, total 20).

### Épilogue

Le poème complet s'illumine caractère par caractère, avec pinyin.
Titre, auteur, prose finale (« Xiao Lin sits down… »), puis prompt
`[Enter] return to menu`.

### Teaser Halloween

Écran noir → flash blanc + son aigu → alien procédural qui zoome
(yeux qui suivent la souris) → message personnalisé (« Wang Yi, you
weren't supposed to see this. ») → fausse fin sur `[Enter]` → écran
noir 0.7 s → `play_scream()` + alien plein écran → retour menu.
Déclenché **une seule fois**, entre septembre et novembre.

---

## 12. Juice — Plan détaillé

### 12.1 Match simple (tier 0)

1. Shrink 1 → 0 sur 0.25 s (`EaseInOut` implicite).
2. Shockwave : rayon 40, épaisseur 1.5, couleur ivoire, 0.35 s.
3. Floating text : `+60`, taille 22, ivoire.
4. Screen shake : 2.0, 0.15 s.
5. Particules jade : 10 polygones par tuile.

### 12.2 Cascade (tiers 1-4)

Escalade par tier :
- **tier 1** : floating text jaune pâle, taille 27, shockwave dorée.
- **tier 2** : rayons radiaux (6), vignette pulsée, flash léger.
- **tier 3** : 10 rayons, flash chaud, **slow-mo** (strength 0.30).
- **tier 4** : 14 rayons, flash orange, slow-mo (strength 0.40+).

Banner `CASCADE ×N` flottant, taille croissante.

### 12.3 Swap

- Sélection : halo shape-matched pulsé + 4 coins dorés.
- Swap : interpolation 0.15 s.
- Rejet : swap inverse + shake léger.
- Hover : double anneau pulsé **qui épouse la silhouette** du jade
  (jamais clippé, contrairement à l'ancien rectangle).

### 12.4 Fin de niveau

**Victoire** :
- Flash doré plein écran (via `juice.flash`) + shake 12.
- Overlay poème avec les caractères révélés un par un
  (`CHAR_REVEAL_DURATION = 0.35 s`), pinyin en dessous, titre et
  auteur.
- Indicateur `N / 20`.
- `[Enter] continue   [R] replay   [Esc] menu` après la révélation.

**Défaite** :
- Overlay sombre (alpha 0.72).
- « The jades fall asleep » + score + cible.
- `[Enter] menu   [R] retry   [Esc] menu`.

### 12.5 Ambiance générale

- Halo des jades : pulse sinusoïdal permanent.
- Fond : dérive automatique, brume intercalée.
- Timer visible en haut à droite, purement indicatif.

---

## 13. Pièges connus / décisions délicates

### 13.1 Hérités du workspace

- `env_color` : `{PREFIX}_R/_G/_B/_A`, pas une chaîne.
- `clippy::collapsible_if` : let-chains.
- `clippy::too_many_arguments` : bundle en tuple ou `#[allow]`.
- `clippy::doc_overindented_list_items` : indentation stricte.
- `gen` réservé → `id_gen`/`rng_state`.
- `should_implement_trait` sur `next()` custom → `next_id()`.
- `set_default_font` après `load_ttf_font`.

### 13.2 Spécifiques à jade-garden

- **Détection de match** : scanner horizontal + vertical, puis
  dédupliquer via `HashSet`. Ne jamais itérer sur le résultat brut.
- **Swap valide** : après `grid.swap(a, b)`, tester
  `has_match_at(a)` || `has_match_at(b)`. Ne pas scanner la grille
  entière.
- **Cascade infinie** : cap à `MAX_CASCADE = 15`. Au-delà, on sort
  et on reshuffle si la grille est bloquée.
- **Dead-lock** : détecté en fin de cascade via `has_any_valid_move`,
  suivi d'un `reshuffle`.
- **Refill et auto-match** : `pick_safe_jade` évite les runs de 3+
  au spawn. Fallback déterministe si aucun type autorisé.
- **Grille toujours pleine** : `[[Tile; GRID_W]; GRID_H]`, pas
  d'`Option`. Simplifie le code et la copie (`Copy` → test swaps
  gratuits).
- **Chemins runtime** : `env!("CARGO_MANIFEST_DIR")` pointe sur la
  source au compile time. Utiliser `paths::asset()` /
  `paths::progress_path()` **partout**.
- **Timers** : `elapsed` et `time` ne subissent pas le slow-mo. Les
  `best_times` sont en temps réel.
- **FillPoem** : ne compte que les cascades (`cascade_level > 0`).
  Le match initial après un swap est gratuit.
- **Chute** : interpolation exponentielle (`K = 15.0`), pas de tween
  à durée fixe. Converge sans ralentir le jeu.
- **Clip d'anneaux de sélection** : utiliser la **silhouette** du
  jade pour dessiner les anneaux pulsés, pas un rectangle — sinon
  coupé par la tuile du dessous.

### 13.3 Décisions validées

- ✅ Nom : `jade-garden`.
- ✅ Grille 8×8, 6 types de jade, 6 silhouettes distinctes.
- ✅ Match-3 classique avec cascade, multiplicateur cap ×5.
- ✅ Coups limités (pas de timer de fin).
- ✅ 4 chapitres × 3 niveaux = 12 niveaux.
- ✅ Poème《春晓》de Meng Haoran.
- ✅ 3 types d'objectifs : Score, ClearJade, FillPoem.
- ✅ Étoiles 1-3 basées sur le score vs `star_target`.
- ✅ Persistance RON dans le dossier utilisateur.
- ✅ Juice riche : shockwave, rayons, vignette, flash, slow-mo.
- ✅ Fond parallax shan shui, dérive automatique.
- ✅ Jades procéduraux, board en bois laqué.
- ✅ Fenêtre 1200×900, redimensionnable.
- ✅ Code 100 % anglais, sauf les données chinoises volontaires
  (poème, noms des jades, auteur).
- ✅ Audio hybride soloud → sous-processus système.
- ✅ Cinématiques épilogue + teaser Halloween.

---

## 14. Hors-scope / Open questions

### 14.1 Post-release

- **Stèle dans le menu** après 20/20 : bouton pour revoir le poème
  complet avec pinyin.
- **Slider de volume** dans le menu.
- **Screenshot de victoire** pour partage.
- **Achievements** style Steam.
- **Bombes / lignes spéciales** (match 4+, match 5).
- **Mode zen** : pas d'objectif, juste jouer.
- **Extraction `ember_stdlib::juice`** (3e occurrence confirmée).
- **Extraction police Unicode** (3e occurrence).
- **Fallback hint FillPoem** : si aucun swap n'avance l'objectif,
  pointer sur *n'importe quel* swap produisant un match, pour ne
  jamais consommer un hint en silence. Théorique aujourd'hui (les 3
  seeds réels passent).

### 14.2 Open questions

1. **Icône de fenêtre** : à ajouter avant release si un PNG est
   disponible, sinon skip.
2. **Format de bundle** : `binaire + assets/` côte à côte, ou
   `include_bytes!` pour tout ce qui est petit ? Le premier est
   retenu pour la release.
3. **Skip de musique** : actuellement, si une piste manque, on
   essaie les suivantes en round-robin. À voir si on veut logguer
   plus agressivement, ou éteindre complètement.

---

## 15. État actuel

Voir [`RELEASE.md`](RELEASE.md) pour la checklist de release
détaillée et l'état des tests. Le design initial (§1-§12) reste la
référence pour la mécanique, la narration et la palette ; les écarts
par rapport à la Session 1 sont documentés dans les sections
correspondantes ci-dessus :

- `components.rs` : plus de `TileState`, grille toujours pleine.
- `scoring.rs` : 60 pts pour 3, +20 par tuile, tiers 0-4.
- `systems.rs` : `stop_on_objective`, slow-mo, hints, temps.
- `main.rs` : 5 écrans d'état (menu, intro, jeu, épilogue, teaser).
- `paths.rs` : résolution runtime des assets et de `progress.ron`.
- `audio.rs` : backend hybride + SFX.
- `help.rs` : modal d'aide.
- Palette : dark lacquer + or + cinabre, remplaçant le shan shui
  pâle de la Session 1.
- Localisation : anglais partout sauf les données chinoises.

---

## 16. Ce que ce jeu ajoute au moteur

| Nouvelle capacité | Précédent max |
|---|---|
| **Tweens / easing** | Aucun (0 occurrence) |
| **Animation de chute sans tween** (interpolation exp.) | Aucune |
| **Détection de pattern 2D** (match-3) | Simple grille |
| **Cascade / chaînage avec multiplicateur + tiers** | Aucun |
| **Narration par fragments + cinématiques** | Aucune |
| **Juice intensif** (3e occurrence) | **Prêt à extraire** |
| **Police CJK subset** (3e occurrence) | **Prêt à extraire** |
| **Fond parallax contemplatif** | 2e occurrence |
| **Résolution runtime des assets** (`paths.rs`) | 1re occurrence |
| **Audio hybride soloud + sous-processus** | 1re occurrence |

**Prochaines extractions probables** : `ember_stdlib::juice` et la
police Unicode. À traiter en phase refacto séparée, après la release.

---

*Ce design privilégie la contemplation et la narration sur le défi
brut. Le joueur joue à son rythme, avec un objectif clair et une
récompense narrative à chaque victoire. La mécanique match-3 est un
support, pas une fin — le vrai sujet, c'est le jardin qui revit et
le poème qui se recompose.*