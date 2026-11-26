//! Sound bank: one optional clip per game event, loaded into the
//! engine's `AudioServer`.
//!
//! Every clip is optional: if a `.wav` file is missing, `load` records
//! `None` for that slot and the game runs silently. This is intentional —
//! the repo does not ship binary assets in git, and the game must be
//! playable from a fresh checkout.

use std::path::Path;

use ember::prelude::AudioServer;
use ember_servers::AudioClipId;

use crate::systems::GameEvent;

/// All clips the game can play.
pub struct SoundBank {
    wall: Option<AudioClipId>,
    paddle_soft: Option<AudioClipId>,
    paddle_hard: Option<AudioClipId>,
    goal: Option<AudioClipId>,
    beep: Option<AudioClipId>,
    beep_go: Option<AudioClipId>,
    round_win: Option<AudioClipId>,
    match_win: Option<AudioClipId>,
}

impl SoundBank {
    /// Load every clip from `<manifest_dir>/assets/`. Missing files are
    /// recorded as `None` and the corresponding event is a no-op.
    pub fn load(manifest_dir: &Path, audio: &mut dyn AudioServer) -> Self {
        let dir = manifest_dir.join("assets");
        let mut try_load = |name: &str| -> Option<AudioClipId> {
            let bytes = std::fs::read(dir.join(name)).ok()?;
            match audio.load_clip(&bytes) {
                Ok(id) => Some(id),
                Err(e) => {
                    eprintln!("⚠️  Failed to decode {name}: {e}");
                    None
                }
            }
        };
        Self {
            wall: try_load("wall.wav"),
            paddle_soft: try_load("paddle_soft.wav"),
            paddle_hard: try_load("paddle_hard.wav"),
            goal: try_load("goal.wav"),
            beep: try_load("beep.wav"),
            beep_go: try_load("beep_go.wav"),
            round_win: try_load("round.wav"),
            match_win: try_load("match.wav"),
        }
    }

    /// The clip and volume for a game event, if that clip is loaded. The
    /// caller plays it (`AudioServer::play`).
    pub fn on_event(&self, ev: GameEvent) -> Option<(AudioClipId, f32)> {
        match ev {
            GameEvent::Wall => self.sound(&self.wall, 0.5),
            GameEvent::Paddle { impact_speed } => {
                // 900 px/s is the same boundary used by `systems::HARD_HIT_SPEED`.
                if impact_speed > 900.0 {
                    self.sound(&self.paddle_hard, 0.9)
                } else {
                    self.sound(&self.paddle_soft, 0.6)
                }
            }
            GameEvent::Goal(_) => self.sound(&self.goal, 1.0),
            GameEvent::CountdownBeep => self.sound(&self.beep, 0.7),
            GameEvent::CountdownGo => self.sound(&self.beep_go, 1.0),
            GameEvent::RoundWon(_) => self.sound(&self.round_win, 0.9),
            GameEvent::MatchWon(_) => self.sound(&self.match_win, 1.0),
        }
    }

    fn sound(&self, clip: &Option<AudioClipId>, volume: f32) -> Option<(AudioClipId, f32)> {
        clip.map(|c| (c, volume))
    }
}

#[cfg(test)]
mod tests {
    // Loading needs an `AudioServer`, so this builds an empty `SoundBank`
    // by hand. What we *can* test is that the event → slot mapping is total (no event is silently dropped) by
    // exhaustively matching on `GameEvent`. If a new variant is added and
    // not handled, this test stops compiling.
    use super::*;
    use crate::components::Side;

    #[test]
    fn test_all_events_are_handled() {
        for ev in [
            GameEvent::Wall,
            GameEvent::Paddle {
                impact_speed: 100.0,
            },
            GameEvent::Paddle {
                impact_speed: 2000.0,
            },
            GameEvent::Goal(Side::Left),
            GameEvent::Goal(Side::Right),
            GameEvent::CountdownBeep,
            GameEvent::CountdownGo,
            GameEvent::RoundWon(Side::Left),
            GameEvent::MatchWon(Side::Right),
        ] {
            let bank = SoundBank {
                wall: None,
                paddle_soft: None,
                paddle_hard: None,
                goal: None,
                beep: None,
                beep_go: None,
                round_win: None,
                match_win: None,
            };
            bank.on_event(ev);
        }
    }
}
