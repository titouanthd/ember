# FreeCell — Design Document

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

**Terminal dev**. Fond sombre, symboles de cartes en Unicode (si la
police par défaut les rend — sinon fallback ASCII S H D C).

- Fond de fenêtre : gris très sombre
- Cartes : fond clair, bordure discrète
- Rouges (♥ ♦) : rouge vif
- Noirs (♠ ♣) : blanc cassé ou noir profond
- Fondations vides : placeholder subtil (un carré, un A fantôme)
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

- Colonnes 1-4 : 7 cartes chacune
- Colonnes 5-8 : 6 cartes chacune

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

Exemple : ♥ A → ♥ 2 → ♥ 3 → ...

### Déplacements de piles (super-move)

**Règle clé de FreeCell** : on peut déplacer une **séquence valide**
(descendante, couleurs alternées) d'un coup, dans la limite des cellules
libres et des colonnes vides.

**Formule** : (1 + cellules_libres) * 2^colonnes_vides.

Exemples :
- 4 cellules libres, 0 colonnes vides : 5 cartes déplaçables.
- 4 cellules libres, 1 colonne vide : 10 cartes.
- 2 cellules libres, 0 colonnes vides : 3 cartes.

**Décision V1** : on implémente le **déplacement de pile valide**
jusqu'à la limite calculée. C'est essentiel pour FreeCell.

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
- Idle : rien en cours.
- Pressing { origin, card } : souris enfoncée sur une carte (pas
  encore draggée). Utile pour distinguer un clic d'un drag.
- Dragging { origin, cards, offset } : la souris bouge, on affiche
  les cartes « collées » au curseur.
- Dropping { origin, cards, target } : relâchement de la souris,
  tentative de dépôt.

**Règles du drag** :
- On peut drag :
  - La carte du haut d'une colonne.
  - Une séquence valide à partir d'une carte (si elle est valide).
  - Une carte d'une cellule libre.
  - **Pas** une carte d'une fondation (une fois placée, elle est fixe).
- Pendant le drag, la carte (ou les cartes) suit la souris.
- Au drop, on vérifie la cible et on applique le déplacement si valide.
- Sinon, on annule (les cartes reviennent à leur position).

### Sélection alternative (V2)

Pour les joueurs qui préfèrent cliquer plutôt que drag : cliquer une
carte pour la sélectionner, puis cliquer une destination. **V2.**

### Auto-complete (V2)

Quand toutes les cartes sont dans un état qui garantit une victoire
(toutes les colonnes triées, plus aucune contrainte), on peut finir
automatiquement. **V2.**

### Annuler / Refaire (V1)

Ctrl+Z pour annuler un coup. Ctrl+Y (ou Shift+Ctrl+Z) pour refaire.

Stocke un historique des déplacements. Chaque entrée contient l'état
complet **avant** le move : les 8 colonnes, les 4 cellules, les 4
fondations. Comme l'état est petit (~52 cartes + 4 options), un snapshot
est peu coûteux.

**Décision V1** : on implémente l'undo/redo dès la V1. C'est une feature
attendue dans un jeu de cartes, et un snapshot complet est trivial à
stocker pour FreeCell.

### Hint / solve (V3)

Un solveur automatique de FreeCell existe (algorithme A*). **V3.**

### Score

**Pas de score numérique.** FreeCell se mesure en :
- Nombre de coups (moves)
- Temps (best time par partie si on ajoute une seed)

**Décision V1** : compter les **moves** et le **temps**. Persister le
**best time** sur une seed fixe.

### Seed et reproductibilité

**FreeCell a une feature classique** : les parties sont numérotées. La
partie #1, #2, #3 ont des distributions de cartes **connues et
reproductibles**. Microsoft FreeCell a rendu cette feature célèbre.

**Décision V1** : implémenter la génération de parties numérotées.
- Le joueur peut générer une nouvelle partie aléatoire (bouton / touche N).
- L'input d'une seed spécifique est repoussé en V2.
- La distribution est déterministe à partir de la seed.
- Le **best time** est persisté par numéro de partie.

**Note** : la distribution « classique » de Microsoft FreeCell utilise
un algorithme spécifique (LCG + shuffle). On peut utiliser notre Rng
à la place, tant que c'est **déterministe**.

## Réutilisation des briques moteur

| Brique | Provenance | Usage |
|---|---|---|
| Button, Label, Panel, TimerDisplay | ember-stdlib/ui | HUD, menu, écrans |
| Input | ember-stdlib | Souris (drag-and-drop) |
| Persistence<HashMap<String, f32>> | ember-stdlib | Best time par seed |
| GameContext | ember-core | Config .env |
| GameState | ember-core | Start / Playing / Win |
| Rng | ember-core | Mélange déterministe |

**Aucune brique nouvelle n'est strictement nécessaire**, mais ce jeu
**déclenche** le drag-and-drop (voir ci-dessous).

## Nouvelles briques potentielles (à extraire ?)

### Drag-and-drop

C'est **la** brique que ce jeu déclenche. Actuellement, Input fournit
les événements souris bruts (mouse_pos, mouse_left_pressed/down/released).
Le drag-and-drop demande :

- **Machine à états** : Idle → Pressing → Dragging → Dropping.
- **Hit testing** : convertir la position souris en zone/carte.
- **Overlay de drag** : dessiner les cartes au-dessus du reste.

**Rule of Three** : c'est la **1re occurrence**. Donc **local au jeu**
pour la V1. À extraire dans ember_stdlib si un 2e jeu en a besoin
(Mahjong, Tower Defense, Card Battler...).

**MAIS** : un pattern **générique** de « hit testing sur un layout
rectangulaire » pourrait être extrait. À voir.

### Layout de cartes superposées

Les colonnes contiennent des cartes qui se chevauchent (offset vertical
entre les cartes). Ce n'est pas du Grid<T> (qui suppose des cases
régulières non chevauchantes). Il faut un layout custom.

**V1** : layout local au jeu. Si un 2e jeu fait pareil (Mahjong,
Solitaire), on extraira.

## Structure du projet

games/freecell/
├── Cargo.toml
├── .env                       # couleurs terminal
├── DESIGN.md                  # ce document
├── src/
│   ├── main.rs                # boucle, input, rendu
│   ├── lib.rs
│   ├── config.rs              # GameContext
│   ├── components.rs          # Card, Suit, Rank, Color, Zone
│   ├── deck.rs                # création, distribution, seed
│   ├── layout.rs              # positions des colonnes/cellules/fondations
│   ├── drag.rs                # DragState, machine à états
│   ├── systems.rs             # Game struct, moves, rules, undo/redo
│   └── ...
└── tests/
    └── game_scenarios.rs      # tests d'intégration

## Machine à états

Start ──(Space/Click)──▶ Playing ──(toutes cartes en fondation)──▶ Win
                            │
                            └──(R)──▶ Start

Dans Playing, un **sous-état** gère le drag :

- Idle : rien en cours, clics acceptés.
- Pressing { origin, card } : souris enfoncée sur une carte.
- Dragging { origin, cards, offset } : souris bouge, carte suit.
- Dropping { origin, cards, target } : relâchement, tentative.

Ce sous-état est un enum DragState dans Game.

## API pressentie

Card model:

    pub enum Suit { Spade, Heart, Diamond, Club }
    pub enum Color { Red, Black }   // dérivé de Suit
    pub struct Rank(pub u8);        // 1 = Ace, 11 = Jack, 12 = Queen, 13 = King

    pub struct Card {
        pub suit: Suit,
        pub rank: Rank,
    }

    impl Card {
        pub fn color(&self) -> Color;
        pub fn symbol(&self) -> char;      // '♠', '♥', '♦', '♣' (ou 'S', 'H', ...)
        pub fn rank_label(&self) -> char;  // 'A', '2', ..., 'K'
    }

Zones:

    pub enum Zone {
        Column(usize),      // 0..8
        FreeCell(usize),    // 0..4
        Foundation(usize),  // 0..4
    }

Drag:

    pub enum DragState {
        Idle,
        Pressing { origin: Zone, card: Card },
        Dragging { origin: Zone, cards: Vec<Card>, offset: Vec2 },
    }

Game:

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
        pub best_times: HashMap<String, f32>,  // seed → best time
        pub best_handle: BestTimes,
        // widgets...
    }

    impl Game {
        pub fn new() -> Self;
        pub fn start_game(&mut self, seed: u32, ctx: &GameContext);
        pub fn try_drop(&mut self, from: Zone, to: Zone, cards: &[Card]) -> bool;
        pub fn can_place_on_column(&self, card: Card, col: usize) -> bool;
        pub fn can_place_on_foundation(&self, card: Card, foundation: usize) -> bool;
        pub fn can_place_in_cell(&self, cell: usize) -> bool;
        pub fn max_super_move(&self) -> usize;
        pub fn undo(&mut self) -> bool;
        pub fn redo(&mut self) -> bool;
        pub fn check_win(&mut self);
        // ...
    }

## Layout

Fenêtre : 900×700 (cohérent avec Memory).

- **Header** : 50 px (seed, moves, timer, Restart, Menu)
- **Haut du plateau** :
  - 4 cellules libres à gauche
  - 4 fondations à droite
  - Gap central
- **Tableau** : 8 colonnes, cartes superposées
  - Offset vertical entre cartes : ~30 px (assez pour voir le rang/couleur)
  - Cartes de 80×110 px
- **Footer** : 30 px (raccourcis)

## Persistance

best_times.ron : HashMap<String, f32> (clé = seed, valeur = best time).
Même format que Minesweeper / Memory. Réutilise
ember_stdlib::persistence::Persistence.

## Tests prévus

Unitaires (systems.rs, deck.rs) :
- test_deck_has_52_cards
- test_deck_has_4_of_each_rank
- test_deck_has_13_of_each_suit
- test_distribution_column_sizes (4x7 + 4x6)
- test_distribution_deterministic (même seed → même distribution)
- test_different_seeds_differ
- test_can_place_red_on_black_and_vice_versa
- test_cannot_place_same_color
- test_cannot_place_wrong_rank
- test_ace_goes_to_empty_foundation
- test_ace_goes_to_correct_color_foundation
- test_foundation_accepts_ascending_same_suit
- test_foundation_rejects_wrong_suit
- test_cell_accepts_any_card_when_empty
- test_cell_rejects_when_full
- test_max_super_move_calculation (formule)
- test_undo_restores_previous_state
- test_redo_after_undo
- test_undo_then_new_move_clears_redo
- test_win_when_all_foundations_complete

Intégration (tests/game_scenarios.rs) :
- test_full_easy_solve (seed connue, résolu en N coups)
- test_super_move_moves_sequence
- test_best_time_persisted_by_seed
- test_drag_state_transitions

## Pièges anticipés

- **Unicode rendering** : ♠ ♥ ♦ ♣ doivent être rendus par la police
  par défaut de macroquad. Si non, fallback ASCII (S H D C).
  **Tester avant de coder le rendu final.**
- **Cartes superposées** : le hit test doit trouver la **bonne** carte
  (la plus haute au point cliqué), pas la première dans l'ordre du Vec.
- **Super-move** : la formule (1 + cells) * 2^empty_columns doit être
  exacte, sinon les déplacements de piles seront impossibles à valider.
- **Drag et timer** : le timer continue pendant le drag (le joueur
  « perd » du temps). Cohérent avec Memory.
- **Undo** : stocker un snapshot **avant** le move, pas après. Sinon on
  ne peut pas annuler le premier move.
- **Redo effacé par un nouveau move** : si on undo puis qu'on joue un
  nouveau coup, la pile redo est vidée (comportement standard).
- **Seed "connues"** : la seed #1 de Microsoft FreeCell est célèbre.
  On n'essaie pas de reproduire leur algo exact — on utilise notre
  Rng avec un offset pour que la seed #1 soit résoluble (à vérifier
  sur quelques seeds).

## Roadmap

- **Session 12** : Setup (Cargo.toml, config, components, deck, layout) + test Unicode visuel.
- **Session 13** : Règles (can_place, try_drop, super_move, undo/redo) + tests unitaires.
- **Session 14** : Drag-and-drop (machine à états + hit testing + rendu).
- **Session 15** : Rendu complet (colonnes, cellules, fondations, HUD, menu) + tests d'intégration.
- **Session 16** : Polish (auto-complete ?) + doc-sync.

## Ce que FreeCell ne fait PAS (et pourquoi)

- **Pas de pioche** : FreeCell a toutes les cartes visibles.
- **Pas de score numérique** : temps + moves.
- **Pas d'IA/solveur** : V3.
- **Pas de multi-joueurs** : V2/V3.
- **Pas de tween/easing** : à faire dans Match-3.
- **Pas de caméra/scroll** : fenêtre fixe.

## Décisions prises

1. **Super-move en V1** : complet.
2. **Undo/Redo en V1** : oui.
3. **Auto-complete en V1** : non (V2).
4. **Seed UI** : bouton "New Game" (seed random) en V1, input de seed en V2.
5. **Drag vs sélection** : drag only en V1.
6. **Unicode vs ASCII** : test en Session 12, décision visuelle.