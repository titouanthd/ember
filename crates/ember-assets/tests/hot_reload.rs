//! Font and audio loaders, and `AssetServer::reload_changed` (ADR-038).

use ember_assets::{Asset, AssetError, AssetLoadContext, AssetServer, AudioClip, Font};
use ember_servers::{MockAudio, MockRenderer, RecordedAudioCall, RecordedCall};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime};

/// A plain text file, to test reloads without a backend.
#[derive(Debug, PartialEq)]
struct Text(String);

impl Asset for Text {
    fn extensions() -> &'static [&'static str] {
        &["txt"]
    }
    fn load(bytes: &[u8], _ctx: &mut AssetLoadContext) -> Result<Self, AssetError> {
        let s = std::str::from_utf8(bytes).map_err(|e| AssetError::Parse(e.to_string()))?;
        if s.starts_with('!') {
            return Err(AssetError::Parse("broken".into()));
        }
        Ok(Self(s.to_string()))
    }
}

fn root() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ember-assets-reload-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(dir.join("fonts")).unwrap();
    std::fs::create_dir_all(dir.join("sfx")).unwrap();
    std::fs::write(dir.join("note.txt"), "one").unwrap();
    // The mock servers accept any bytes.
    std::fs::write(dir.join("fonts/ui.ttf"), b"font").unwrap();
    std::fs::write(dir.join("sfx/jump.wav"), b"RIFF").unwrap();
    dir
}

/// Write `content` and move the file's mtime `minutes` into the future,
/// so the change is seen even on coarse-timestamp file systems.
fn edit(path: &Path, content: &str, minutes: u64) {
    std::fs::write(path, content).unwrap();
    let f = std::fs::File::options().write(true).open(path).unwrap();
    f.set_modified(SystemTime::now() + Duration::from_secs(60 * minutes))
        .unwrap();
}

#[test]
fn fonts_load_through_the_renderer() {
    let server = AssetServer::new(root()).unwrap();
    let mut r = MockRenderer::new();
    let font = server.font("fonts/ui.ttf", &mut r).unwrap();
    let id = font.get().id;
    assert!(r.calls().contains(&RecordedCall::CreateFont(id)));
    assert_eq!(
        server.load::<Font>("fonts/ui.ttf").err(),
        None,
        "cached: no renderer needed for a second handle"
    );
    let fresh = AssetServer::new(root()).unwrap();
    assert_eq!(
        fresh.load::<Font>("fonts/ui.ttf").err(),
        Some(AssetError::NeedsRenderer("fonts/ui.ttf".into()))
    );
}

#[test]
fn audio_clips_load_through_the_audio_server() {
    let server = AssetServer::new(root()).unwrap();
    let mut audio = MockAudio::new();
    let clip = server.audio_clip("sfx/jump.wav", &mut audio).unwrap();
    let id = clip.get().id;
    assert!(audio.calls().contains(&RecordedAudioCall::LoadClip(id)));
    let fresh = AssetServer::new(root()).unwrap();
    assert_eq!(
        fresh.load::<AudioClip>("sfx/jump.wav").err(),
        Some(AssetError::NeedsAudio("sfx/jump.wav".into()))
    );
}

#[test]
fn reload_changed_picks_up_edited_files_only() {
    let dir = root();
    let server = AssetServer::new(&dir).unwrap();
    let note = server.load::<Text>("note.txt").unwrap();
    assert!(
        server.reload_changed(None, None).is_empty(),
        "nothing edited"
    );

    edit(&dir.join("note.txt"), "two", 1);
    let done = server.reload_changed(None, None);
    assert_eq!(done, vec![("note.txt".to_string(), Ok(()))]);
    assert_eq!(*note.get(), Text("two".into()));
    assert!(
        server.reload_changed(None, None).is_empty(),
        "reloaded once"
    );
}

#[test]
fn dropped_assets_are_not_reloaded() {
    let dir = root();
    let server = AssetServer::new(&dir).unwrap();
    drop(server.load::<Text>("note.txt").unwrap());
    edit(&dir.join("note.txt"), "two", 1);
    assert!(server.reload_changed(None, None).is_empty());
}

#[test]
fn a_broken_file_keeps_the_old_value_and_is_not_retried() {
    let dir = root();
    let server = AssetServer::new(&dir).unwrap();
    let note = server.load::<Text>("note.txt").unwrap();
    edit(&dir.join("note.txt"), "!oops", 1);
    let done = server.reload_changed(None, None);
    assert!(matches!(done[..], [(_, Err(AssetError::Parse(_)))]));
    assert_eq!(*note.get(), Text("one".into()));
    assert!(server.reload_changed(None, None).is_empty());

    edit(&dir.join("note.txt"), "fixed", 2);
    assert_eq!(server.reload_changed(None, None).len(), 1);
    assert_eq!(*note.get(), Text("fixed".into()));
}

#[test]
fn gpu_and_audio_assets_reload_with_their_servers() {
    let dir = root();
    let server = AssetServer::new(&dir).unwrap();
    let mut r = MockRenderer::new();
    let mut audio = MockAudio::new();
    let font = server.font("fonts/ui.ttf", &mut r).unwrap();
    let clip = server.audio_clip("sfx/jump.wav", &mut audio).unwrap();
    let (old_font, old_clip) = (font.get().id, clip.get().id);

    edit(&dir.join("fonts/ui.ttf"), "font2", 1);
    edit(&dir.join("sfx/jump.wav"), "RIFF2", 1);
    let done = server.reload_changed(Some(&mut r), Some(&mut audio));
    assert_eq!(
        done,
        vec![
            ("fonts/ui.ttf".to_string(), Ok(())),
            ("sfx/jump.wav".to_string(), Ok(()))
        ]
    );
    assert_ne!(font.get().id, old_font);
    assert_ne!(clip.get().id, old_clip);
    assert!(r.calls().contains(&RecordedCall::DestroyFont(old_font)));
    assert!(
        audio
            .calls()
            .contains(&RecordedAudioCall::DestroyClip(old_clip))
    );
}

#[test]
fn reloading_without_the_needed_server_reports_it() {
    let dir = root();
    let server = AssetServer::new(&dir).unwrap();
    let mut r = MockRenderer::new();
    let _font = server.font("fonts/ui.ttf", &mut r).unwrap();
    edit(&dir.join("fonts/ui.ttf"), "font2", 1);
    assert_eq!(
        server.reload_changed(None, None),
        vec![(
            "fonts/ui.ttf".to_string(),
            Err(AssetError::NeedsRenderer("fonts/ui.ttf".into()))
        )]
    );
}
