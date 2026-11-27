//! A level as a node tree (spec 42 §4):
//!
//! ```text
//! World                  the battle: towers, mana, AI, combat
//! ├── Catapults
//! │   ├── PlayerCatapult   Catapult
//! │   └── EnemyCatapult    Catapult (levels that have one)
//! ├── Units                one Unit node per stickman
//! └── Projectiles          one Projectile node per shot in flight
//! ```
//!
//! Catapults, units and projectiles live in their nodes. Combat is
//! pairwise (a unit picks a target among the others, a shot hits the
//! first body in its way), so each frame `World` takes them out of their
//! nodes in tree order, runs the battle (`Game::tick`) on them, and puts
//! them back: units by `UnitId` (a new one gets a new node, a dead one's
//! node is despawned), projectiles in order (they have no identity).
//! There is no cached list of unit ids: the children are walked each
//! frame (spec 42 §5.1). Each node draws its own data.
//!
//! The world never touches the campaign. It emits `LevelFinished` once
//! when the battle ends, and `RestartLevel` / `LeaveLevel` for R, Enter
//! and Escape; the root (`EmberWars`) saves and swaps the world.

use std::collections::HashMap;
use std::sync::Arc;

use crate::catapult::{Catapult, ShotKind};
use crate::components::{Projectile, Team, Unit, UnitId};
use crate::config::{Colors, GameContext};
use crate::frame::{mouse_position, screen_height, screen_width};
use crate::render::{self, Hud};
use crate::scene::MAX_FRAME_DT;
use crate::systems::{Game, Phase};
use crate::ui::spawn_bar::SpawnBar;
use ember::prelude::*;

// ---------- Signals ----------

/// The battle ended (once per level). The root banks the gold and the
/// stars and saves.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelFinished {
    pub level_id: String,
    pub won: bool,
    /// The gold bank at the end of the battle (kills included).
    pub gold: f32,
    /// The level's reward; 0 on a defeat.
    pub reward: f32,
    pub tower_hp_fraction: f32,
}

/// R: play this level again from the start.
#[derive(Debug, Clone, PartialEq)]
pub struct RestartLevel {
    pub level_id: String,
}

/// Escape, or Enter once the battle is over: back to the menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeaveLevel;

// ---------- Nodes ----------

macro_rules! node_object {
    ($ty:ty, $class:literal) => {
        impl Object for $ty {
            fn class_name() -> &'static str {
                $class
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }

        impl NodeName for $ty {
            fn name(&self) -> &str {
                &self.name
            }
            fn set_name(&mut self, name: String) {
                self.name = name;
            }
        }
    };
}

/// Where the world is seen from this frame; every world node draws
/// through it.
#[derive(Debug, Clone)]
pub struct View {
    pub cam_x: f32,
    pub shake: Vec2,
    pub colors: Arc<Colors>,
}

/// A container: `Catapults`, `Units`, `Projectiles`.
pub struct Group {
    name: String,
}

impl Group {
    pub fn new(name: &str) -> Self {
        Self { name: name.into() }
    }
}

node_object!(Group, "Group");
impl Node for Group {}

/// A catapult. `None` only while the world runs the frame's battle.
pub struct CatapultNode {
    name: String,
    pub team: Team,
    pub catapult: Option<Catapult>,
    view: View,
}

node_object!(CatapultNode, "Catapult");

impl Node for CatapultNode {
    fn draw(&self, ctx: &mut DrawContext) {
        if let Some(c) = &self.catapult {
            let v = &self.view;
            render::draw_catapult(ctx.render(), &v.colors, c, self.team, v.cam_x, v.shake);
        }
    }
}

/// A stickman. `id` stays with the node for its whole life.
pub struct UnitNode {
    name: String,
    pub id: UnitId,
    pub unit: Option<Unit>,
    view: View,
}

node_object!(UnitNode, "Unit");

impl Node for UnitNode {
    fn draw(&self, ctx: &mut DrawContext) {
        if let Some(u) = &self.unit {
            let v = &self.view;
            render::draw_unit(ctx.render(), &v.colors, u, v.cam_x, v.shake);
        }
    }
}

/// A shot in flight.
pub struct ProjectileNode {
    name: String,
    pub projectile: Option<Projectile>,
    view: View,
}

node_object!(ProjectileNode, "Projectile");

impl Node for ProjectileNode {
    fn draw(&self, ctx: &mut DrawContext) {
        if let Some(p) = &self.projectile {
            let v = &self.view;
            render::draw_projectile(ctx.render(), p, v.cam_x, v.shake);
        }
    }
}

fn node_mut<T: Node>(tree: &mut NodeTree, id: NodeId) -> Option<&mut T> {
    tree.get_mut(id)?.as_any_mut().downcast_mut::<T>()
}

// ---------- World ----------

/// The ids of the world's fixed children, set in `ready`.
#[derive(Debug, Clone, Copy, Default)]
struct Slots {
    player_catapult: NodeId,
    enemy_catapult: Option<NodeId>,
    units: NodeId,
    projectiles: NodeId,
}

pub struct World {
    name: String,
    ctx: GameContext,
    colors: Arc<Colors>,
    /// The battle. Its catapults, units and projectiles are only filled
    /// while `process` runs; between frames they are in the nodes.
    pub game: Game,
    spawn_bar: SpawnBar,
    slots: Slots,
    /// This frame's screen-shake offset, taken once per frame (it
    /// advances the shake's seed).
    shake: Vec2,
    hud: Hud,
    finished: bool,
}

impl World {
    pub fn new(ctx: &GameContext, game: Game) -> Self {
        let hud = Hud::of(&game);
        Self {
            name: "World".into(),
            colors: Arc::new(ctx.colors.clone()),
            ctx: ctx.clone(),
            game,
            spawn_bar: SpawnBar::new(),
            slots: Slots::default(),
            shake: Vec2::ZERO,
            hud,
            finished: false,
        }
    }

    pub fn level_id(&self) -> &str {
        &self.game.level.id
    }

    fn view(&self) -> View {
        View {
            cam_x: self.game.camera.x,
            shake: self.shake,
            colors: Arc::clone(&self.colors),
        }
    }

    /// Take the catapults, units and projectiles out of their nodes, in
    /// tree order.
    fn gather(&mut self, tree: &mut NodeTree) {
        let s = self.slots;
        if let Some(c) =
            node_mut::<CatapultNode>(tree, s.player_catapult).and_then(|n| n.catapult.take())
        {
            self.game.catapult = c;
        }
        if let Some(id) = s.enemy_catapult {
            self.game.enemy_catapult =
                node_mut::<CatapultNode>(tree, id).and_then(|n| n.catapult.take());
        }
        for id in tree.children(s.units).to_vec() {
            if let Some(u) = node_mut::<UnitNode>(tree, id).and_then(|n| n.unit.take()) {
                self.game.units.push(u);
            }
        }
        for id in tree.children(s.projectiles).to_vec() {
            if let Some(p) = node_mut::<ProjectileNode>(tree, id).and_then(|n| n.projectile.take())
            {
                self.game.projectiles.push(p);
            }
        }
    }

    /// Put them back: each unit into its node (a new unit gets one, a
    /// node whose unit is gone is despawned), the projectiles in order.
    fn scatter(&mut self, ctx: &mut NodeContext) {
        self.hud = Hud::of(&self.game);
        let view = self.view();
        let s = self.slots;
        let tree = ctx.tree();

        let catapult = std::mem::replace(&mut self.game.catapult, Catapult::new(Vec2::ZERO));
        if let Some(n) = node_mut::<CatapultNode>(tree, s.player_catapult) {
            n.catapult = Some(catapult);
            n.view = view.clone();
        }
        if let Some(id) = s.enemy_catapult
            && let Some(n) = node_mut::<CatapultNode>(tree, id)
        {
            n.catapult = self.game.enemy_catapult.take();
            n.view = view.clone();
        }

        let mut unit_nodes: HashMap<UnitId, NodeId> = HashMap::new();
        for id in tree.children(s.units).to_vec() {
            if let Some(n) = node_mut::<UnitNode>(tree, id) {
                unit_nodes.insert(n.id, id);
            }
        }
        for unit in std::mem::take(&mut self.game.units) {
            match unit_nodes.remove(&unit.id) {
                Some(id) => {
                    if let Some(n) = node_mut::<UnitNode>(ctx.tree(), id) {
                        n.unit = Some(unit);
                        n.view = view.clone();
                    }
                }
                None => {
                    ctx.spawn_child(
                        s.units,
                        UnitNode {
                            name: format!("Unit{}", unit.id.0),
                            id: unit.id,
                            unit: Some(unit),
                            view: view.clone(),
                        },
                    );
                }
            }
        }
        for id in unit_nodes.into_values() {
            ctx.despawn(id);
        }

        let nodes = ctx.tree().children(s.projectiles).to_vec();
        let mut shots = std::mem::take(&mut self.game.projectiles).into_iter();
        for id in nodes {
            match shots.next() {
                Some(p) => {
                    if let Some(n) = node_mut::<ProjectileNode>(ctx.tree(), id) {
                        n.projectile = Some(p);
                        n.view = view.clone();
                    }
                }
                None => ctx.despawn(id),
            }
        }
        for p in shots {
            ctx.spawn_child(
                s.projectiles,
                ProjectileNode {
                    name: "Projectile".into(),
                    projectile: Some(p),
                    view: view.clone(),
                },
            );
        }
    }

    /// The battle's frame, as the old loop ran it.
    fn play(&mut self, keys: &Input, dt: f32) {
        let g = &mut self.game;
        g.camera.viewport_w = self.ctx.viewport_w;
        if g.phase != Phase::Playing {
            return;
        }
        let mut cam_dir = 0.0;
        if keys.is_key_down(Key::Left) {
            cam_dir -= 1.0;
        }
        if keys.is_key_down(Key::Right) {
            cam_dir += 1.0;
        }

        let mouse_screen = Vec2::new(mouse_position().0, mouse_position().1);
        let mouse_world = Vec2::new(g.camera.screen_to_world(mouse_screen.x), mouse_screen.y);

        let mut consumed = false;
        if keys.mouse_left_pressed
            && let Some(kind) = self.spawn_bar.handle_click(keys, g)
        {
            g.try_player_spawn(&kind);
            consumed = true;
        }

        let fire_click = keys.mouse_left_pressed && !consumed;
        g.tick(dt, mouse_world, fire_click, cam_dir);
    }

    /// The level's keys after the battle's frame. Returns `false` once
    /// the world has asked to be replaced.
    fn keys(&mut self, keys: &Input, ctx: &mut NodeContext) -> bool {
        if keys.is_key_pressed(Key::Escape) {
            ctx.emit(LeaveLevel);
            return false;
        }
        if keys.is_key_pressed(Key::R) {
            ctx.emit(RestartLevel {
                level_id: self.game.level.id.clone(),
            });
            return false;
        }
        if keys.is_key_pressed(Key::Enter) && self.game.phase != Phase::Playing {
            ctx.emit(LeaveLevel);
            return false;
        }
        let shot = &mut self.game.catapult.shot_kind;
        if keys.is_key_pressed(Key::Num1) {
            *shot = ShotKind::Basic;
        }
        if keys.is_key_pressed(Key::Num2) {
            *shot = ShotKind::Piercing;
        }
        if keys.is_key_pressed(Key::Num3) {
            *shot = ShotKind::Explosive;
        }
        true
    }
}

node_object!(World, "World");

impl Node for World {
    fn ready(&mut self, ctx: &mut NodeContext) {
        let me = ctx.current_id();
        let view = self.view();
        let catapults = ctx.spawn_child(me, Group::new("Catapults"));
        let catapult = std::mem::replace(&mut self.game.catapult, Catapult::new(Vec2::ZERO));
        self.slots.player_catapult = ctx.spawn_child(
            catapults,
            CatapultNode {
                name: "PlayerCatapult".into(),
                team: Team::Player,
                catapult: Some(catapult),
                view: view.clone(),
            },
        );
        self.slots.enemy_catapult = self.game.enemy_catapult.take().map(|c| {
            ctx.spawn_child(
                catapults,
                CatapultNode {
                    name: "EnemyCatapult".into(),
                    team: Team::Enemy,
                    catapult: Some(c),
                    view: view.clone(),
                },
            )
        });
        self.slots.units = ctx.spawn_child(me, Group::new("Units"));
        self.slots.projectiles = ctx.spawn_child(me, Group::new("Projectiles"));
        // The old loop took the shake once on the frame a level began.
        self.shake = self.game.juice.screen_shake.offset();
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        // The root set this frame's size, pointer and time.
        self.ctx.viewport_w = screen_width();
        self.ctx.viewport_h = screen_height();
        let keys = ctx.input().clone();
        let dt = dt.min(MAX_FRAME_DT);

        self.gather(ctx.tree());
        self.play(&keys, dt);

        // The end of the battle, once.
        if self.game.phase != Phase::Playing && !self.finished {
            self.finished = true;
            let g = &self.game;
            let won = g.phase == Phase::Won;
            ctx.emit(LevelFinished {
                level_id: g.level.id.clone(),
                won,
                gold: g.player_upgrades.gold,
                reward: if won { g.level.reward_gold } else { 0.0 },
                tower_hp_fraction: g.tower_hp_fraction(),
            });
        }

        if self.keys(&keys, ctx) {
            self.shake = self.game.juice.screen_shake.offset();
        }
        self.scatter(ctx);
    }

    fn draw(&self, ctx: &mut DrawContext) {
        render::draw_backdrop(ctx.render(), &self.ctx, &self.game, self.shake);
    }

    fn draw_after_children(&self, ctx: &mut DrawContext) {
        let gfx = ctx.render();
        render::draw_overlay(gfx, &self.ctx, &self.game, &self.hud, self.shake);
        self.spawn_bar
            .draw(gfx, &self.ctx, &self.game, &self.hud.at_cap);
        render::draw_end_overlay(gfx, &self.ctx, &self.game);
    }
}
