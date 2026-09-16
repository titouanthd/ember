# FreeCell — Design Document

## État actuel (post-V1)

FreeCell est **entièrement jouable**. Tous les objectifs V1 sont atteints :

- Règles complètes (colonnes, cellules, fondations, super-move)
- Drag-and-drop avec feedback visuel (contour vert/rouge sur la cible)
- Undo/redo, hint, restart, menu, new game
- Best time persisté par seed
- 89 tests verts (81 unitaires + 8 intégration), Clippy clean

**Raccourcis clavier actuels** (cross-layout QWERTY/AZERTY) :

- `N` : nouvelle partie (seed aléatoire)
- `R` : restart (même seed)
- `Tab` : menu (fallback `M` / `;` selon layout)
- `Z` ou `W` : undo
- `Y` : redo
- `Escape` : quitter

**Décisions finales (par rapport aux prévisions V1)** :

- Raccourcis sans modificateur (pas `Ctrl+Z` / `Ctrl+Y`).
- `Tab` au lieu de `M` pour le menu (contrainte AZERTY).
- Auto-complete **reporté V2** (pas fait en V1).
- Saisie de seed **reportée V2** (bouton « New » uniquement en V1).
- Sélection clic-clic **reportée V2** (drag only).
- Unicode activé (police DejaVu Sans Mono chargée).

---

## Concept

FreeCell, le solitaire à information parfaite (1978, Paul Alfille). Un
jeu de 52 cartes, toutes visibles dès le départ. Pas de pioche, pas de
hasard pendant la partie. Le joueur déplace des cartes entre 8 colonnes
du tableau, 4 cellules libres (en haut à gauche), et 4 fondations
(en haut à droite).

Objectif : déplacer toutes les cartes vers les fondations, par couleur
et dans l'ordre croissant (A → 2 → ... → K).

Le thème visuel est un terminal dev : fond sombre, cartes colorées
rouge/noir, symboles Unicode (♠ ♥ ♦ ♣).

## Thème

**Terminal dev**. Fond sombre, symboles de cartes en Unicode (DejaVu
Sans Mono chargée au démarrage, fallback ASCII S H D C documenté).

- Fond de fenêtre : gris très sombre
- Cartes : fond clair, bordure discrète
- Rouges (♥ ♦) : rouge vif
- Noirs (♠ ♣) : blanc cassé ou noir profond
- Fondations vides : placeholder avec symbole de la couleur (alpha 0.45)
- Cellules vides : petit carré discret

## Règles de FreeCell

### Plateau

- **8 colonnes** (tableau) : reçoivent les cartes distribuées au départ.
- **4 cellules libres** : peuvent contenir 1 carte chacune. Utiles comme
  zone de transit.
- **4 fondations** : une par couleur, reçoivent les cartes de A à K.
  La partie est gagnée quand les 4 sont complètes.

### Distribution initiale

Les 52 cartes sont distribuées face visible dans les 8 colonnes :

- Colonnes 0-3 : 7 cartes chacune
- Colonnes 4-7 : 6 cartes chacune

**Toutes les cartes sont visibles.** Aucun hasard après la distribution.

### Déplacements valides

**Vers une colonne** : une carte peut être posée sur une autre si
- la carte de destination a une **valeur immédiatement supérieure**, et
- les deux sont de **couleur opposée** (rouge sur noir, noir sur rouge).

Exemple : ♦ 7 peut aller sur ♠ 8 ou ♣ 8. Pas sur ♥ 8.

**Vers une cellule libre** : n'importe quelle carte, si la cellule est
vide. Une carte par cellule maximum.

**Vers une fondation** : une carte peut aller sur une fondation si
- la fondation est vide et la carte est un **As** de la couleur
  correspondante, ou
- la fondation a une carte de valeur N, et la carte posée est N+1
  de la même couleur.

Exemple : ♥ A → ♥ 2 → ♥ 3 → ... → ♥ R. **Pas de saut** :
T → Q est refusé, il faut T → V → D → R.

### Déplacements de piles (super-move)

**Règle clé de FreeCell** : on peut déplacer une **séquence valide**
(descendante, couleurs alternées) d'un coup, dans la limite des cellules
libres et des colonnes vides.

**Formule** : `(1 + cellules_libres) × 2^colonnes_vides`.

Exemples :
- 4 cellules libres, 0 colonne vide : 5 cartes déplaçables.
- 4 cellules libres, 1 colonne vide : 10 cartes.
- 2 cellules libres, 0 colonne vide : 3 cartes.
- **0 cellule libre, 1 colonne vide : 2 cartes.** (piège classique)

**Piège** : si la destination est une **colonne vide**, elle ne compte
pas comme buffer disponible (elle est occupée par le move lui-même). La
formule officielle exclut la destination vide du calcul. **Bug latent
dans l'implémentation actuelle** : `max_super_move()` compte toutes les
colonnes vides sans filtrer la destination. À corriger en V2.

**Décision V1** : on implémente le **déplacement de pile valide**
jusqu'à la limite calculée. C'est essentiel pour FreeCell.

### Accessibilité des cartes

On ne peut prendre que le **run contigu du bas** d'une colonne. Les
cartes du milieu sont recouvertes par celles du dessous et ne sont pas
accessibles.

Validation : `start_idx + n == columns[col].len()`.

Exemple : colonne `[R♠, D♥, V♣, T♦]` (R en haut, T en bas).

- `is_accessible(Column(0), 3, 1)` → `3 + 1 == 4` ✔ (T♦)
- `is_accessible(Column(0), 2, 2)` → `2 + 2 == 4` ✔ (V♣, T♦)
- `is_accessible(Column(0), 1, 1)` → `1 + 1 == 2 != 4` ✘ (D♥ bloquée)

### Victoire

Toutes les cartes sont dans les fondations.

### Défaite

Il n'y a pas de défaite en FreeCell (contrairement à Klondike). On peut
toujours abandonner (bouton « give up ») mais pas perdre.

## Mécaniques

### Drag-and-drop

Le **cœur** du gameplay : cliquer-glisser une carte (ou une séquence)
d'une zone à une autre.

**États du drag** :

- `Idle` : rien en cours.
- `Pressing { origin, card, press_pos }` : souris enfoncée sur une
  carte (pas encore draggée). Utile pour distinguer un clic d'un drag.
- `Dragging { origin, start_index, cards, offset }` : la souris bouge,
  on affiche les cartes « collées » au curseur.

**Seuil** : 5 px de mouvement pour passer de `Pressing` à `Dragging`.

**Règles du drag** :

- On peut drag :
  - Le run valide à partir d'une carte **accessible** d'une colonne.
  - Une carte d'une cellule libre.
  - **Pas** une carte d'une fondation (une fois placée, elle est fixe).
- Pendant le drag, la carte (ou les cartes) suit la souris.
- Au drop, on vérifie la cible et on applique le déplacement si valide.
- Sinon, on annule (les cartes reviennent à leur position).

**Feedback visuel** : la zone cible est entourée d'un contour vert
(valide) ou rouge (invalide). La colonne entière est mise en évidence,
pas seulement la carte du haut.

### Sélection alternative (V2)

Pour les joueurs qui préfèrent cliquer plutôt que drag : cliquer une
carte pour la sélectionner, puis cliquer une destination. **V2.**

### Auto-complete (V2)

Quand toutes les cartes sont dans un état qui garantit une victoire
(toutes les colonnes triées, plus aucune contrainte), on peut finir
automatiquement. **V2.**

### Annuler / Refaire (V1)

**Raccourcis** : `Z` (ou `W` sur AZERTY) pour undo, `Y` pour redo.
Pas de modificateur Ctrl.

Stocke un historique des déplacements. Chaque entrée contient l'état
complet **avant** le move : les 8 colonnes, les 4 cellules, les 4
fondations. Comme l'état est petit (~52 cartes + 4 options), un snapshot
est peu coûteux.

**Comportement** : un nouveau move vide la pile `future` (redo). Undo
restaure l'état précédent et pousse l'état actuel dans `future`. Redo
fait l'inverse. Le timer ne s'arrête pas pendant undo/redo.

**Décision V1** : on implémente l'undo/redo dès la V1.

### Hint (V1, indicateur pur)

`find_hint()` retourne `Hint { from, to, card }` sans muter l'état.

**Priorité** :
1. Fondation (As, puis carte suivante).
2. Colonne (poser sur une carte existante).
3. Cellule libre (transit).

Le hint est dessiné comme un contour sur `from` et `to`, plus un texte
« Hint: X → zone ». Aucune modification de l'état de jeu.

### Solveur automatique (V3)

Un solveur automatique de FreeCell existe (algorithme A*). **V3.**

### Score

**Pas de score numérique.** FreeCell se mesure en :

- Nombre de coups (moves)
- Temps (best time par seed)

**Décision V1** : compter les **moves** et le **temps**. Persister le
**best time** par seed.

### Seed et reproductibilité

**FreeCell a une feature classique** : les parties sont numérotées. La
partie #1, #2, #3 ont des distributions de cartes **connues et
reproductibles**. Microsoft FreeCell a rendu cette feature célèbre.

**Décision V1** : implémenter la génération de parties numérotées.

- Le joueur peut générer une nouvelle partie aléatoire (touche `N`).
- La distribution est déterministe à partir de la seed.
- Le **best time** est persisté par numéro de partie.

**Note** : la distribution « classique » de Microsoft FreeCell utilise
un algorithme spécifique (LCG + shuffle). On utilise notre `Rng`
(xorshift32) + Fisher-Yates. Déterministe mais pas compatible MS.

## Réutilisation des briques moteur

| Brique | Provenance | Usage |
|---|---|---|
| Button, Label, Panel, TimerDisplay | ember-stdlib/ui | HUD, menu, écrans |
| Input | ember-stdlib | Souris (drag-and-drop) + clavier |
| Persistence<HashMap<String, f32>> | ember-stdlib | Best time par seed |
| GameContext | ember-core | Config .env |
| GameState | ember-core | Start / Playing / Win |
| Rng | ember-core | Mélange déterministe |

**Aucune brique nouvelle n'a été extraite** en V1. Ce jeu est la **1re
occurrence** du drag-and-drop (Rule of Three). À extraire dans
ember_stdlib si un 2e jeu en a besoin (Mahjong, Solitaire).

## Nouvelles briques potentielles (à extraire ?)

### Drag-and-drop

C'est **la** brique que ce jeu déclenche. Actuellement local à
`drag.rs` + `systems.rs`.

**Rule of Three** : 1re occurrence. **Local pour la V1.**

Si un 2e jeu (Mahjong, Solitaire, Card Battler) a besoin de drag-and-drop
générique, on extraira :
- Une machine à états `DragState` générique.
- Un trait `Draggable` / `Droppable`.
- Un hit testing sur layout rectangulaire.

### Hit testing sur layout rectangulaire

Le pattern `zone_at(mouse) -> Option<Zone>` + `hit_test(mouse) -> Option<(Zone, usize)>`
est générique. Actuellement dans `systems.rs`.

**2e occurrence** (Minesweeper clic + FreeCell drag). **Surveiller** :
si un 3e jeu fait du hit testing rectangulaire, extraire un helper
`hit_test_rects(rects: &[Rect], mouse) -> Option<usize>`.

### Layout de cartes superposées

Les colonnes contiennent des cartes qui se chevauchent (offset vertical
entre les cartes). Ce n'est pas du `Grid<T>` (qui suppose des cases
régulières non chevauchantes). Layout custom dans `layout.rs`.

**1re occurrence.** Local pour V1.

### Snapshots undo/redo

`GameSnapshot` + piles `history` / `future`. Générique et peu coûteux
(52 cartes + 4 options).

**1re occurrence.** Local. À surveiller si un 2e jeu a besoin de undo
(Card Battler, Puzzle).

## Structure du projet

```
games/freecell/
├── Cargo.toml
├── .env                       # couleurs terminal, layout
├── DESIGN.md                  # ce document
├── best_times.ron             # créé au runtime, gitignored
├── assets/
│   └── DejaVuSansMono.ttf     # police pour symboles Unicode ♠ ♥ ♦ ♣
├── src/
│   ├── main.rs                # boucle, input, rendu
│   ├── lib.rs
│   ├── config.rs              # GameContext
│   ├── components.rs          # Card, Suit, Rank, Color, Zone
│   ├── deck.rs                # création, distribution, seed
│   ├── drag.rs                # DragState, machine à états
│   ├── font.rs                # chargement DejaVu Sans Mono
│   ├── layout.rs              # positions des colonnes/cellules/fondations
│   ├── persistence.rs         # façade BestTimes
│   └── systems.rs             # Game struct, moves, rules, undo/redo, hint
└── tests/
    └── game_scenarios.rs      # tests d'intégration
```

## Machine à états

```
Start ──(Espace/Entrée)──▶ Playing ──(toutes cartes en fondation)──▶ Win
                              │
                              ├──(Tab)──▶ Start
                              └──(R)────▶ Playing (reset, même seed)
```

Dans Playing, un **sous-état** gère le drag :

- `Idle` : rien en cours, clics acceptés.
- `Pressing { origin, card, press_pos }` : souris enfoncée sur une carte.
- `Dragging { origin, start_index, cards, offset }` : souris bouge, cartes suivent.

Ce sous-état est `DragState` (dans `drag.rs`), intégré dans `Game.drag`.

## API

### Card model

```rust
pub enum Suit { Spade, Heart, Diamond, Club }
pub enum Color { Red, Black }        // dérivé de Suit
pub struct Rank(pub u8);             // 1 = As, 11 = Valet, 12 = Dame, 13 = Roi

pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self;
    pub fn color(&self) -> Color;
    pub fn symbol(&self) -> char;       // '♠', '♥', '♦', '♣'
    pub fn ascii(&self) -> char;        // 'S', 'H', 'D', 'C' (fallback)
    pub fn rank_label(&self) -> char;   // 'A', '2', ..., 'T', 'J', 'Q', 'K'
}
```

Note : `Rank::label()` retourne `'T'` pour 10 (notation bridge), et
`'J'` / `'Q'` / `'K'` pour Valet / Dame / Roi (convention anglaise).

### Zones

```rust
pub enum Zone {
    Column(usize),      // 0..8
    FreeCell(usize),    // 0..4
    Foundation(usize),  // 0..4
}
```

### Drag

```rust
pub enum DragState {
    Idle,
    Pressing { origin: Zone, card: Card, press_pos: Vec2 },
    Dragging { origin: Zone, start_index: usize, cards: Vec<Card>, offset: Vec2 },
}
```

### Game

```rust
pub struct Game {
    pub state: GameState,
    pub columns: [Vec<Card>; 8],
    pub free_cells: [Option<Card>; 4],
    pub foundations: [Vec<Card>; 4],
    pub drag: DragState,
    pub moves: u32,
    pub elapsed: f32,
    pub timer_running: bool,
    pub seed: u32,
    pub history: Vec<GameSnapshot>,   // undo
    pub future: Vec<GameSnapshot>,    // redo
    pub best_times: HashMap<String, f32>,
    pub best_handle: BestTimes,
    pub was_new_best: bool,
    pub hint: Option<Hint>,
    // widgets...
}

impl Game {
    pub fn new() -> Self;
    pub fn with_best_handle(handle: BestTimes) -> Self;
    pub fn start_game(&mut self, seed: u32, ctx: &GameContext);
    pub fn try_drop(&mut self, from: Zone, to: Zone, cards: &[Card], start_idx: usize) -> bool;
    pub fn can_drop_at(&self, target: Zone) -> bool;
    pub fn can_place_on_column(&self, card: Card, col: usize) -> bool;
    pub fn can_place_on_foundation(&self, card: Card, foundation: usize) -> bool;
    pub fn can_place_in_cell(&self, cell: usize) -> bool;
    pub fn is_accessible(&self, from: Zone, start_idx: usize, n: usize) -> bool;
    pub fn max_super_move(&self) -> usize;
    pub fn find_hint(&self) -> Option<Hint>;
    pub fn undo(&mut self) -> bool;
    pub fn redo(&mut self) -> bool;
    pub fn tick(&mut self, dt: f32);
    pub fn zone_at(&self, mouse: Vec2, ctx: &GameContext) -> Option<Zone>;
    pub fn hit_test(&self, mouse: Vec2, ctx: &GameContext) -> Option<(Zone, usize)>;
    pub fn drag_cards(&self, zone: Zone, idx: usize) -> Vec<Card>;
    pub fn start_drag(&mut self, mouse: Vec2, ctx: &GameContext);
    pub fn update_drag(&mut self, mouse: Vec2, ctx: &GameContext);
    pub fn end_drag(&mut self, mouse: Vec2, ctx: &GameContext);
    pub fn cancel_drag(&mut self);
}
```

## Layout

Fenêtre : 1000×700.

- **Header** : 50 px (seed, moves, timer, boutons Hint/Restart/New/Menu)
- **Haut du plateau** :
  - 4 cellules libres à gauche
  - 4 fondations à droite (slots 4-7 alignés sur les colonnes du bas)
  - Gap central
- **Tableau** : 8 colonnes, cartes superposées
  - Offset vertical entre cartes : 30 px (assez pour voir le rang/couleur)
  - Cartes de 80×110 px
- **Footer** : 30 px (raccourcis)

**Centrage** : le plateau complet (8 colonnes + gaps) est centré
horizontalement via `board_x_offset`. Les cellules et fondations
partagent la même grille horizontale.

## Persistance

`best_times.ron` : `HashMap<String, f32>` (clé = seed stringifiée,
valeur = best time en secondes).

Format RON, même que Minesweeper / Memory. Réutilise
`ember_stdlib::persistence::Persistence<T>`.

`Persistence::in_manifest_dir(env!("CARGO_MANIFEST_DIR"), "best_times.ron")`
— attention à passer le `manifest_dir` du jeu, pas celui de la stdlib.

## Tests

### Unitaires (81)

Répartis dans `components.rs`, `deck.rs`, `drag.rs`, `layout.rs`,
`persistence.rs`, `systems.rs`.

Couverture :
- Deck : 52 cartes, 4 de chaque rang, 13 de chaque couleur, pas de doublons, déterministe.
- Distribution : 4×7 + 4×6 = 52, même seed → même distribution, seeds différentes → distributions différentes.
- Règles : rouge sur noir, pas même couleur, rang immédiatement supérieur.
- Fondations : As sur vide, ascension stricte même couleur, refus mauvaise couleur.
- Cellules : accepte si vide, refuse si pleine.
- Super-move : formule, accessibilité, run contigu.
- Drag : transitions Idle/Pressing/Dragging, seuil 5 px.
- Hit testing : carte du haut, carte du bas, hors zone → None.
- Undo/redo : restauration, redo vidée par nouveau move.
- Hint : retourne un move sans muter, None si aucune.

### Intégration (8)

Dans `tests/game_scenarios.rs` :
- `test_full_easy_solve_known_sequence` : résolution complète, game state Win.
- `test_super_move_sequence_to_empty_column` : 3 cartes déplacées d'un coup.
- `test_best_time_persisted_on_win` : persistance sur disque vérifiée.
- `test_undo_redo_roundtrip` : move → undo → redo → état cohérent.
- `test_dealt_board_contains_all_52_unique_cards` : permutation des 52.
- `test_win_after_all_foundations_filled` : win détectée.
- `test_hint_is_only_an_indicator` : hint ne mute pas l'état.
- `test_hint_returns_none_when_no_move_exists` : plateau résolu → None.

**Total : 89 tests** (81 unit + 8 integration), Clippy clean.

## Pièges anticipés → rencontrés en vrai

### Unicode (anticipé, confirmé)

La police par défaut de macroquad (`ProggyClean.ttf`) ne couvre que
l'ASCII de base. Les symboles ♠ ♥ ♦ ♣ (U+2660..U+2667) rendent en tofu
(□). **Solution** : charger `DejaVuSansMono.ttf` depuis `assets/` via
`load_ttf_font`, puis `set_default_font`. Fallback ASCII (`S H D C`)
documenté si la police ne charge pas.

### Clavier AZERTY (non anticipé, découvert en dev)

**macroquad utilise les scancodes QWERTY**, pas les caractères du
layout. Sur un clavier AZERTY :

- `M` physique → `KeyCode::Semicolon`
- `Z` physique → `KeyCode::W`
- `W` physique → `KeyCode::Z`
- `A` ↔ `Q` swappés

**Symptôme** : `is_key_pressed(KeyCode::M)` retourne toujours `false`
même en tapant `M`. `get_last_key_pressed()` confirme `Semicolon`.

**Solution** :
- Menu : `Tab` (position identique sur tous les layouts).
- Undo : `Z` ou `W` (double mapping).
- Redo : `Y` (position identique).
- Toujours préférer `Tab`, `Escape`, `Space`, `Enter`, `F1..F12`.

### Fondations noires invisibles (non anticipé)

Sur fond sombre, un symbole noir à alpha 0.20 est invisible. **Solution** :
couleur claire (gris `[0.55, 0.60, 0.68]`) + alpha ≥ 0.45 pour les
placeholders des fondations.

### Symbole de fondation mal centré (non anticipé)

`draw_text` prend une **baseline**, pas un top. Calcul naïf
`y = rect.center().y - dims.height * 0.5` ne centre pas. **Solution** :
`y = rect.y + rect.h * 0.5 + dims.height * 0.35` (approximation optique).

### Timer chevauchant le bouton Hint (non anticipé)

Timer positionné en dur à `window_w - 380.0 - time_dims.width` chevauche
le bouton Hint (à `window_w - 480.0`). **Solution** : calculer
`time_x = hint_btn.rect.0 - time_dims.width - 20.0`. Dessiner le timer
**avant** les boutons pour qu'il soit recouvert proprement si collision.

### `max_super_move` et destination (latent)

La formule officielle exclut la **colonne de destination si elle est
vide** (elle est occupée par le move lui-même). L'implémentation actuelle
compte toutes les colonnes vides, y compris la destination.

**Conséquence** : sur-compte dans un cas précis, autorise un move
invalide. **Fix V2** : `max_super_move_to(dest: Option<Zone>)`.

**Bug visible** : le joueur peut être surpris qu'un move de 3 cartes
échoue avec 0 cellule libre + 1 colonne vide, alors qu'il réussit avec
0 cellule libre + 2 colonnes vides. Le HUD n'affiche pas la limite, donc
le joueur ne comprend pas. **Amélioration V2** : afficher `Max: X` dans
le header.

### Super-move `T → Q` refusé (comportement correct)

Le joueur peut s'attendre à pouvoir sauter des rangs sur la fondation.
Non : c'est strictement `+1`, même couleur. `T → V → D → R`. C'est la
règle officielle.

### Dernière carte de colonne (mineur)

Avec `card_stack_offset = 30`, les longues colonnes (7 cartes) occupent
`6 × 30 + 110 = 290 px`. Tient dans le playfield. Ne pas dépasser 45 px
sinon débordement du footer.

### Hit testing sur cartes superposées

`hit_test` doit trouver la **bonne** carte (la plus profonde au point
cliqué). Parcours en ordre décroissant (`(0..len).rev()`) pour trouver
la carte qui recouvre les autres. Actuellement correct.

## Ce que FreeCell ne fait PAS (et pourquoi)

- **Pas de pioche** : FreeCell a toutes les cartes visibles.
- **Pas de score numérique** : temps + moves.
- **Pas d'IA/solveur** : V3.
- **Pas de multi-joueurs** : V2/V3.
- **Pas de tween/easing** : à faire dans Match-3.
- **Pas de caméra/scroll** : fenêtre fixe.
- **Pas d'auto-complete** : V2.
- **Pas d'input de seed** : V2 (bouton « New » random uniquement).
- **Pas de sélection clic-clic** : V2 (drag only).

## Roadmap

- **Session 12** : Setup (Cargo.toml, config, components, deck, layout) + test Unicode visuel. ✅
- **Session 13** : Règles (can_place, try_drop, super_move, undo/redo) + tests unitaires. ✅
- **Session 14** : Drag-and-drop (machine à états + hit testing + rendu). ✅
- **Session 15** : Rendu complet (colonnes, cellules, fondations, HUD, menu) + tests d'intégration. ✅
- **Session 16** : Polish + correction AZERTY + fondations + doc-sync. ✅

## Roadmap V2 (post-V1)

- **Auto-complete** : détecter les positions gagnantes et finir en un clic.
- **Input de seed** : champ texte pour rejouer une partie précise.
- **Sélection clic-clic** : alternative au drag pour accessibilité.
- **HUD `Max: X`** : afficher la limite de super-move en temps réel.
- **Fix `max_super_move_to(dest)`** : prendre la destination en compte.
- **Test `test_super_move_9_cards_with_one_empty_column`** : régression.

## Roadmap V3

- **Solveur A*** : trouver une solution automatique depuis n'importe quel état.
- **Score avancé** : statistiques par seed (best moves, best time).

## Décisions prises

1. **Super-move en V1** : oui, complet.
2. **Undo/Redo en V1** : oui, sans modificateur (`Z`/`W`/`Y`).
3. **Auto-complete en V1** : non (V2).
4. **Seed UI** : bouton « New » (seed random) en V1, input en V2.
5. **Drag vs sélection** : drag only en V1, sélection en V2.
6. **Unicode vs ASCII** : Unicode activé (DejaVu Sans Mono chargée).
7. **Raccourci menu** : `Tab` (contrainte AZERTY).
8. **Timer pendant drag** : continue (cohérent Memory).
9. **Redo vidée par nouveau move** : oui (comportement standard).
10. **Undo stocke snapshot avant move** : oui.
11. **Best time persisté par seed** : oui.
12. **Drag-and-drop local** : oui (Rule of Three, 1re occurrence).