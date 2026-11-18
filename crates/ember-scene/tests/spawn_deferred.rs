mod common;
use common::*;
use ember_core::Object;
use ember_scene::prelude::*;
use std::any::Any;

/// Spawns a child and despawns a sibling from its first `process`,
/// recording what the tree can see mid-pass.
struct Spawner {
    log: Log,
    victim: Option<NodeId>,
    done: bool,
}

impl Object for Spawner {
    fn class_name() -> &'static str {
        "Spawner"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Node for Spawner {
    fn name(&self) -> &str {
        "spawner"
    }
    fn set_name(&mut self, _: String) {}
    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        if self.done {
            return;
        }
        self.done = true;
        let me = ctx.current_id();
        let child = ctx.spawn_child(me, Recorder::new("child", &self.log));
        let visible_now = ctx.tree().get(child).is_some();
        self.log
            .lock()
            .unwrap()
            .push(format!("child_visible_mid_pass:{visible_now}"));
        if let Some(v) = self.victim {
            ctx.despawn(v);
            let still = ctx.tree().contains(v);
            self.log
                .lock()
                .unwrap()
                .push(format!("victim_alive_mid_pass:{still}"));
        }
    }
}

#[test]
fn spawn_and_despawn_apply_after_the_pass() {
    let log = new_log();
    let mut tree = NodeTree::new();
    let victim = tree.insert(Recorder::new("victim", &log));
    let spawner = tree.insert(Spawner {
        log: log.clone(),
        victim: Some(victim),
        done: false,
    });
    let root = tree.insert(Recorder::new("root", &log));
    tree.attach(spawner, root).unwrap();
    tree.attach(victim, root).unwrap();
    tree.set_root(root);

    let mut h = Harness::new();
    tree.process(&mut h.servers(), 0.016);
    let e = entries(&log);

    assert!(e.contains(&"child_visible_mid_pass:false".to_string()));
    assert!(e.contains(&"victim_alive_mid_pass:true".to_string()));
    // victim still processed this pass (it comes after the spawner)
    assert!(e.contains(&"victim:process".to_string()));
    // ...then removed, and the child is live and readied at the flush.
    assert!(e.contains(&"victim:finalize".to_string()));
    assert!(e.contains(&"child:ready".to_string()));
    assert!(!tree.contains(victim));
    assert_eq!(tree.children(spawner).len(), 1);
    assert!(
        !e.contains(&"child:process".to_string()),
        "child must not process the frame it spawned in"
    );

    log.lock().unwrap().clear();
    tree.process(&mut h.servers(), 0.016);
    assert!(entries(&log).contains(&"child:process".to_string()));
}
