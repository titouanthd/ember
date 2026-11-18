// games/pong/src/main.rs
use ember::prelude::*;
use pong::{config, scene};

fn main() {
    let cfg = config::load_config();
    let (w, h) = (cfg.screen_w, cfg.screen_h);
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u32)
        .unwrap_or(1);
    let tree = scene::build_tree(cfg, seed);
    let game = SceneLoop::new(
        tree,
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    run_window("Pong - Powered by Ember", w as i32, h as i32, game);
}
