# Memory — Design Document

## Concept

Un jeu de memory (jeu de paires) sur le thème du **terminal du développeur**.
Chaque carte cachée affiche un symbole ASCII / ponctuation au dos
(`$`, `§`, `>`, `{`, `}`, `[`, `]`, …). Le joueur retourne deux cartes ;
si les symboles correspondent, la paire reste visible. Sinon, les cartes
se retournent après un court délai.

Objectif : trouver toutes les paires en un minimum de temps.

## Thème

**Terminal du dev**. Les symboles sont des caractères que tout dev tape
tous les jours. Pas d'émojis (problèmes de rendu macroquad), pas d'images.

Le fond et les couleurs évoquent un terminal : fond sombre, texte clair
(vert/blanc/cyan), bordures façon ASCII. La carte cachée affiche un
`?` ou un caractère neutre. La carte révélée affiche le symbole.

## Mécaniques

### Boucle de jeu

1. Le plateau est une grille de cartes face cachée.
2. Le joueur clique une première carte → elle se retourne.
3. Le joueur clique une seconde carte → elle se retourne aussi.
4. **Match** : les deux symboles correspondent → les deux cartes restent
   face visible (état `Matched`). Le compteur `pairs_found` augmente.
5. **No match** : les symboles diffèrent → après un délai (`NO_MATCH_DELAY`,
   ~800 ms), les deux cartes se retournent (état `Hidden`).
6. Tant que le délai court, **les clics sont ignorés** (le joueur ne peut
   pas retourner une 3e carte).
7. La partie est gagnée quand toutes les paires sont trouvées.

### États d'une carte

- `Hidden` : face cachée, cliquable.
- `Flipped` : face visible, **transition en cours** (premier clic ou
  dans le délai de no-match). Non cliquable.
- `Matched` : face visible, définitive. Non cliquable.

### Difficultés

- **Easy** : 4x4 (16 cartes, 8 paires)
- **Medium** : 6x6 (36 cartes, 18 paires)
- **Hard** : 8x8 (64 cartes, 32 paires)

Chaque difficulté a un **best time** persisté.

### Timer

Le timer démarre au **premier clic** (comme Minesweeper) et s'arrête à
la victoire. Pas de game over (on peut toujours finir une partie de
memory, même en cliquant au hasard).

### Score

Pas de score numérique. Le **temps** est l'unique mesure de performance.
Best time par difficulté.

## Réutilisation des briques moteur

| Brique | Provenance | Usage |
|---|---|---|
| `Grid<T>` | ember-stdlib | Plateau de cartes |
| `Input` | ember-stdlib | Clic souris |
| `Button`, `Label`, `Panel`, `TimerDisplay` | ember-stdlib/ui | Menu, HUD, écrans |
| `Persistence<HashMap<String, f32>>` | ember-stdlib | Best times par difficulté |
| `GameContext` | ember-core | Config `.env` |
| `GameState` | ember-core | Start / Playing / Win |
| `Rng` | ember-core | Mélange du deck |

**Aucune brique nouvelle n'est strictement nécessaire** pour la V1.
C'est un jeu qui **réutilise** massivement ce qu'on a construit.

## Nouvelles briques potentielles (pour plus tard)

Ces briques **ne sont pas** dans le scope de la V1, mais le jeu les
prépare naturellement :

- **Sélection** : cliquer un élément, le mettre en surbrillance, vérifier
  une combinaison. Utile pour Tower Defense, Card Battler, Match-3.
- **Délai transitoire** : un timer éphémère qui bloque les inputs pendant
  une action (le no-match delay). On l'implémentera localement pour la V1.
- **Retournement visuel** : animation simple (couleur, taille, ou flip).
  Pour la V1, on fera un changement de couleur instantané. Les vraies
  animations (tween/easing) viendront dans un jeu ultérieur (Match-3).
- **Drag-and-drop** : à faire dans un jeu type FreeCell / Mahjong.
  **Pas dans Memory.**

## Structure du projet

games/memory/
├── Cargo.toml
├── .env                       # couleurs terminal, tailles de cartes
├── DESIGN.md                  # ce document
├── best_times.ron             # créé au runtime, gitignored
├── src/
│   ├── main.rs                # boucle, input, rendu
│   ├── lib.rs
│   ├── config.rs              # GameContext
│   ├── components.rs          # Card, CardState, Symbol, DifficultyData
│   ├── difficulties.rs        # chargement difficulties.ron
│   ├── persistence.rs         # façade BestTimes (comme Minesweeper)
│   ├── systems.rs             # Game struct, reveal, match, tick
│   └── ...
└── tests/
    └── game_scenarios.rs      # tests d'intégration

## Machine à états

Start ──(Space/Click)──▶ Playing ──(toutes paires trouvées)──▶ Win
                            │
                            └──(R)──▶ Start

Dans `Playing`, un **sous-état** gère la phase de sélection :

- `Idle` : aucune carte sélectionnée, clic autorisé.
- `FirstPick { card }` : une carte retournée, en attente de la seconde.
- `SecondPick { first, second }` : deux cartes retournées, en attente de
  la résolution (match ou no-match).
- `Resolving { cards, until }` : délai de no-match en cours, clics bloqués.

Ce sous-état peut être un `enum SelectionPhase` dans `Game`.

## API pressentie

// Card
pub struct Card {
    pub symbol: Symbol,
    pub state: CardState,
}

pub enum CardState { Hidden, Flipped, Matched }

pub struct Symbol(pub char);   // '$', '§', '>', '{', ...

// Game
pub struct Game {
    pub state: GameState,
    pub board: Grid<Card>,
    pub selection: SelectionPhase,
    pub pairs_found: usize,
    pub total_pairs: usize,
    pub elapsed: f32,
    pub timer_running: bool,
    pub difficulty: usize,
    pub best_times: HashMap<String, f32>,
    pub best_handle: BestTimes,
    pub rng: Rng,
    // widgets...
}

impl Game {
    pub fn new() -> Self;
    pub fn start_game(&mut self);
    pub fn cell_at(&self, mouse: Vec2) -> Option<(usize, usize)>;
    pub fn reveal(&mut self, col: usize, row: usize);
    pub fn tick(&mut self, dt: f32);
    // ...
}

## Layout

Le plateau est centré dans la fenêtre. Chaque carte est un rectangle
de taille fixe (paramétrable via `.env`). Un padding entre cartes.
Le HUD affiche : difficulté, temps, paires trouvées / total.

Menu : liste des 3 difficultés (comme Minesweeper), best time par
difficulté, bouton START.

## Persistance

`best_times.ron` : `HashMap<String, f32>` (nom de la difficulté → best
time en secondes). Même format que Minesweeper. Réutilise
`ember_stdlib::persistence::Persistence`.

## Tests prévus

**Unitaires** (`systems.rs`) :
- `test_card_starts_hidden`
- `test_deck_has_two_of_each_symbol`
- `test_deck_is_shuffled` (déterministe avec seed)
- `test_reveal_flips_hidden_card`
- `test_reveal_ignored_on_matched`
- `test_match_marks_both_matched`
- `test_no_match_starts_resolving_phase`
- `test_resolving_blocks_clicks`
- `test_resolving_ends_after_delay`
- `test_win_when_all_matched`
- `test_timer_starts_on_first_reveal`
- `test_timer_stops_on_win`

**Intégration** (`tests/game_scenarios.rs`) :
- `test_full_easy_playthrough`
- `test_no_match_cycles_back_to_hidden`
- `test_best_time_persisted_on_win`
- `test_medium_and_hard_deck_sizes`

## Pièges anticipés

- **Timer pendant le no-match delay** : le timer de jeu doit continuer
  à tourner pendant le délai de no-match (le joueur "perd" du temps).
- **Mélange** : le deck doit être **mélangé** au démarrage, mais de façon
  déterministe (seed) pour les tests.
- **Grille non-carée** : si on ajoute une difficulté 5x4 (20 cartes, 10
  paires), le code doit gérer les rectangles. On restera sur des grilles
  carées pour la V1.
- **Widgets dans le state** : `start_btn`, `restart_btn`, `menu_btn` dans
  `Game` (comme Minesweeper), update + draw sur la même instance.
- **Click pendant Resolving** : ignoré explicitement, sinon on peut
  retourner une 3e carte.

## Roadmap

- **Session 7** : Setup (Cargo.toml, config, components, difficultés) + squelette `Game`.
- **Session 8** : Logique de jeu (reveal, match, no-match, win) + tests unitaires.
- **Session 9** : Rendu (cartes, HUD, menu) + tests d'intégration + best times.
- **Session 10** : Polish visuel (thème terminal, couleurs, feedback) + doc-sync.

## Ce que Memory ne fait PAS (et pourquoi)

- **Pas de drag-and-drop** : à faire dans un jeu type FreeCell / Mahjong.
- **Pas d'animations** (tween/easing) : à faire dans Match-3.
- **Pas de son** : aucun symbole ne "sonne" naturellement. À voir si on
  veut un petit "click" à chaque flip (optionnel, à décider plus tard).
- **Pas de score numérique** : le temps est la seule métrique.
- **Pas de multi-joueurs** : V2/V3 si envie.