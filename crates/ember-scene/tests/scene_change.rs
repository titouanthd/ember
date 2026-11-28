//! Runtime scene change (ADR-054), headless on mocks.
//!
//! Order within the frame a change is applied in (`SceneLoop::process`):
//! 1. the old scene's `process` pass runs to the end, the requester and
//!    every node after it included (nothing changes mid-pass);
//! 2. the new scene is built, floating;
//! 3. the old nodes are finalized: the old root's subtree bottom-up
//!    (children before parents, in child order), then any other old
//!    node in insertion order;
//! 4. the new root becomes the root and is readied bottom-up;
//! 5. a change asked for in that `ready` is applied next (steps 2–4);
//! 6. `draw` draws the new scene; its first `process` is the next frame.

mod common;
use common::*;
use ember_core::math::Rect;
use ember_core::{ClassDB, MainLoop, Object};
use ember_input::{ActionMap, Key};
use ember_scene::prelude::*;
use ember_servers::{MockAudio, MockRenderer};
use std::any::Any;
use std::sync::Arc;

type Loop = SceneLoop<MockRenderer, MockAudio>;

fn scene_loop(tree: NodeTree) -> Loop {
    SceneLoop::new(
        tree,
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 800.0, 600.0),
    )
}

/// A builder for `name -> [name_child]`, both `Recorder`s.
fn recorder_scene(
    name: &'static str,
    log: &Log,
) -> impl for<'a> FnOnce(&mut NodeTree, &SceneLibrary<'a>) -> Result<NodeId, SceneError>
+ Send
+ Sync
+ 'static {
    let log = log.clone();
    move |tree, _| {
        let root = tree.insert(Recorder::new(name, &log));
        let child = tree.insert(Recorder::new(&format!("{name}_child"), &log));
        tree.attach(child, root)?;
        Ok(root)
    }
}

/// root (Script, asks for a change once) -> [a -> [a1], b], Recorders.
fn old_scene(
    log: &Log,
    ask: impl FnOnce(&mut NodeContext) + Send + Sync + 'static,
) -> (NodeTree, NodeId) {
    let mut ask = Some(ask);
    let mut t = NodeTree::new();
    let a = t.insert(Recorder::new("a", log));
    let a1 = t.insert(Recorder::new("a1", log));
    let script = t.insert(Script::new("script", move |ctx| {
        if let Some(ask) = ask.take() {
            ask(ctx);
        }
    }));
    let b = t.insert(Recorder::new("b", log));
    let root = t.insert(Recorder::new("root", log));
    t.attach(a, root).unwrap();
    t.attach(a1, a).unwrap();
    t.attach(script, root).unwrap();
    t.attach(b, root).unwrap();
    t.set_root(root);
    (t, root)
}

fn names(l: &Loop) -> Vec<String> {
    let mut v: Vec<String> = l.tree.iter().map(|(_, n)| n.name().to_string()).collect();
    v.sort();
    v
}

#[test]
fn old_scene_finalizes_bottom_up_then_new_root_readies() {
    let log = new_log();
    let new = recorder_scene("level", &log);
    let (tree, _) = old_scene(&log, move |ctx| ctx.change_scene_with(new));
    let mut l = scene_loop(tree);
    assert!(l.init());
    log.lock().unwrap().clear();

    assert!(l.process(0.016));
    assert_eq!(
        entries(&log),
        [
            // the whole pass runs: b comes after the script
            "root:process",
            "a:process",
            "a1:process",
            "b:process",
            // old root's subtree, bottom-up
            "a1:finalize",
            "a:finalize",
            "b:finalize",
            "root:finalize",
            // new scene, bottom-up
            "level_child:ready",
            "level:ready",
        ]
    );
    assert_eq!(names(&l), ["level", "level_child"]);
    assert!(!l.tree.scene_change_pending());
}

#[test]
fn the_new_scene_draws_this_frame_and_processes_next_frame() {
    let log = new_log();
    let new = recorder_scene("level", &log);
    let (tree, _) = old_scene(&log, move |ctx| ctx.change_scene_with(new));
    let mut l = scene_loop(tree);
    l.init();
    l.process(0.016);
    log.lock().unwrap().clear();

    l.draw();
    l.physics_process(1.0 / 60.0);
    l.process(0.016);
    assert_eq!(
        entries(&log),
        [
            "level:draw",
            "level_child:draw",
            "level:physics",
            "level_child:physics",
            "level:process",
            "level_child:process",
        ]
    );
}

#[test]
fn nothing_changes_mid_pass() {
    let log = new_log();
    let seen = new_log();
    let seen2 = seen.clone();
    let new = recorder_scene("level", &log);
    let (tree, old_root) = old_scene(&log, move |ctx| {
        ctx.change_scene_with(new);
        let t = ctx.tree();
        seen2
            .lock()
            .unwrap()
            .push(format!("{:?} {}", t.root(), t.scene_change_pending()));
    });
    let mut l = scene_loop(tree);
    l.init();
    l.process(0.016);
    assert_eq!(entries(&seen), [format!("{:?} true", Some(old_root))]);
    assert_ne!(l.tree.root(), Some(old_root));
    assert!(!l.tree.contains(old_root));
}

#[test]
fn a_change_from_physics_process_waits_for_the_end_of_process() {
    struct PhysicsAsker(Option<SceneChange>);
    impl Object for PhysicsAsker {
        fn class_name() -> &'static str {
            "PhysicsAsker"
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    impl NodeName for PhysicsAsker {
        fn name(&self) -> &str {
            "asker"
        }
        fn set_name(&mut self, _: String) {}
    }
    impl Node for PhysicsAsker {
        fn physics_process(&mut self, _: f32, ctx: &mut NodeContext) {
            if let Some(next) = self.0.take() {
                ctx.change_scene(next);
            }
        }
    }

    let log = new_log();
    let new = recorder_scene("level", &log);
    let mut tree = NodeTree::with_root(Recorder::new("root", &log));
    let root = tree.root().unwrap();
    let asker = tree.insert(PhysicsAsker(Some(SceneChange::build(new))));
    tree.attach(asker, root).unwrap();
    let mut l = scene_loop(tree);
    l.init();
    log.lock().unwrap().clear();

    l.physics_process(1.0 / 60.0);
    l.physics_process(1.0 / 60.0);
    assert!(l.tree.scene_change_pending());
    l.process(0.016);
    assert_eq!(
        entries(&log),
        [
            "root:physics",
            "root:physics",
            "root:process",
            "root:finalize",
            "level_child:ready",
            "level:ready",
        ]
    );
}

/// On `ready`, asks to change to `next` (once).
struct ChangeOnReady {
    name: &'static str,
    log: Log,
    next: Option<SceneChange>,
}

impl Object for ChangeOnReady {
    fn class_name() -> &'static str {
        "ChangeOnReady"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl NodeName for ChangeOnReady {
    fn name(&self) -> &str {
        self.name
    }
    fn set_name(&mut self, _: String) {}
}

impl Node for ChangeOnReady {
    fn ready(&mut self, ctx: &mut NodeContext) {
        self.log
            .lock()
            .unwrap()
            .push(format!("{}:ready", self.name));
        if let Some(next) = self.next.take() {
            ctx.change_scene(next);
        }
    }
    fn finalize(&mut self) {
        self.log
            .lock()
            .unwrap()
            .push(format!("{}:finalize", self.name));
    }
}

#[test]
fn chained_changes_apply_in_the_same_frame() {
    let log = new_log();
    let (l2, l3) = (log.clone(), log.clone());
    // splash -> title (whose ready asks for the menu) -> menu
    let title = SceneChange::build(move |tree, _| {
        Ok(tree.insert(ChangeOnReady {
            name: "title",
            log: l2,
            next: Some(SceneChange::build(move |tree, _| {
                Ok(tree.insert(ChangeOnReady {
                    name: "menu",
                    log: l3,
                    next: None,
                }))
            })),
        }))
    });
    let mut title = Some(title);
    let tree = NodeTree::with_root(Script::new("splash", move |ctx| {
        if let Some(t) = title.take() {
            ctx.change_scene(t);
        }
    }));
    let mut l = scene_loop(tree);
    l.init();
    l.process(0.016);
    assert_eq!(
        entries(&log),
        ["title:ready", "title:finalize", "menu:ready"]
    );
    assert_eq!(names(&l), ["menu"]);
}

#[test]
fn a_change_asked_for_during_init_applies_in_init() {
    let log = new_log();
    let l2 = log.clone();
    let tree = NodeTree::with_root(ChangeOnReady {
        name: "boot",
        log: log.clone(),
        next: Some(SceneChange::build(move |tree, _| {
            Ok(tree.insert(Recorder::new("menu", &l2)))
        })),
    });
    let mut l = scene_loop(tree);
    assert!(l.init());
    assert_eq!(entries(&log), ["boot:ready", "boot:finalize", "menu:ready"]);
}

#[test]
fn the_last_request_in_a_frame_wins() {
    let log = new_log();
    let (first, second) = (
        recorder_scene("first", &log),
        recorder_scene("second", &log),
    );
    let (tree, _) = old_scene(&log, move |ctx| {
        ctx.change_scene_with(first);
        ctx.change_scene_with(second);
    });
    let mut l = scene_loop(tree);
    l.init();
    l.process(0.016);
    assert_eq!(names(&l), ["second", "second_child"]);
    assert!(!entries(&log).iter().any(|e| e.starts_with("first")));
}

// --- Failure keeps the old scene ---

#[derive(Default)]
struct Tile {
    name: String,
}

impl Object for Tile {
    fn class_name() -> &'static str {
        "Tile"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl NodeName for Tile {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for Tile {}

fn classes() -> ClassDB {
    let mut db = ClassDB::new();
    db.register_node::<Tile>();
    db
}

const LEVEL: &str = r#"(
    format: 1,
    uid: "uid://level",
    nodes: [
        (name: "Level", class: "Tile", groups: ["levels"]),
        (name: "Gate", class: "Tile", parent: ".", groups: ["gates"]),
    ],
)"#;

const BROKEN: &str = r#"(
    format: 1,
    uid: "uid://broken",
    nodes: [
        (name: "Level", class: "Tile"),
        (name: "Ghost", class: "Unregistered", parent: "."),
    ],
)"#;

fn assert_old_scene_kept(l: &mut Loop, log: &Log, old_root: NodeId) {
    assert_eq!(l.tree.root(), Some(old_root));
    assert_eq!(names(l), ["a", "a1", "b", "root", "script"]);
    assert!(!entries(log).iter().any(|e| e.ends_with(":finalize")));
    log.lock().unwrap().clear();
    l.process(0.016);
    assert!(entries(log).contains(&"root:process".to_string()));
}

#[test]
fn an_unregistered_class_keeps_the_old_scene_and_reports_it() {
    let log = new_log();
    let broken = PackedScene::from_ron(BROKEN).unwrap();
    let (tree, old_root) = old_scene(&log, move |ctx| ctx.change_scene(broken.clone()));
    let mut l = scene_loop(tree).with_class_db(classes());
    l.init();
    l.process(0.016);
    assert_eq!(
        l.take_scene_error(),
        Some(SceneError::ClassNotRegistered("Unregistered".into()))
    );
    assert_eq!(l.take_scene_error(), None);
    assert!(!l.tree.scene_change_pending());
    assert_old_scene_kept(&mut l, &log, old_root);
}

#[test]
fn a_packed_scene_without_a_class_db_is_an_error() {
    let log = new_log();
    let level = PackedScene::from_ron(LEVEL).unwrap();
    let (tree, old_root) = old_scene(&log, move |ctx| ctx.change_scene(level));
    let mut l = scene_loop(tree);
    l.init();
    l.process(0.016);
    assert!(matches!(
        l.take_scene_error(),
        Some(SceneError::SceneChange(_))
    ));
    assert_old_scene_kept(&mut l, &log, old_root);
}

#[test]
fn a_failing_builder_leaves_nothing_behind() {
    let log = new_log();
    let l2 = log.clone();
    let (tree, old_root) = old_scene(&log, move |ctx| {
        let l2 = l2.clone();
        ctx.change_scene_with(move |tree, _| {
            let half = tree.insert(Recorder::new("half", &l2));
            let _ = half;
            Err(SceneError::Parse("level 9 is missing".into()))
        })
    });
    let mut l = scene_loop(tree);
    l.init();
    l.process(0.016);
    assert_eq!(
        l.take_scene_error(),
        Some(SceneError::Parse("level 9 is missing".into()))
    );
    // the half-built node is gone (and was finalized); the old scene was not touched
    assert_eq!(names(&l), ["a", "a1", "b", "root", "script"]);
    assert!(entries(&log).contains(&"half:finalize".to_string()));
    log.lock().unwrap().clear();
    assert_old_scene_kept(&mut l, &log, old_root);
}

#[test]
fn a_builder_must_return_a_new_floating_root() {
    let log = new_log();
    let (tree, old_root) = old_scene(&log, move |ctx| {
        ctx.change_scene_with(|tree, _| Ok(tree.root().unwrap()))
    });
    let mut l = scene_loop(tree);
    l.init();
    l.process(0.016);
    assert!(matches!(
        l.take_scene_error(),
        Some(SceneError::SceneChange(_))
    ));
    assert_old_scene_kept(&mut l, &log, old_root);
}

#[test]
fn a_failed_link_in_a_chain_keeps_the_scene_reached_so_far() {
    let log = new_log();
    let l2 = log.clone();
    let broken = PackedScene::from_ron(BROKEN).unwrap();
    let title = SceneChange::build(move |tree, _| {
        Ok(tree.insert(ChangeOnReady {
            name: "title",
            log: l2,
            next: Some(broken.into()),
        }))
    });
    let mut title = Some(title);
    let tree = NodeTree::with_root(Script::new("splash", move |ctx| {
        if let Some(t) = title.take() {
            ctx.change_scene(t);
        }
    }));
    let mut l = scene_loop(tree).with_class_db(classes());
    l.init();
    l.process(0.016);
    assert_eq!(names(&l), ["title"]);
    assert!(matches!(
        l.take_scene_error(),
        Some(SceneError::ClassNotRegistered(_))
    ));
}

// --- Building from the loop's library ---

#[test]
fn a_packed_scene_is_instanced_with_the_loops_class_db() {
    let log = new_log();
    let level = Arc::new(PackedScene::from_ron(LEVEL).unwrap());
    let (tree, _) = old_scene(&log, move |ctx| ctx.change_scene(level));
    let mut l = scene_loop(tree).with_class_db(Arc::new(classes()));
    l.init();
    l.process(0.016);
    assert_eq!(l.take_scene_error(), None);
    let root = l.tree.root().unwrap();
    assert_eq!(l.tree.get(root).unwrap().name(), "Level");
    assert!(l.tree.find(root, "Gate").is_some());
    assert_eq!(l.tree.nodes_in_group("levels"), [root]);
    assert_eq!(l.tree.len(), 2);
}

#[test]
fn a_path_loads_through_the_loops_asset_server_and_a_builder_can_set_it_up() {
    let dir = std::env::temp_dir().join(format!("ember-scene-change-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("levels")).unwrap();
    std::fs::write(dir.join("levels/level.escene"), LEVEL).unwrap();
    let assets = ember_assets::AssetServer::new(&dir).unwrap();

    let log = new_log();
    let (tree, _) = old_scene(&log, |ctx| ctx.change_scene("levels/level.escene"));
    let mut l = scene_loop(tree)
        .with_class_db(classes())
        .with_assets(assets);
    l.init();
    l.process(0.016);
    assert_eq!(l.take_scene_error(), None);
    let root = l.tree.root().unwrap();
    assert_eq!(l.tree.get(root).unwrap().name(), "Level");

    // A builder gets the same library: instance, then set up.
    l.tree.change_scene(SceneChange::build(|tree, lib| {
        let root = lib.instantiate_path("levels/level.escene", tree)?;
        tree.get_as_mut::<Tile>(root).unwrap().name = "Level 2".into();
        Ok(root)
    }));
    l.process(0.016);
    let root = l.tree.root().unwrap();
    assert_eq!(l.tree.get(root).unwrap().name(), "Level 2");

    // A missing file is reported; level 2 stays.
    l.tree.change_scene("levels/nope.escene");
    l.process(0.016);
    assert!(matches!(l.take_scene_error(), Some(SceneError::Asset(_))));
    assert_eq!(l.tree.root(), Some(root));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_path_without_an_asset_server_is_an_error() {
    let log = new_log();
    let (tree, old_root) = old_scene(&log, |ctx| ctx.change_scene("levels/level.escene"));
    let mut l = scene_loop(tree).with_class_db(classes());
    l.init();
    l.process(0.016);
    assert!(matches!(
        l.take_scene_error(),
        Some(SceneError::SceneChange(_))
    ));
    assert_old_scene_kept(&mut l, &log, old_root);
}

// --- What survives ---

struct Ping;

#[test]
fn clock_quit_groups_signals_and_loop_settings_across_a_change() {
    let log = new_log();
    let pings = new_log();
    let new = recorder_scene("level", &log);
    let mut new = Some(new);
    let (mut tree, old_root) = old_scene(&log, |_| {});
    // A second asker that waits for frame 2.
    let asker = tree.insert(Script::new("asker", move |ctx| {
        if ctx.frame() == 2
            && let Some(new) = new.take()
        {
            ctx.emit_deferred(Ping); // delivered by the pass's own flush
            ctx.quit();
            ctx.change_scene_with(new);
        }
    }));
    tree.attach(asker, old_root).unwrap();
    tree.add_to_group(old_root, "old");
    let p2 = pings.clone();
    tree.connect::<Ping>(old_root, move |_, _| p2.lock().unwrap().push("old".into()));

    let map = ActionMap::new().with_key("jump", Key::Space);
    let mut l = scene_loop(tree)
        .with_action_map(map.clone())
        .with_virtual_resolution(320.0, 180.0);
    l.init();
    assert!(l.process(0.5));
    assert!(l.process(0.25));
    // The change and the quit land in the same frame: the new scene is
    // in place, and the loop still stops.
    assert!(!l.process(0.25));
    assert_eq!(names(&l), ["level", "level_child"]);
    assert!(l.tree.quit_requested());
    // The clock keeps counting across the change.
    assert_eq!(l.tree.frame(), 3);
    assert_eq!(l.tree.time(), 1.0);
    // The pass flushes before the boundary: the old scene got its emit.
    assert_eq!(entries(&pings), ["old"]);
    // Old groups and connections went with the old scene.
    assert!(l.tree.nodes_in_group("old").is_empty());
    assert_eq!(l.tree.group_names().count(), 0);
    assert!(l.tree.signals().is_empty());
    // The loop's own settings are untouched.
    assert_eq!(l.action_map(), Some(&map));
    assert_eq!(
        l.virtual_resolution().unwrap().size,
        glam::Vec2::new(320.0, 180.0)
    );
}

#[test]
fn signals_wired_by_the_new_scene_survive_the_old_scenes_removal() {
    let log = new_log();
    let hits = new_log();
    let h2 = hits.clone();
    let l2 = log.clone();
    let (tree, _) = old_scene(&log, move |ctx| {
        let h2 = h2.clone();
        let l2 = l2.clone();
        ctx.change_scene_with(move |tree, _| {
            let root = tree.insert(Recorder::new("level", &l2));
            tree.add_to_group(root, "levels");
            tree.connect::<Ping>(root, move |_, _| h2.lock().unwrap().push("level".into()));
            Ok(root)
        })
    });
    let mut l = scene_loop(tree);
    l.init();
    l.process(0.016);
    let root = l.tree.root().unwrap();
    assert_eq!(l.tree.nodes_in_group("levels"), [root]);
    assert_eq!(l.tree.signals().len(), 1);
}

#[test]
fn finalize_drops_a_change_still_pending() {
    let log = new_log();
    let new = recorder_scene("level", &log);
    let (tree, _) = old_scene(&log, |_| {});
    let mut l = scene_loop(tree);
    l.init();
    l.tree.change_scene(SceneChange::build(new));
    l.finalize();
    assert!(!l.tree.scene_change_pending());
    assert!(l.tree.is_empty());
    assert!(!entries(&log).iter().any(|e| e.starts_with("level")));
}
