// games/snake/src/systems.rs
use crate::components::{Cell, Direction, Food, Snake, Wall};
use ember_core::app::GameState;
use ember_core::time::TickTimer;
use macroquad::prelude::Color;
use serde::Deserialize;
use std::path::PathBuf;

/// Format RON d'un niveau Snake.
#[derive(Debug, Clone, Deserialize)]
pub struct SnakeLevel {
    pub name: String,
    pub spawn: (i32, i32),
    pub spawn_direction: String,
    pub walls: Vec<(i32, i32)>,
    pub initial_length: u32,
    pub target_score: u32,
}

/// Charge un niveau depuis `levels/levelN.ron`.
pub fn load_level(index: usize) -> Result<SnakeLevel, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join(format!("levels/level{}.ron", index + 1));
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Lecture {:?} échouée : {}", path, e))?;
    ron::de::from_str(&content)
        .map_err(|e| format!("Désérialisation {:?} échouée : {}", path, e))
}

/// Convertit une direction textuelle en `Direction`.
fn parse_direction(s: &str) -> Direction {
    match s {
        "Up" => Direction::Up,
        "Down" => Direction::Down,
        "Left" => Direction::Left,
        "Right" => Direction::Right,
        other => {
            eprintln!("⚠️ Direction inconnue '{}', fallback Right", other);
            Direction::Right
        }
    }
}

pub struct GameContext {
    pub grid_cols: u32,
    pub grid_rows: u32,
    pub cell_size: f32,

    pub initial_tick_ms: u32,
    pub min_tick_ms: u32,
    pub tick_speedup_ms: u32,
    pub food_per_speedup: u32,

    pub bg_color: Color,
    pub grid_color: Color,
    pub snake_head_color: Color,
    pub snake_body_color: Color,
    pub food_color: Color,
    pub wall_color: Color,
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            grid_cols: 20,
            grid_rows: 15,
            cell_size: 30.0,
            initial_tick_ms: 180,
            min_tick_ms: 60,
            tick_speedup_ms: 8,
            food_per_speedup: 5,
            bg_color: Color::new(0.05, 0.05, 0.08, 1.0),
            grid_color: Color::new(0.12, 0.12, 0.16, 1.0),
            snake_head_color: Color::new(0.3, 0.95, 0.4, 1.0),
            snake_body_color: Color::new(0.15, 0.65, 0.25, 1.0),
            food_color: Color::new(1.0, 0.35, 0.35, 1.0),
            wall_color: Color::new(0.5, 0.5, 0.55, 1.0),
        }
    }
}

impl GameContext {
    pub fn screen_w(&self) -> f32 { self.grid_cols as f32 * self.cell_size }
    pub fn screen_h(&self) -> f32 { self.grid_rows as f32 * self.cell_size }

    pub fn is_inside(&self, c: Cell) -> bool {
        c.x >= 0 && c.y >= 0
            && (c.x as u32) < self.grid_cols
            && (c.y as u32) < self.grid_rows
    }

    /// Durée du tick courant, en secondes, selon le score.
    pub fn current_tick_secs(&self, food_eaten: u32) -> f32 {
        let steps = food_eaten / self.food_per_speedup;
        let ms = self.initial_tick_ms
            .saturating_sub(steps * self.tick_speedup_ms)
            .max(self.min_tick_ms);
        ms as f32 / 1000.0
    }
}

/// Résultat d'un tick, pour que `main.rs` réagisse sans dupliquer la logique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickEvent {
    None,
    Ate,
    Died,
}

/// État de progression d'un niveau Snake.
///
/// Combine le `TickTimer` générique du core avec les champs spécifiques
/// au jeu (compteur de pommes, objectif).
#[derive(Debug, Clone)]
pub struct SnakeState {
    pub timer: TickTimer,
    pub food_eaten: u32,
    pub target_score: u32,
}

impl SnakeState {
    pub fn new(tick_duration: f32) -> Self {
        Self {
            timer: TickTimer::new(tick_duration),
            food_eaten: 0,
            target_score: 10,
        }
    }

    pub fn reset(&mut self, tick_duration: f32) {
        self.timer = TickTimer::new(tick_duration);
        self.food_eaten = 0;
    }

    pub fn is_level_complete(&self) -> bool {
        self.food_eaten >= self.target_score
    }
}

// ============================================================================
// SnakeWorld
// ============================================================================

/// L'état mutable d'une partie de Snake.
///
/// Regroupe tout ce qui change d'un tick à l'autre : le serpent, la
/// nourriture, les murs du niveau courant, le score, l'état de la machine
/// à états, et le timer de tick.
///
/// `ctx` (config) et `dt` (temps) restent des paramètres d'`update` :
/// ils ne sont pas de l'état, ils le font évoluer.
///
/// **Ce qui n'est PAS dans `SnakeWorld`** : le best score et l'index du
/// niveau courant. Ce sont des données de progression inter-niveaux, elles
/// appartiennent à la couche qui orchestre les niveaux (`main.rs`).
pub struct SnakeWorld {
    pub snake: Snake,
    pub food: Food,
    pub walls: Vec<Wall>,
    pub score: i32,
    pub state: GameState,
    pub snake_state: SnakeState,
}

impl SnakeWorld {
    /// Construit un monde à partir de pièces déjà créées.
    ///
    /// Utile pour les tests (pas besoin de fabriquer un `SnakeLevel`),
    /// et pour tout appelant qui veut contrôler chaque élément.
    /// L'état résultant est `Start`, timer basé sur `ctx.initial_tick_ms`.
    pub fn new(
        snake: Snake,
        food: Food,
        walls: Vec<Wall>,
        ctx: &GameContext,
    ) -> Self {
        let tick_duration = ctx.initial_tick_ms as f32 / 1000.0;
        Self {
            snake,
            food,
            walls,
            score: 0,
            state: GameState::Start,
            snake_state: SnakeState::new(tick_duration),
        }
    }

    /// Construit un monde à partir d'un niveau chargé.
    ///
    /// Version "one-shot" pour `main.rs` au démarrage : équivalent à
    /// `SnakeWorld::new(...)` suivi de `apply_level(...)`, mais sans
    /// devoir fabriquer un serpent jetable d'abord.
    pub fn from_level(level: &SnakeLevel, ctx: &GameContext) -> Self {
        let spawn = Cell::new(level.spawn.0, level.spawn.1);
        let dir = parse_direction(&level.spawn_direction);

        let snake = Snake::new(
            spawn,
            level.initial_length,
            dir,
            ctx.snake_head_color,
            ctx.snake_body_color,
        );

        let walls: Vec<Wall> = level
            .walls
            .iter()
            .map(|(x, y)| Wall::new(Cell::new(*x, *y), ctx.wall_color))
            .collect();

        let food = spawn_food_avoiding(&snake, &walls, ctx);

        let tick_duration = ctx.initial_tick_ms as f32 / 1000.0;
        let mut snake_state = SnakeState::new(tick_duration);
        snake_state.target_score = level.target_score;

        Self {
            snake,
            food,
            walls,
            score: 0,
            state: GameState::Start,
            snake_state,
        }
    }

    /// Applique un niveau : reconfigure le serpent, les murs, la nourriture,
    /// et le timer. Le score global est préservé (c'est un score cumulé
    /// sur toute la partie, pas un score par niveau).
    pub fn apply_level(&mut self, level: &SnakeLevel, ctx: &GameContext) {
        let spawn = Cell::new(level.spawn.0, level.spawn.1);
        let dir = parse_direction(&level.spawn_direction);

        self.snake = Snake::new(
            spawn,
            level.initial_length,
            dir,
            ctx.snake_head_color,
            ctx.snake_body_color,
        );

        self.walls.clear();
        for (x, y) in &level.walls {
            self.walls.push(Wall::new(Cell::new(*x, *y), ctx.wall_color));
        }

        self.food = spawn_food_avoiding(&self.snake, &self.walls, ctx);

        let tick_duration = ctx.initial_tick_ms as f32 / 1000.0;
        self.snake_state.reset(tick_duration);
        self.snake_state.target_score = level.target_score;
    }

    /// Réinitialise serpent, nourriture, timer (sans toucher au score,
    /// aux murs ni à l'état).
    pub fn reset_round(&mut self, ctx: &GameContext) {
        let start = Cell::new(ctx.grid_cols as i32 / 2, ctx.grid_rows as i32 / 2);
        self.snake = Snake::new(
            start,
            3,
            Direction::Right,
            ctx.snake_head_color,
            ctx.snake_body_color,
        );
        self.food = spawn_food(&self.snake, ctx);
        let tick_duration = ctx.initial_tick_ms as f32 / 1000.0;
        self.snake_state.reset(tick_duration);
    }
}

// ============================================================================
// Helpers (inchangés)
// ============================================================================

/// Choisit une case libre au hasard pour la nourriture.
pub fn spawn_food(snake: &Snake, ctx: &GameContext) -> Food {
    spawn_food_avoiding(snake, &[], ctx)
}

pub fn spawn_food_avoiding(snake: &Snake, walls: &[Wall], ctx: &GameContext) -> Food {
    use macroquad::rand::gen_range;

    let is_occupied = |c: Cell| -> bool {
        snake.body.contains(&c) || walls.iter().any(|w| w.cell == c)
    };

    for _ in 0..200 {
        let c = Cell::new(
            gen_range(0, ctx.grid_cols as i32),
            gen_range(0, ctx.grid_rows as i32),
        );
        if !is_occupied(c) {
            return Food::new(c, ctx.food_color);
        }
    }

    for y in 0..ctx.grid_rows as i32 {
        for x in 0..ctx.grid_cols as i32 {
            let c = Cell::new(x, y);
            if !is_occupied(c) {
                return Food::new(c, ctx.food_color);
            }
        }
    }

    Food::new(Cell::new(-1, -1), ctx.food_color)
}

/// Un pas de simulation.
///
/// **Primitive de bas niveau** : opère sur les pièces individuelles
/// plutôt que sur un `SnakeWorld`. Les tests l'appellent directement,
/// et `update` l'appelle en boucle.
pub fn tick(
    snake: &mut Snake,
    food: &mut Food,
    walls: &[Wall],
    ctx: &GameContext,
    score: &mut i32,
    food_eaten: &mut u32,
) -> TickEvent {
    snake.apply_pending_direction();

    let will_eat = snake.head() + snake.direction.delta() == food.cell;
    let new_head = snake.advance(will_eat);

    if !ctx.is_inside(new_head) {
        return TickEvent::Died;
    }

    if walls.iter().any(|w| w.cell == new_head) {
        return TickEvent::Died;
    }

    let body_without_head: Vec<Cell> = snake.body.iter().skip(1).copied().collect();
    if body_without_head.contains(&new_head) {
        return TickEvent::Died;
    }

    if will_eat {
        *score += 1;
        *food_eaten += 1;
        let new_food = spawn_food_avoiding(snake, walls, ctx);
        if new_food.cell.x < 0 {
            return TickEvent::Ate;
        }
        *food = new_food;
        return TickEvent::Ate;
    }

    TickEvent::None
}

/// Appelée chaque frame. Accumule le temps et déclenche des ticks
/// tant que nécessaire. Retourne le dernier événement significatif.
pub fn update(world: &mut SnakeWorld, ctx: &GameContext, dt: f32) -> TickEvent {
    if world.state != GameState::Playing {
        return TickEvent::None;
    }

    // Adapter la durée du tick AVANT d'avancer.
    let tick_dur = ctx.current_tick_secs(world.snake_state.food_eaten);
    world.snake_state.timer.set_duration(tick_dur);

    let ticks = world.snake_state.timer.advance(dt);
    let mut last_event = TickEvent::None;

    for _ in 0..ticks {
        let ev = tick(
            &mut world.snake,
            &mut world.food,
            &world.walls,
            ctx,
            &mut world.score,
            &mut world.snake_state.food_eaten,
        );
        match ev {
            TickEvent::Died => {
                world.state = GameState::GameOver;
                return TickEvent::Died;
            }
            other => last_event = other,
        }

        if world.snake_state.is_level_complete() {
            world.state = GameState::LevelCleared;
            return TickEvent::Ate;
        }
    }

    last_event
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> GameContext {
        GameContext {
            grid_cols: 10,
            grid_rows: 10,
            cell_size: 10.0,
            initial_tick_ms: 100,
            min_tick_ms: 50,
            tick_speedup_ms: 5,
            food_per_speedup: 2,
            ..Default::default()
        }
    }

    fn snake_at(cx: i32, cy: i32, dir: Direction) -> Snake {
        Snake::new(
            Cell::new(cx, cy),
            1,
            dir,
            Color::new(0.0, 1.0, 0.0, 1.0),
            Color::new(0.0, 0.5, 0.0, 1.0),
        )
    }

    #[test]
    fn test_tick_moves_snake() {
        let cx = ctx();
        let mut s = snake_at(5, 5, Direction::Right);
        let mut food = Food::new(Cell::new(0, 0), Color::new(1.0, 0.0, 0.0, 1.0));
        let mut score = 0;
        let mut eaten = 0;
        let ev = tick(&mut s, &mut food, &[], &cx, &mut score, &mut eaten);
        assert_eq!(ev, TickEvent::None);
        assert_eq!(s.head(), Cell::new(6, 5));
    }

    #[test]
    fn test_tick_eats_food_and_grows() {
        let cx = ctx();
        let mut s = snake_at(5, 5, Direction::Right);
        let mut food = Food::new(Cell::new(6, 5), Color::new(1.0, 0.0, 0.0, 1.0));
        let mut score = 0;
        let mut eaten = 0;
        let ev = tick(&mut s, &mut food, &[], &cx, &mut score, &mut eaten);
        assert_eq!(ev, TickEvent::Ate);
        assert_eq!(s.len(), 2);
        assert_eq!(score, 1);
        assert_eq!(eaten, 1);
    }

    #[test]
    fn test_tick_self_collision() {
        let cx = ctx();
        let mut s = Snake::new(
            Cell::new(5, 5), 1,
            Direction::Left,
            Color::new(0.0, 1.0, 0.0, 1.0),
            Color::new(0.0, 0.5, 0.0, 1.0),
        );
        s.body.clear();
        s.body.push_back(Cell::new(5, 5));
        s.body.push_back(Cell::new(4, 5));
        s.body.push_back(Cell::new(4, 4));
        s.body.push_back(Cell::new(5, 4));
        let mut food = Food::new(Cell::new(9, 9), Color::new(1.0, 0.0, 0.0, 1.0));
        let mut score = 0;
        let mut eaten = 0;
        let ev = tick(&mut s, &mut food, &[], &cx, &mut score, &mut eaten);
        assert_eq!(ev, TickEvent::Died);
    }

    #[test]
    fn test_current_tick_secs_speeds_up() {
        let cx = ctx();
        assert_eq!(cx.current_tick_secs(0), 0.100);
        assert_eq!(cx.current_tick_secs(2), 0.095);
        assert_eq!(cx.current_tick_secs(4), 0.090);
        assert_eq!(cx.current_tick_secs(10_000), 0.050);
    }

    #[test]
    fn test_spawn_food_not_on_snake() {
        let _cx = ctx();
        let s = Snake::new(
            Cell::new(0, 0), 1,
            Direction::Right,
            Color::new(0.0, 1.0, 0.0, 1.0),
            Color::new(0.0, 0.5, 0.0, 1.0),
        );
        let small = GameContext {
            grid_cols: 2,
            grid_rows: 1,
            ..ctx()
        };
        let f = spawn_food(&s, &small);
        assert_eq!(f.cell, Cell::new(1, 0));
    }

    #[test]
    fn test_tick_wall_collision() {
        let cx = GameContext {
            grid_cols: 10,
            grid_rows: 10,
            ..Default::default()
        };
        let mut s = snake_at(5, 5, Direction::Right);
        let mut food = Food::new(Cell::new(0, 0), Color::new(1.0, 0.0, 0.0, 1.0));
        let walls = vec![Wall::new(Cell::new(6, 5), Color::new(0.5, 0.5, 0.5, 1.0))];
        let mut score = 0;
        let mut eaten = 0;
        let ev = tick(&mut s, &mut food, &walls, &cx, &mut score, &mut eaten);
        assert_eq!(ev, TickEvent::Died);
    }

    #[test]
    fn test_spawn_food_avoids_walls() {
        let cx = GameContext {
            grid_cols: 2,
            grid_rows: 2,
            ..Default::default()
        };
        let s = Snake::new(
            Cell::new(0, 0), 1, Direction::Right,
            Color::new(0.0, 1.0, 0.0, 1.0),
            Color::new(0.0, 0.5, 0.0, 1.0),
        );
        let walls = vec![
            Wall::new(Cell::new(0, 1), Color::new(0.5, 0.5, 0.5, 1.0)),
            Wall::new(Cell::new(1, 0), Color::new(0.5, 0.5, 0.5, 1.0)),
        ];
        let f = spawn_food_avoiding(&s, &walls, &cx);
        assert_eq!(f.cell, Cell::new(1, 1));
    }

    // --- Tests SnakeState ---

    #[test]
    fn test_snake_state_new() {
        let s = SnakeState::new(0.1);
        assert_eq!(s.food_eaten, 0);
        assert_eq!(s.target_score, 10);
        assert_eq!(s.timer.duration(), 0.1);
    }

    #[test]
    fn test_snake_state_reset() {
        let mut s = SnakeState::new(0.1);
        s.food_eaten = 5;
        s.reset(0.05);
        assert_eq!(s.food_eaten, 0);
        assert_eq!(s.timer.duration(), 0.05);
        assert_eq!(s.target_score, 10);
    }

    #[test]
    fn test_snake_state_level_complete() {
        let mut s = SnakeState::new(0.1);
        s.target_score = 5;
        s.food_eaten = 4;
        assert!(!s.is_level_complete());
        s.food_eaten = 5;
        assert!(s.is_level_complete());
        s.food_eaten = 6;
        assert!(s.is_level_complete());
    }

    // --- Tests SnakeWorld ---

    #[test]
    fn test_world_new_starts_at_start_state() {
        let cx = ctx();
        let w = SnakeWorld::new(
            snake_at(5, 5, Direction::Right),
            Food::new(Cell::new(0, 0), Color::new(1.0, 0.0, 0.0, 1.0)),
            Vec::new(),
            &cx,
        );
        assert_eq!(w.state, GameState::Start);
        assert_eq!(w.score, 0);
        assert_eq!(w.snake_state.food_eaten, 0);
    }

    #[test]
    fn test_world_update_ignores_non_playing_state() {
        let cx = ctx();
        let mut w = SnakeWorld::new(
            snake_at(5, 5, Direction::Right),
            Food::new(Cell::new(0, 0), Color::new(1.0, 0.0, 0.0, 1.0)),
            Vec::new(),
            &cx,
        );
        // state == Start par défaut
        let ev = update(&mut w, &cx, 1.0);
        assert_eq!(ev, TickEvent::None);
        assert_eq!(w.snake.head(), Cell::new(5, 5));
    }

    #[test]
    fn test_world_reset_round_places_snake_at_center() {
        let cx = ctx();
        let mut w = SnakeWorld::new(
            snake_at(0, 0, Direction::Up),
            Food::new(Cell::new(5, 5), Color::new(1.0, 0.0, 0.0, 1.0)),
            Vec::new(),
            &cx,
        );
        w.snake_state.food_eaten = 3;

        w.reset_round(&cx);

        assert_eq!(
            w.snake.head(),
            Cell::new(cx.grid_cols as i32 / 2, cx.grid_rows as i32 / 2)
        );
        assert_eq!(w.snake_state.food_eaten, 0);
    }
}