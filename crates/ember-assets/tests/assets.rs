use ember_assets::{AssetError, AssetServer, RonAsset, Texture, Uid};
use ember_core::Color;
use ember_core::math::Rect;
use ember_servers::{MockRenderer, RecordedCall, RenderingServer};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

const PIXEL: &[u8] = include_bytes!("../../../assets/test_fixtures/pixel.png");

/// A fresh asset root with a copy of the PNG fixture and a RON file.
fn root() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ember-assets-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(dir.join("sprites")).unwrap();
    std::fs::write(dir.join("sprites/pixel.png"), PIXEL).unwrap();
    std::fs::write(dir.join("data.ron"), "(speed: 4.5)").unwrap();
    dir
}

#[derive(Deserialize, Debug, PartialEq)]
struct Stats {
    speed: f32,
}

#[test]
fn png_loads_into_a_texture_the_renderer_accepts() {
    let server = AssetServer::new(root()).unwrap();
    let mut r = MockRenderer::new();
    let tex = server.texture("sprites/pixel.png", &mut r).unwrap();
    let (id, w, h) = {
        let t = tex.get();
        (t.id, t.width, t.height)
    };
    assert_eq!((w, h), (2, 2));

    r.begin_frame(Rect::new(0.0, 0.0, 100.0, 100.0));
    r.draw_sprite(id, Rect::new(0.0, 0.0, 2.0, 2.0), None, Color::WHITE);
    r.end_frame();
    assert!(r.calls().contains(&RecordedCall::CreateTexture(id)));
}

#[test]
fn loads_are_cached_and_type_checked() {
    let server = AssetServer::new(root()).unwrap();
    let mut r = MockRenderer::new();
    let a = server.texture("sprites/pixel.png", &mut r).unwrap();
    let b = server.texture("sprites/pixel.png", &mut r).unwrap();
    assert_eq!(a, b);
    let creates = r
        .calls()
        .iter()
        .filter(|c| matches!(c, RecordedCall::CreateTexture(_)))
        .count();
    assert_eq!(creates, 1);

    assert!(matches!(
        server.load::<RonAsset<Stats>>("sprites/pixel.png"),
        Err(AssetError::WrongType { .. })
    ));
}

#[test]
fn textures_need_a_renderer() {
    let server = AssetServer::new(root()).unwrap();
    assert_eq!(
        server.load::<Texture>("sprites/pixel.png").err(),
        Some(AssetError::NeedsRenderer("sprites/pixel.png".into()))
    );
}

#[test]
fn bad_paths_are_rejected() {
    let server = AssetServer::new(root()).unwrap();
    for p in ["../x.ron", "/etc/x.ron", "./data.ron", ""] {
        assert!(
            matches!(
                server.load::<RonAsset<Stats>>(p),
                Err(AssetError::InvalidPath(_))
            ),
            "{p}"
        );
    }
    assert!(matches!(
        server.load::<RonAsset<Stats>>("missing.ron"),
        Err(AssetError::NotFound(_))
    ));
}

#[test]
fn ron_loads_and_reloads_in_place() {
    let dir = root();
    let server = AssetServer::new(&dir).unwrap();
    let h = server.load::<RonAsset<Stats>>("data.ron").unwrap();
    assert_eq!(h.get().0, Stats { speed: 4.5 });

    std::fs::write(dir.join("data.ron"), "(speed: 9.0)").unwrap();
    server.reload::<RonAsset<Stats>>("data.ron", None).unwrap();
    assert_eq!(h.get().0, Stats { speed: 9.0 });
}

#[test]
fn uid_registry_survives_a_restart() {
    let dir = root();
    let uid: Uid;
    {
        let server = AssetServer::new(&dir).unwrap();
        server.load::<RonAsset<Stats>>("data.ron").unwrap();
        uid = server.uid_of("data.ron").unwrap();
        assert!(uid.as_str().starts_with("uid://data_"));
    }
    assert!(dir.join("uid_registry.ron").exists());

    let server = AssetServer::new(&dir).unwrap();
    assert_eq!(server.uid_of("data.ron"), Some(uid.clone()));
    // Content edits never change the UID.
    std::fs::write(dir.join("data.ron"), "(speed: 1.0)").unwrap();
    let h = server.load_by_uid::<RonAsset<Stats>>(&uid).unwrap();
    assert_eq!(h.uid(), &uid);
    assert_eq!(h.get().0, Stats { speed: 1.0 });

    assert!(matches!(
        server.load_by_uid::<RonAsset<Stats>>(&Uid::parse("uid://nope").unwrap()),
        Err(AssetError::UnknownUid(_))
    ));
    assert!(Uid::parse("nope").is_err());
}

#[test]
fn poll_drops_unreferenced_assets() {
    let server = AssetServer::new(root()).unwrap();
    let h = server.load::<RonAsset<Stats>>("data.ron").unwrap();
    let weak = h.downgrade();
    assert_eq!(server.poll(), 0);
    drop(h);
    assert!(weak.upgrade().is_none());
    assert_eq!(server.poll(), 1);
    assert_eq!(server.cached(), 0);
}
