//! Persistence: RON on native, localStorage on web (miniquad FFI).
//!
//! DESIGN §5.5. The `Game` struct is serialized as-is — no separate
//! `Progress`/`RunState` split in V1.2 (that arrives with endgame).
//!
//! Native: `save.ron` in the working directory.
//! Web:    `localStorage["accretion:save"]` (RON string).
//!
//! The web bridge uses a miniquad plugin (`tools/localStorage_plugin.js`)
//! because `wasm-bindgen` is incompatible with miniquad's import system.
//! See `BROWSER_BUILD.md` §2.D and DESIGN §17.

use crate::systems::Game;

/// localStorage key — prefixed by the game name to avoid collisions
/// when multiple Ember games share an itch.io domain.
#[cfg(target_arch = "wasm32")]
const SAVE_KEY: &str = "accretion:save";

/// Native save file name (working directory).
#[cfg(not(target_arch = "wasm32"))]
const SAVE_FILE: &str = "save.ron";

/// Maximum size of the web save buffer. 100 KB is >10× what a full
/// Game needs (~5 KB), but bounds the FFI read safely.
#[cfg(target_arch = "wasm32")]
const MAX_WEB_SAVE_BYTES: usize = 100 * 1024;

// ─── Native ────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub fn save(game: &Game) -> bool {
    use std::path::PathBuf;
    ember_core::io::save_to_file(&PathBuf::from(SAVE_FILE), game).is_ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load() -> Option<Game> {
    use std::path::PathBuf;
    let path = PathBuf::from(SAVE_FILE);
    if !path.exists() {
        return None;
    }
    ember_core::io::load_from_file(&path).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn delete() -> bool {
    std::fs::remove_file(SAVE_FILE).is_ok()
}

// ─── Web (miniquad FFI) ───────────────────────────────────────

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn ember_storage_get(key: *const u8, out_buf: *mut u8, out_buf_len: usize) -> i32;
    fn ember_storage_set(key: *const u8, value: *const u8) -> i32;
    fn ember_storage_remove(key: *const u8) -> i32;
}

#[cfg(target_arch = "wasm32")]
pub fn save(game: &Game) -> bool {
    let Ok(text) = ron::ser::to_string(game) else {
        return false;
    };
    let Ok(key) = std::ffi::CString::new(SAVE_KEY) else {
        return false;
    };
    let Ok(value) = std::ffi::CString::new(text) else {
        return false;
    };
    unsafe {
        ember_storage_set(key.as_ptr() as *const u8, value.as_ptr() as *const u8) == 0
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load() -> Option<Game> {
    let key = std::ffi::CString::new(SAVE_KEY).ok()?;
    let mut buf = vec![0u8; MAX_WEB_SAVE_BYTES];
    let n = unsafe {
        ember_storage_get(key.as_ptr() as *const u8, buf.as_mut_ptr(), MAX_WEB_SAVE_BYTES)
    };
    if n <= 0 {
        return None;
    }
    buf.truncate(n as usize);
    let text = String::from_utf8(buf).ok()?;
    ron::from_str(&text).ok()
}

#[cfg(target_arch = "wasm32")]
pub fn delete() -> bool {
    let Ok(key) = std::ffi::CString::new(SAVE_KEY) else {
        return false;
    };
    unsafe { ember_storage_remove(key.as_ptr() as *const u8) == 0 }
}

// ─── Tests (native only) ──────────────────────────────────────

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::components::{BuildingId, Paradigm, ResourceId};
    use crate::prestige;
    use std::sync::Mutex;

    /// Serializes tests that touch the shared `save.ron` file.
    /// `cargo test` runs tests in parallel by default, and all save
    /// tests use the same path — without this, one test's write/delete
    /// clobbers another's mid-flight.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// RAII guard: removes `save.ron` on drop (test isolation).
    struct TempSave;
    impl Drop for TempSave {
        fn drop(&mut self) {
            let _ = delete();
        }
    }

    #[test]
    fn save_and_load_roundtrip_empty_game() {
        let _lock = TEST_LOCK.lock().unwrap();
        let _guard = TempSave;
        let _ = delete();

        let game = Game::new();
        assert!(save(&game));
        let loaded = load().expect("save should load back");
        assert_eq!(loaded.paradigm, Paradigm::Dust);
        assert_eq!(loaded.g_total, 0);
        assert_eq!(loaded.g_multiplier, 1.0);
    }

    #[test]
    fn save_and_load_preserves_resources_and_buildings() {
        let _lock = TEST_LOCK.lock().unwrap();
        let _guard = TempSave;
        let _ = delete();

        let mut game = Game::new();
        game.paradigm = Paradigm::Pebbles;
        *game.resources.get_mut(&ResourceId::Dust).unwrap() = 1234.5;
        for _ in 0..3 {
            game.buy_building(BuildingId::DustCollector);
        }
        assert!(save(&game));

        let loaded = load().expect("save should load back");
        assert_eq!(loaded.paradigm, Paradigm::Pebbles);
        assert!(
            (loaded.resources[&ResourceId::Dust] - game.resources[&ResourceId::Dust]).abs()
                < 1e-6
        );
        assert_eq!(loaded.building_count(BuildingId::DustCollector), 3);
    }

    #[test]
    fn save_and_load_preserves_prestige() {
        let _lock = TEST_LOCK.lock().unwrap();
        let _guard = TempSave;
        let _ = delete();

        let mut game = Game::new();
        game.paradigm = Paradigm::GasGiant;
        *game.resources.get_mut(&ResourceId::Dust).unwrap() = 500_000.0;
        prestige::prestige(&mut game);
        assert!(game.g_total >= 1);

        assert!(save(&game));
        let loaded = load().expect("save should load back");
        assert_eq!(loaded.g_total, game.g_total);
        assert!((loaded.g_multiplier - game.g_multiplier).abs() < 1e-9);
    }

    #[test]
    fn load_returns_none_when_no_file() {
        let _lock = TEST_LOCK.lock().unwrap();
        let _guard = TempSave;
        let _ = delete();
        assert!(load().is_none());
    }

    #[test]
    fn load_returns_none_when_file_is_garbage() {
        let _lock = TEST_LOCK.lock().unwrap();
        let _guard = TempSave;
        std::fs::write(SAVE_FILE, "not valid ron").unwrap();
        assert!(load().is_none());
    }

    #[test]
    fn delete_removes_save() {
        let _lock = TEST_LOCK.lock().unwrap();
        let _guard = TempSave;
        let _ = delete();

        let game = Game::new();
        assert!(save(&game));
        assert!(load().is_some());
        assert!(delete());
        assert!(load().is_none());
    }
}